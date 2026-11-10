//! 会员企业认证服务（member::user_company），4 条管理端链路。
//!
//! 对应原 Java `com.nextrade.service.user.impl.UserCompanyServiceImpl`
//! （317 行）+ `com.nextrade.adminapi.controller.user.UserCompanyController`
//! （80 行）中的 4 条链路：
//!
//! | 本域端点（挂载于 `/adminapi`） | 原 Java | 落点 |
//! |--------------------------------|---------|------|
//! | `GET  /user/userCompany/list` | `UserCompanyController.list` | [`UserCompanyService::list`] |
//! | `GET  /user/userCompany/detail` | `UserCompanyController.detail` | [`UserCompanyService::detail`] |
//! | `POST /user/userCompany/audit` | `UserCompanyController.audit` | [`UserCompanyService::audit`] |
//! | `POST /user/userCompany/del` | `UserCompanyController.del` | [`UserCompanyService::del`] |
//!
//! ## 迁移要点（逐条对照 Java 源）
//!
//! ### `list`
//!
//! 1. 控制器先做 `keyword` 回填：`StringUtils.isBlank(keyword)` ⇒
//!    `keyword = username`（`UserCompanyController` 第 56–58 行）。
//! 2. `buildSortOrder(page, sortField, sortOrder)` 三分支，见
//!    [`UserCompanyListQuery::order_clause`]。
//! 3. 关键字过滤（`StrUtil.isNotEmpty(keyword)` 才整段生效）：
//!    - 先 `userService.list(username ILIKE %kw% OR mobile ILIKE %kw%)` 取 `userIds`
//!    - `userIds` **非空** ⇒ `(user_id IN (...) OR contact_mobile ILIKE %kw%)`
//!    - `userIds` **为空** ⇒ 仅 `contact_mobile ILIKE %kw%`
//! 4. `status != null && status != 0` ⇒ `status = ?`。
//! 5. 每条记录补 `user` / `username` / `isCompanyAuth` / `statusText` /
//!    `typeText` / `companyData` / `addTime` / `auditTime`。
//!
//! ⚠️ 第 5 步 Java 是**逐条** `userService.lambdaQuery().eq(...).one()`
//! （N+1）；本实现用一次 `IN` 批量预取，结果等价（`user_id` 是主键）。
//!
//! ### `detail`
//!
//! `getById(id)` → null 抛「企业认证信息不存在」，否则
//! `buildDetailVO`：关联会员（`Optional.ofNullable(...).ifPresent(...)`，
//! 查不到时 `user` 保持 `null`）、状态/类型文案、时间格式化、
//! `companyData` 反序列化，并在 `licenseAddrProvince` 非空时
//! 查 `region` **按原顺序拼接**地区名到 `licenseAddrProvinceName`。
//!
//! ⚠️ Java `buildDetailVO` 第 192 行直接 `companyDataVO.getLicenseAddrProvince()`，
//! 若 `company_data` 列是 `null`（该列可空）或 JSON 里没有该键，
//! Jackson 反序列化结果是 `null` ⇒ **NPE**。本实现返回空对象兜底
//! （见 `uncertain_points`）。
//!
//! ⚠️ 地区名拼接是 `StringBuilder.append` **无分隔符**（不是用逗号连），
//! 且**按 `licenseAddrProvince` 的原顺序**遍历，查不到的 id 静默跳过。
//!
//! ### `audit`
//!
//! Java 顺序（`@Transactional(rollbackFor = Exception.class)`）：
//! 1. `getById(id)`；`Assert.notNull` ⇒ 「未知的会员企业认证信息」
//! 2. `Assert.isTrue(status == PENDING)` ⇒ 「只有待审核状态可以更新」
//! 3. `Assert.isFalse(status == FAIL && StringUtils.isEmpty(auditRemark))`
//!    ⇒ 「请填写不通过说明」
//! 4. `status == PASS` ⇒ `auditTime = now`
//! 5. `status = dto.status`；`auditRemark = dto.auditRemark ?? ""`；`updateById`
//! 6. `status == PASS` ⇒ 查会员，`Assert.notNull` ⇒ 「该会员不存在」，
//!    然后 `is_company_auth = 1`、`updateById`
//!
//! ⚠️ 第 3 步判的是 `auditDTO.getStatus() == FAIL`，即**只有** 3 才要求备注；
//! 而第 2 步要求库中记录当前必须是 1。
//!
//! ⚠️ 第 5 步之后 `status` 可能被写成 `null`（DTO 的 `status` 没有 `@NotNull`），
//! 而 `user_company.status` 是 `NOT NULL` ⇒ 数据库报错回滚。
//! 本实现提前用 `Error::Validation` 拦掉 `status` 缺失（见 `uncertain_points`）。
//!
//! ### `del`
//!
//! Java `userCompanyService.del(operateDTO.getId())` →
//! `BaseServiceImpl#del`：`id == null` 返回 `false`，否则 `removeById(id)`
//! 硬删除（表无软删列），**不校验记录是否存在**。
//! 本实现保持该语义：删不存在的 id 也返回成功（`false` 而非报错）。
//! DTO 的 `id` 缺失时抛「会员企业认证id不能为空」
//! （Java 由 `OperateDTO` 的校验注解或 `del(null)` 的 `false` 处理，
//! 两者文案不同，见 `uncertain_points`）。

