//! 收货后自动评价服务（Tigshop v5.8.65 功能①，原版有·我方改口径）。
//!
//! 设计见 `~/workspace/auto-review/DESIGN.md`。三部分：
//! 1. **店铺设置**：商家自助读写本店设置（开关/天数/多文案），平台可按
//!    shopId 查看代改；无平台统一开关覆盖店铺。
//! 2. **收货快照** [`AutoCommentService::snapshot_on_received`]：买家手动确认
//!    收货与 scheduler 自动收货两条路径共用同一入口，收货成功后按店铺
//!    **当时**设置落快照（best-effort，失败只记日志，绝不回滚收货）。
//! 3. **到期生成** [`AutoCommentService::run_due_scan`]：scheduler loop 调用；
//!    抢占式状态机 + 逐件已有评价检查 + `order_id` 唯一行三重幂等；
//!    生成评价 `comment_rank = 5`、`is_default = 1`（系统默认评价来源标记，
//!    前台展示「系统自动评价」——我方改口径合规底线）。
//!
//! 决策逻辑全部在 `nextrade_core::models::catalog::auto_comment` 的纯函数里
//! （CI 测试套件无数据库，与仓库既有口径一致）。

use std::collections::HashSet;
use std::sync::Arc;

use nextrade_common::utils;
use nextrade_core::entities::Comment;
use nextrade_core::models::catalog::auto_comment::{
    normalize_contents, plan_generation, plan_snapshot, validate_setting, AutoCommentSetting,
    AutoCommentSettingDTO, ItemAction, AUTO_COMMENT_RANK,
};
use nextrade_core::models::catalog::comment::{
    COMMENT_STATUS_APPROVED, COMMENT_STATUS_PENDING, DEFAULT_SORT_ORDER, NOT_ANONYMOUS, NOT_SHOWED,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 每轮扫描最多处理的到期任务数。
const SCAN_BATCH: u64 = 100;

/// 评价审核配置键（与 `comment.rs` 的 `CFG_COMMENT_CHECK` 同键同口径）。
const CFG_COMMENT_CHECK: &str = "commentCheck";

#[derive(Clone)]
pub struct AutoCommentService {
    repos: Arc<Repositories>,
}

impl AutoCommentService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    fn repo(&self) -> &nextrade_infrastructure::db::repo::catalog::auto_comment::AutoCommentRepository {
        &self.repos.catalog.auto_comment
    }

    // ————————————————————— 店铺设置 —————————————————————

    /// 读店铺自动评价设置（未配置 → 默认关口径回显，不落行）。
    pub async fn get_setting(&self, shop_id: i32) -> Result<AutoCommentSetting> {
        Ok(self
            .repo()
            .find_setting(shop_id)
            .await?
            .unwrap_or_else(|| AutoCommentSetting::defaults(shop_id)))
    }

    /// 保存店铺自动评价设置（整行覆盖；校验不通过直接报错，不静默截断）。
    pub async fn update_setting(
        &self,
        shop_id: i32,
        dto: &AutoCommentSettingDTO,
    ) -> Result<AutoCommentSetting> {
        let days = dto
            .days
            .unwrap_or(nextrade_core::models::catalog::auto_comment::DEFAULT_AUTO_COMMENT_DAYS);
        let contents = normalize_contents(dto.contents.as_deref().unwrap_or(&[]));
        validate_setting(days, &contents).map_err(Error::Validation)?;
        let setting = AutoCommentSetting {
            shop_id,
            enabled: if dto.enabled == Some(1) { 1 } else { 0 },
            days,
            contents,
            update_time: utils::now_seconds() as i32,
        };
        self.repo()
            .save_setting(&setting, setting.update_time)
            .await?;
        Ok(setting)
    }

    // ————————————————————— 收货快照 —————————————————————

    /// 确认收货成功后落快照（两条收货路径共用入口）。
    ///
    /// 店铺当时未开启 → 不落行（返回 false）；开启 → 按当时天数/文案落快照行。
    /// `order_id` 唯一约束兜底：重复调用不重复落行。
    pub async fn snapshot_on_received(
        &self,
        order_id: i32,
        shop_id: i32,
        user_id: i32,
        received_time: i32,
    ) -> Result<bool> {
        let setting = self.get_setting(shop_id).await?;
        let Some(task) = plan_snapshot(&setting, order_id, shop_id, user_id, received_time) else {
            return Ok(false);
        };
        self.repo().insert_snapshot(&task).await
    }

    /// best-effort 快照：收货主流程收尾调用，失败只记 warn 不向上抛。
    pub async fn snapshot_on_received_quiet(
        &self,
        order_id: i32,
        shop_id: i32,
        user_id: i32,
        received_time: i32,
    ) {
        match self
            .snapshot_on_received(order_id, shop_id, user_id, received_time)
            .await
        {
            Ok(true) => {
                tracing::info!("自动评价：订单 {order_id} 已落收货快照（shop_id={shop_id}）");
            }
            Ok(false) => {}
            Err(e) => {
                tracing::warn!("自动评价：订单 {order_id} 落收货快照失败（不影响收货）: {e}");
            }
        }
    }

    // ————————————————————— 到期生成 —————————————————————

    /// 执行一轮到期扫描，返回本轮生成的评价条数。
    pub async fn run_due_scan(&self, now: i32) -> Result<u64> {
        let tasks = self.repo().find_due_tasks(now, SCAN_BATCH).await?;
        let mut generated = 0_u64;
        for task in &tasks {
            match self.process_task(task, now).await {
                Ok(n) => generated += n,
                Err(e) => {
                    tracing::warn!("自动评价：任务 {} 处理失败: {e}", task.id);
                }
            }
        }
        Ok(generated)
    }

    /// 处理单个到期任务：抢占 → 逐件生成 → 收尾。返回生成件数。
    async fn process_task(
        &self,
        task: &nextrade_core::models::catalog::auto_comment::AutoCommentTask,
        now: i32,
    ) -> Result<u64> {
        // 抢占失败 = 已被其它轮次处理（或状态已变），直接跳过（幂等）。
        if !self.repo().claim_task(task, now).await? {
            return Ok(0);
        }
        let comment_repo = &self.repos.catalog.comment;

        // 订单项与已有评价集合（真实评价 + 此前已生成的自动评价都在内）。
        let item_ids = comment_repo
            .find_order_item_ids(task.order_id, task.user_id)
            .await?;
        let items = comment_repo.find_order_items(&item_ids).await?;
        let commented: HashSet<i32> = {
            let mut set = HashSet::new();
            for item in &items {
                if comment_repo
                    .find_one_by_order_item(task.user_id, task.order_id, item.item_id)
                    .await?
                    .is_some()
                {
                    set.insert(item.item_id);
                }
            }
            set
        };
        let plan = plan_generation(&item_ids, &commented);
        if plan.generate_count == 0 {
            self.repo().finish_task(task.id, plan.final_status, now).await?;
            return Ok(0);
        }

        let user = self.repos.member.user.find_by_id(task.user_id).await?;
        let (username, avatar) = match &user {
            Some(u) => (u.username.clone(), u.avatar.clone()),
            None => (String::new(), String::new()),
        };
        let status = self.comment_initial_status().await?;

        let mut made = 0_u64;
        for (item, action) in items.iter().zip(plan.actions.iter()) {
            if *action != ItemAction::Generate {
                continue;
            }
            // comment.shop_id 与 evaluate 同口径：被评价商品归属店铺，兜底订单店铺。
            let shop_id = comment_repo
                .find_product_shop_id(item.product_id)
                .await
                .ok()
                .flatten()
                .unwrap_or(task.shop_id);
            let comment = Comment {
                comment_id: 0,
                user_id: task.user_id,
                username: username.clone(),
                is_anonymous: NOT_ANONYMOUS,
                avatar: avatar.clone(),
                product_id: item.product_id,
                order_id: task.order_id,
                order_item_id: item.item_id,
                comment_rank: AUTO_COMMENT_RANK,
                content: task.content.clone(),
                add_time: now,
                status,
                parent_id: 0,
                usefull: 0,
                useless: 0,
                comment_tag: Some("[]".to_string()),
                show_pics: Some("[]".to_string()),
                is_recommend: Some(0),
                is_top: 0,
                is_showed: NOT_SHOWED,
                // 来源标记：系统默认评价（前台展示「系统自动评价」）。
                is_default: 1,
                sort_order: DEFAULT_SORT_ORDER,
                shop_id,
            };
            comment_repo.insert(&comment).await?;
            made += 1;
        }

        if plan.order_fully_commented {
            comment_repo.mark_order_commented(task.order_id).await?;
        }
        self.repo().finish_task(task.id, plan.final_status, now).await?;
        Ok(made)
    }

    /// 新评价初始审核状态：与 `CommentService::comment_initial_status` 同口径
    /// （`commentCheck == "1"` 时与买家真实评价走同一初始状态）。
    async fn comment_initial_status(&self) -> Result<i32> {
        let raw = self
            .repos
            .catalog
            .comment
            .find_config_val(CFG_COMMENT_CHECK)
            .await?;
        Ok(if raw.as_deref().map(str::trim) == Some("1") {
            COMMENT_STATUS_APPROVED
        } else {
            COMMENT_STATUS_PENDING
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::catalog::auto_comment::TASK_STATUS_PENDING;

    /// 自动收货与手动收货共用同一快照入口的结构性验证：
    /// 两条路径都只调用 `snapshot_on_received`，其决策完全由
    /// `plan_snapshot` 纯函数决定——开启落行、关闭不落行，与路径无关。
    #[test]
    fn snapshot_decision_is_path_independent() {
        let received = 1_700_200_000;
        let on = AutoCommentSetting {
            shop_id: 8,
            enabled: 1,
            days: 5,
            contents: vec!["五天后好评".to_string()],
            update_time: 0,
        };
        let task = plan_snapshot(&on, 777, 8, 66, received).expect("开启应落快照");
        assert_eq!(task.status, TASK_STATUS_PENDING);
        assert_eq!(task.due_time, received + 5 * 86_400);
        assert_eq!(task.content, "五天后好评");

        let off = AutoCommentSetting {
            enabled: 0,
            ..on.clone()
        };
        assert!(plan_snapshot(&off, 777, 8, 66, received).is_none());
    }

    /// 设置保存的清洗口径：文案 trim/去空/去重后才校验与落库。
    #[test]
    fn update_setting_normalizes_contents() {
        let raw = vec![
            "  好评  ".to_string(),
            String::new(),
            "好评".to_string(),
            "物流快".to_string(),
        ];
        assert_eq!(
            normalize_contents(&raw),
            vec!["好评".to_string(), "物流快".to_string()]
        );
    }
}