use std::collections::HashMap;
use std::sync::Arc;

use nextrade_core::entities::{Region, User, UserCompany};
use nextrade_core::models::member::user_company::{
    audit_remark_value, format_time, is_auditable, list_vo_base, parse_company_data_detail,
    parse_company_data_list, should_mark_user_company_auth, should_stamp_audit_time,
    status_text_or_empty, type_text_or_empty, validate_audit, CompanyDataVO, UserCompanyAuditDTO,
    UserCompanyDetailVO, UserCompanyListQuery, UserCompanyListVO, UserVO, IS_COMPANY_AUTH_YES,
    MSG_AUDIT_REMARK_NOT_NULL, MSG_STATUS_ONLY_WAIT, MSG_UNKNOWN_USER_COMPANY,
    MSG_USER_COMPANY_ID_NOT_NULL, MSG_USER_COMPANY_NOT_FOUND, MSG_USER_NOT_EXIST,
};
use nextrade_core::types::PageResult;
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::repo::member::user_company::CompanyAuditPatch;
use nextrade_infrastructure::db::Repositories;

/// 会员企业认证域服务（挂载于 `AdminState.user_company`）。
pub struct UserCompanyService {
    repos: Arc<Repositories>,
}

impl UserCompanyService {
    /// 构造（与 order_admin / area_code / message_type 的 `new(Arc<Repositories>)` 一致）。
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    // ————————————————————— 列表 —————————————————————

    /// 会员企业认证分页（对应 Java `UserCompanyServiceImpl#list`
    /// + `UserCompanyController#list` 的 keyword 回填）。
    ///
    /// 入参 `q` 需**已经过** [`UserCompanyListQuery::apply_keyword_fallback`]
    /// （路由层调用，复刻控制器在进入 service 之前的 `setKeyword`）。
    pub async fn list(&self, q: &UserCompanyListQuery) -> Result<PageResult<UserCompanyListVO>> {
        // 1. 关键字反查会员 id（Java `userService.list(...)`），
        //    仅在 keyword 非空时执行（Java `StrUtil.isNotEmpty(keyword)` 门）。
        let user_ids: Vec<i32> = match q.effective_keyword() {
            Some(kw) => self.repos.member.user_company.find_user_ids_by_keyword(kw).await?,
            None => Vec::new(),
        };

        // 2. 分页取记录
        let page = q.page_query();
        let (total, rows) = self
            .repos.member.user_company
            .paginate(q, if q.effective_keyword().is_some() { Some(&user_ids) } else { None })
            .await?;

        // 3. 批量预取关联会员（替代 Java 的逐条 one()）
        let ids: Vec<i32> = rows.iter().map(|r| r.user_id).collect();
        let users: Vec<User> = self.repos.member.user_company.find_users_by_ids(&ids).await?;
        let mut user_map: HashMap<i32, User> = HashMap::with_capacity(users.len());
        for u in users {
            user_map.insert(u.user_id, u);
        }

        // 4. 组装 VO
        let list: Vec<UserCompanyListVO> = rows
            .iter()
            .map(|row| build_list_vo(row, user_map.get(&row.user_id)))
            .collect();

        Ok(PageResult::new(total, list, &page))
    }

    // ————————————————————— 详情 —————————————————————

    /// 会员企业认证详情（对应 Java `UserCompanyServiceImpl#detail`
    /// → `buildDetailVO`）。
    ///
    /// 记录不存在 ⇒ `企业认证信息不存在`。
    pub async fn detail(&self, id: i32) -> Result<UserCompanyDetailVO> {
        let entity = self
            .repos.member.user_company
            .find_by_id(id)
            .await?
            // Java 抛 `GlobalException("企业认证信息不存在")`；
            // Rust 侧按项目约定映射为 404（与 pc_cat_floor/018/075 一致）。
            .ok_or_else(|| Error::NotFound(MSG_USER_COMPANY_NOT_FOUND.into()))?;

        // 关联会员：Java `Optional.ofNullable(user).ifPresent(...)`
        // ⇒ 查不到时 user 保持 null（与列表的「全空对象」不同）
        let user = self.repos.member.user_company.find_user_by_id(entity.user_id).await?;

        let mut company_data = parse_company_data_detail(entity.company_data.as_deref());

        // 地区名拼接（Java 第 192–207 行）
        if let Some(province_ids) = company_data.license_addr_province.as_ref() {
            if !province_ids.is_empty() {
                let regions = self.repos.member.user_company.find_regions_by_ids(province_ids).await?;
                company_data.license_addr_province_name =
                    Some(join_region_names(province_ids, &regions));
            }
        }

        Ok(UserCompanyDetailVO {
            status_text: status_text_or_empty(entity.status),
            type_text: type_text_or_empty(entity.r#type),
            id: entity.id,
            user_id: entity.user_id,
            r#type: entity.r#type,
            contact_name: entity.contact_name,
            contact_mobile: entity.contact_mobile,
            company_name: entity.company_name,
            company_data,
            status: entity.status,
            audit_remark: entity.audit_remark,
            audit_time: format_time(entity.audit_time),
            add_time: format_time(entity.add_time),
            user: user.as_ref().map(UserVO::from_entity),
        })
    }

    // ————————————————————— 审核 —————————————————————

    /// 审核（对应 Java `UserCompanyServiceImpl#audit`）。
    ///
    /// 校验顺序与 Java 逐条一致（缺记录 → 状态门 → 备注门），
    /// 全部通过后在**单个事务**里更新认证记录（并可选地标记会员）。
    ///
    /// Java 的 `@Transactional(rollbackFor = Exception.class)` 保证
    /// 「改认证记录」与「改会员 `is_company_auth`」同生共死，本实现用
    /// 显式事务复刻。
    ///
    /// 返回是否真的改动了记录（`rows_affected > 0`）；
    /// 0 行意味着并发下已被他人抢先审核（见返回值 `uncertain_points`）。
    pub async fn audit(&self, dto: &UserCompanyAuditDTO) -> Result<bool> {
        // Java `getById(auditDTO.getId())`：id 为 null 时 getById(null) 返回 null
        let id = dto
            .id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_USER_COMPANY))?;

        let current = self
            .repos.member.user_company
            .find_by_id(id)
            .await?
            .ok_or_else(|| Error::biz_msg(MSG_UNKNOWN_USER_COMPANY))?;

        // Java `Assert.isTrue(PENDING.equals(userCompany.getStatus()), USER_COMPANY_STATUS_ONLY_WAIT)`
        // 与 aftersales 一致：`Assert.isXxx` 的 GlobalException 映射为 `Error::biz_msg`
        // （HTTP 200 + 业务错误码，与 Java `ResultWrapper.fail` 响应形状相同）。
        if !is_auditable(current.status) {
            return Err(Error::biz_msg(MSG_STATUS_ONLY_WAIT));
        }

        // Java `Assert.isFalse(FAIL.equals(status) && isEmpty(auditRemark), ...)`
        validate_audit(dto).map_err(Error::biz_msg)?;

        // Java 第 5 步的 `setStatus(dto.getStatus())`：DTO 无 `@NotNull`，
        // 为 null 时写 NULL 会撞 `status NOT NULL` 约束（库层报错、事务回滚）。
        // 本实现提前拦成业务错误，避免把 SQL 层错误暴露给前端。
        let new_status = dto
            .status
            .ok_or_else(|| Error::biz_msg("审核状态不能为空"))?;

        let mut tx = self.repos.member.user_company.begin().await?;

        // 仅「通过」时写 audit_time（Java `if (PASS.equals(status)) setAuditTime(now)`）
        let audit_time = if should_stamp_audit_time(Some(new_status)) {
            Some(now_secs())
        } else {
            None
        };

        let patch = CompanyAuditPatch {
            status: Some(new_status),
            // Java 只判 `!= null`，空白串原样写入（不 trim）
            audit_remark: Some(audit_remark_value(dto.audit_remark.as_deref())),
            audit_time,
        };

        let affected = self
            .repos.member.user_company
            .update_audit(&mut tx, id, current.status, &patch)
            .await?;

        // 仅「通过」时标记会员（Java `if (PASS.equals(status)) { ... }`）
        if should_mark_user_company_auth(Some(new_status)) {
            let user = self.repos.member.user_company.find_user_by_id(current.user_id).await?;
            let user = user.ok_or_else(|| Error::biz_msg(MSG_USER_NOT_EXIST))?;
            self.repos.member.user_company
                .mark_user_company_auth(&mut tx, user.user_id, IS_COMPANY_AUTH_YES)
                .await?;
        }

        tx.commit().await.map_err(|e| Error::Database(e.to_string()))?;

        tracing::info!(
            "会员企业认证审核: id={id}, status={}, 影响 {affected} 行",
            new_status
        );
        Ok(affected > 0)
    }

    // ————————————————————— 删除 —————————————————————

    /// 删除（对应 Java `UserCompanyServiceImpl#del` →
    /// `BaseServiceImpl#del(id)` → `removeById(id)`，硬删除）。
    ///
    /// ⚠️ Java 不校验记录是否存在，删不存在的 id 也返回成功。
    /// 本实现保持这一语义（返回删除是否命中），不额外抛 404。
    pub async fn del(&self, id: Option<i32>) -> Result<bool> {
        // Java `BaseServiceImpl#del`：`id == null` ⇒ 直接 false（不报错）。
        // 但本域控制器走的是 `userCompanyService.del(operateDTO.getId())`，
        // 而 `OperateDTO.id` 的缺失在 Java 侧由校验注解拦成 400；
        // 这里按 `USER_COMPANY_ID_NOT_NULL` 文案报 400（见 `uncertain_points`）。
        let id = id
            .filter(|v| *v > 0)
            .ok_or_else(|| Error::biz_msg(MSG_USER_COMPANY_ID_NOT_NULL))?;
        self.repos.member.user_company.delete(id).await
    }

    /// 批量操作（`POST /user/userCompany/batch`）。
    ///
    /// Java 来源：新版管理端前端
    /// `view/admin/src/api/user/userCertification.ts#batchSubmit`
    /// （body `{ type, ids }`；Java `UserCompanyController` 未含 `batch`，
    /// 语义对齐各域 `BaseServiceImpl#batch`：`type = "del"` 逐条删除，
    /// 其它 `type` 静默返回 `false`（HTTP 200，不报错））。
    ///
    /// `ids` 逐条走 [`UserCompanyService::del`] 的删除路径（硬删除）。
    ///
    /// # Errors
    /// 数据库错误 → `Error::Database`；`type = "del"` 时单个 id 删除失败同 `del`。
    pub async fn batch(&self, batch_type: &str, ids: &[i32]) -> Result<bool> {
        if batch_type != "del" {
            return Ok(false);
        }
        let mut deleted = false;
        for id in ids {
            if self.del(Some(*id)).await? {
                deleted = true;
            }
        }
        Ok(deleted)
    }
}

/// 由实体 + 关联会员构造列表项 VO
/// （对应 Java `list` 里 lambda 内的 `BeanUtils.copyProperties` + 逐项 set）。
///
/// ⚠️ Java 对**每一条**记录都 `new UserVO()` 并 `copyProperties(user, userVO)`；
/// `user` 为 null 时 `copyProperties(null, vo)` 是 no-op ⇒ 得到一个**全空对象**
/// （不是 null）。本实现同样始终给出一个 `UserVO`。
fn build_list_vo(entity: &UserCompany, user: Option<&User>) -> UserCompanyListVO {
    let mut vo = list_vo_base(entity);
    let user_vo = user.map_or_else(UserVO::default, UserVO::from_entity);
    // Java `if (user != null) { setUsername; setIsCompanyAuth; }`
    vo.username = user.map(|u| u.username.clone());
    vo.is_company_auth = user.map(|u| u.is_company_auth);
    vo.user = user_vo;
    vo.status_text = status_text_or_empty(entity.status);
    vo.type_text = type_text_or_empty(entity.r#type);
    // Java 列表走 `UserCompanyCreateDTO.CompanyDataVO`（比详情少 3 个字段）
    vo.company_data = Some(parse_company_data_list(entity.company_data.as_deref()));
    vo.add_time = format_time(entity.add_time);
    vo.audit_time = format_time(entity.audit_time);
    vo
}

/// 按 `licenseAddrProvince` 的**原顺序**拼接地区名
/// （对应 Java `buildDetailVO` 第 199–205 行的 `StringBuilder.append`）。
///
/// ⚠️ Java **不加任何分隔符**，且查不到的 `regionId` 静默跳过
/// （`if (regionMap.get(regionId) != null)`）；重复 id 会重复拼接。
/// 全部查不到时结果是**空串**（不是 null），Java 同样 set 空串。
#[must_use]
pub fn join_region_names(province_ids: &[i32], regions: &[Region]) -> String {
    let map: HashMap<i32, &str> = regions
        .iter()
        .map(|r| (r.region_id, r.region_name.as_str()))
        .collect();
    let mut out = String::new();
    for region_id in province_ids {
        if let Some(name) = map.get(region_id) {
            out.push_str(name);
        }
    }
    out
}

/// 当前秒级时间戳（Java `StringUtils.getCurrentTime()` =
/// `System.currentTimeMillis() / 1000`）。
///
/// ⚠️ 返回 `i32` 是有依据的：本域写入的 `user_company.audit_time` 列是
/// `int unsigned`（见 `schema_parser.py --table user_company`），
/// 仓储层 `CompanyAuditPatch.audit_time` 也是 `Option<i32>`。
/// 注意这与其它切片返回 `i64` 的 `now_secs()` **不同名同签名**，改动前先看调用点。
#[must_use]
pub fn now_secs() -> i32 {
    i32::try_from(chrono::Utc::now().timestamp()).unwrap_or(i32::MAX)
}

/// 供路由层复用的 keyword 回填（把控制器逻辑收在一处，
/// 避免路由文件里散落业务规则）。
#[must_use]
pub fn apply_controller_keyword_fallback(mut q: UserCompanyListQuery) -> UserCompanyListQuery {
    q.apply_keyword_fallback();
    q
}

#[cfg(test)]
mod tests {
    use super::*;

    fn region(id: i32, name: &str) -> Region {
        Region {
            region_id: id,
            region_name: name.into(),
            ..Region::default()
        }
    }

    fn company_row() -> UserCompany {
        UserCompany {
            id: 1,
            user_id: 7,
            r#type: 2,
            contact_name: "张三".into(),
            contact_mobile: "13800000000".into(),
            company_name: "甲公司".into(),
            company_data: Some(
                r#"{"corporateName":"甲公司","contactPhone":"13800000000"}"#.into(),
            ),
            status: 1,
            audit_remark: String::new(),
            audit_time: None,
            add_time: Some(1_700_000_000),
        }
    }

    // ————————————————————— 地区名拼接 —————————————————————

    /// 无分隔符拼接 + 按原顺序 + 查不到静默跳过（Java `StringBuilder.append`）。
    #[test]
    fn join_region_names_matches_java_string_builder() {
        let regions = vec![region(1, "北京市"), region(2, "上海市")];
        // 顺序按入参，不是按 region_id
        assert_eq!(join_region_names(&[2, 1], &regions), "上海市北京市");
        assert_eq!(join_region_names(&[1, 2], &regions), "北京市上海市");
        // 查不到的 id 跳过
        assert_eq!(join_region_names(&[1, 99], &regions), "北京市");
        // 全部查不到 ⇒ 空串（不是 null）
        assert_eq!(join_region_names(&[99], &regions), "");
        // 重复 id 重复拼接（Java 的 for 循环没有去重）
        assert_eq!(join_region_names(&[1, 1], &regions), "北京市北京市");
        // 空入参 ⇒ 空串
        assert_eq!(join_region_names(&[], &regions), "");
        // ⚠️ 关键：**没有**逗号分隔符
        assert!(!join_region_names(&[1, 2], &regions).contains(','));
    }

    // ————————————————————— 列表 VO 组装 —————————————————————

    /// 有会员时：`username` / `isCompanyAuth` 取自会员，`user` 为其投影。
    #[test]
    fn list_vo_with_user_fills_related_fields() {
        let row = company_row();
        let mut user = User::default();
        user.user_id = 7;
        user.username = "alice".into();
        user.mobile = "13900000000".into();
        user.is_company_auth = 0;

        let vo = build_list_vo(&row, Some(&user));
        assert_eq!(vo.username.as_deref(), Some("alice"));
        assert_eq!(vo.is_company_auth, Some(0));
        assert_eq!(vo.user.user_id, 7);
        assert_eq!(vo.user.username, "alice");
        assert_eq!(vo.user.mobile, "13900000000");
        // 文案与时间
        assert_eq!(vo.status_text, "待审核");
        assert_eq!(vo.type_text, "企业");
        assert!(!vo.add_time.is_empty());
        // audit_time 为 null ⇒ 空串（Java `handelTime(null)`）
        assert_eq!(vo.audit_time, "");
        // companyData 经白名单解析
        assert_eq!(
            vo.company_data.unwrap().corporate_name.as_deref(),
            Some("甲公司")
        );
    }

    /// 无会员时：`username` / `isCompanyAuth` 保持 `None`，
    /// 但 `user` 是**全空对象**而非 null（Java `copyProperties(null, new UserVO())`）。
    #[test]
    fn list_vo_without_user_keeps_empty_user_object() {
        let vo = build_list_vo(&company_row(), None);
        assert_eq!(vo.username, None);
        assert_eq!(vo.is_company_auth, None);
        assert_eq!(vo.user.user_id, 0);
        assert!(vo.user.username.is_empty());
        assert!(vo.user.mobile.is_empty());
        // 状态/类型文案与会员无关，仍然填上
        assert_eq!(vo.status_text, "待审核");
        assert_eq!(vo.type_text, "企业");
    }

    /// 未知 status/type 时文案回退空串（Java 会抛异常，见 uncertain_points）。
    #[test]
    fn list_vo_unknown_enum_codes_fall_back_to_empty_text() {
        let mut row = company_row();
        row.status = 9;
        row.r#type = 9;
        let vo = build_list_vo(&row, None);
        assert!(vo.status_text.is_empty());
        assert!(vo.type_text.is_empty());
    }

    // ————————————————————— 详情 companyData —————————————————————

    /// `company_data` 为 null 时详情**不能**崩（Java 会 NPE，见 uncertain_points）。
    #[test]
    fn detail_company_data_null_does_not_panic() {
        let mut row = company_row();
        row.company_data = None;
        let cd: CompanyDataVO = parse_company_data_detail(row.company_data.as_deref());
        assert!(cd.license_addr_province.is_none());
        assert!(cd.license_addr_province_name.is_none());
    }

    /// `licenseAddrProvince` 为空数组时**不**查地区、也不写 provinceName
    /// （Java `!= null && !isEmpty()` 门）。
    #[test]
    fn detail_skips_region_lookup_for_empty_province_list() {
        let raw = r#"{"licenseAddrProvince":[]}"#;
        let cd = parse_company_data_detail(Some(raw));
        let ids = cd.license_addr_province.as_ref().unwrap();
        assert!(ids.is_empty(), "空数组不该触发地区查询");
    }

    // ————————————————————— 审核门 —————————————————————

    /// 审核三门顺序与文案：缺记录 / 非待审核 / 不通过缺备注。
    #[test]
    fn audit_gate_messages_match_java() {
        // 非待审核 ⇒ `只有待审核状态可以更新`
        assert!(!is_auditable(2));
        assert_eq!(MSG_STATUS_ONLY_WAIT, "只有待审核状态可以更新");
        // 不通过缺备注
        let dto = UserCompanyAuditDTO {
            id: Some(1),
            status: Some(3),
            audit_remark: None,
        };
        assert_eq!(
            validate_audit(&dto).unwrap_err(),
            MSG_AUDIT_REMARK_NOT_NULL
        );
        // 记录不存在 / id 缺失
        assert_eq!(MSG_UNKNOWN_USER_COMPANY, "未知的会员企业认证信息");
        assert_eq!(MSG_USER_NOT_EXIST, "该会员不存在");
    }

    /// 仅「通过」才盖审核时间、才标记会员。
    #[test]
    fn audit_side_effects_gated_on_pass() {
        assert!(should_stamp_audit_time(Some(2)));
        assert!(!should_stamp_audit_time(Some(1)));
        assert!(!should_stamp_audit_time(Some(3)));
        assert!(should_mark_user_company_auth(Some(2)));
        assert!(!should_mark_user_company_auth(Some(3)));
        assert_eq!(IS_COMPANY_AUTH_YES, 1);
    }

    /// 审核落库备注：null → 空串；空白串原样保留（Java 不 trim）。
    #[test]
    fn audit_remark_persisted_verbatim() {
        assert_eq!(audit_remark_value(None), "");
        assert_eq!(audit_remark_value(Some("")), "");
        assert_eq!(audit_remark_value(Some("   ")), "   ");
        assert_eq!(audit_remark_value(Some("资料不全")), "资料不全");
    }

    // ————————————————————— 路由层 keyword 回填 —————————————————————

    /// 回填在路由层完成：keyword 空白 ⇒ 用 username。
    #[test]
    fn controller_keyword_fallback_is_applied() {
        let q = UserCompanyListQuery {
            keyword: Some("  ".into()),
            username: Some("alice".into()),
            ..Default::default()
        };
        let q = apply_controller_keyword_fallback(q);
        assert_eq!(q.effective_keyword(), Some("alice"));

        let q = UserCompanyListQuery {
            keyword: Some("139".into()),
            username: Some("alice".into()),
            ..Default::default()
        };
        let q = apply_controller_keyword_fallback(q);
        assert_eq!(q.effective_keyword(), Some("139"));
    }

    /// `now_secs` 是秒级（不是毫秒），且为正。
    #[test]
    fn now_secs_is_seconds() {
        // `now_secs()` 返回 `i32`（受 `audit_time int unsigned` 约束），
        // 比较前先提升为 `i64`；`4_000_000_000` 超出 `i32` 范围，
        // 直接写会报 "literal out of range for `i32`"。
        let s = i64::from(now_secs());
        assert!(s > 1_600_000_000, "秒级时间戳应大于 2020 年");
        assert!(s < 4_000_000_000, "秒级时间戳不应是毫秒");
    }

    /// 删除缺 id 的文案（Java `UserCompanyConstants.USER_COMPANY_ID_NOT_NULL`）。
    #[test]
    fn del_requires_id_message() {
        assert_eq!(MSG_USER_COMPANY_ID_NOT_NULL, "会员企业认证id不能为空");
        assert_eq!(MSG_USER_COMPANY_NOT_FOUND, "企业认证信息不存在");
    }
}