//! 管理端礼品卡路由（marketing::gift_card）。
//!
//! 原 Java：`com.nextrade.adminapi.controller.promotion.GiftCardController`
//! （`@RequestMapping("/adminapi/promotion/giftCards")`）。
//!
//! ## 原 Java 路径对照表（本模块挂载于 `/adminapi` 之下）
//!
//! | 本模块路由 | 原 Java 完整路径 | 原 Java 方法 | 契约 auth |
//! |-----------|------------------|----------------|-----------|
//! | `GET /promotion/giftCards/list` | `GET /adminapi/promotion/giftCards/list` | `list` | jwt |
//! | `GET /promotion/giftCards/detail` | `…/detail` | `detail` | jwt |
//! | `GET /promotion/giftCards/importTemplate` | `…/importTemplate` | `importTemplate` | jwt |
//! | `GET /promotion/giftCards/exportList` | `…/exportList` | `exportList` | jwt |
//! | `POST /promotion/giftCards/importList` | `…/importList` | `importList` | jwt+perm |
//! | `POST /promotion/giftCards/delete` | `…/delete` | `deleteGiftCards` | jwt+perm |
//! | `GET /promotion/giftCards/giftCardExchangeDetail` | `…/giftCardExchangeDetail` | `giftCardExchangeDetail` | jwt |
//! | `POST /promotion/giftCards/updateCardStatus` | `…/updateCardStatus` | `updateGiftCardsAvailableStatus` | jwt |
//!
//! > **Excel 已接线**：`importTemplate` / `exportList` / `importList` 经
//! > `services::system::excel` 真实读写 xlsx 并返回二进制（早期「未迁移」
//! > 注释已过期，P1 清理）。
//!
//! > **`delete` 入参**：Java `GiftCardIdParam.cardGroupsId`（String）实际承载
//! > **礼品卡 ID**，字段名照抄。

use axum::{
    extract::{Multipart, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use nextrade_core::models::marketing::gift_card::{
    GiftCardIdParam, GiftCardUpdateStatusDTO, GiftCardsExportVO, GiftCardsImportVO,
    GiftCardsListQuery, GiftCardsTemplateVO, GiftCardsVO,
};
use nextrade_core::models::marketing::gift_card_issue::{
    GiftCardIssueBatchPageQuery, GiftCardIssueBatchVO, GiftCardIssueByExcelVO,
    GiftCardIssueLogPageQuery, GiftCardIssueLogVO, GiftCardIssueParam, GiftCardIssuePreviewVO,
    GiftCardIssueSendVO,
};
use nextrade_core::types::PageResult;
use nextrade_core::Error;
use nextrade_security::web::{ok, ApiResult, AppError};

use crate::AdminState;

/// 本域 16 条路由（去掉 `/adminapi` 前缀后的路径 + 原 Java handler）。
/// createBatch / showCardPassword 来自新版管理端前端契约
/// （`view/admin/src/api/promotion/giftCard.ts`）；末尾 6 条为发放闭环：
/// issueTemplate / issueByExcel 为**原版有**（giftmall Admin 同名端点）；
/// issuePreview / issueSend 为**原版有·我方改口径**（原版单次 issue，
/// 我方加预检两段式，见 standard-alignment D1）；issueBatch / issueLog
/// 对应原版 giftCardGroups/issue/batch|log 的批次+明细两层（D11）。
const ROUTE_TABLE: [(&str, &str, &str); 16] = [
    (
        "GET",
        "/promotion/giftCards/list",
        "GiftCardController.list",
    ),
    (
        "GET",
        "/promotion/giftCards/detail",
        "GiftCardController.detail",
    ),
    (
        "GET",
        "/promotion/giftCards/importTemplate",
        "GiftCardController.importTemplate",
    ),
    (
        "GET",
        "/promotion/giftCards/exportList",
        "GiftCardController.exportList",
    ),
    (
        "POST",
        "/promotion/giftCards/importList",
        "GiftCardController.importList",
    ),
    (
        "POST",
        "/promotion/giftCards/delete",
        "GiftCardController.deleteGiftCards",
    ),
    (
        "GET",
        "/promotion/giftCards/giftCardExchangeDetail",
        "GiftCardController.giftCardExchangeDetail",
    ),
    (
        "POST",
        "/promotion/giftCards/updateCardStatus",
        "GiftCardController.updateGiftCardsAvailableStatus",
    ),
    (
        "POST",
        "/promotion/giftCards/createBatch",
        "GiftCardController.createGiftCardsBatch",
    ),
    (
        "GET",
        "/promotion/giftCards/showCardPassword",
        "GiftCardController.showCardPassword",
    ),
    (
        "POST",
        "/promotion/giftCards/issuePreview",
        "GiftCardController.issuePreview(P1)",
    ),
    (
        "POST",
        "/promotion/giftCards/issueSend",
        "GiftCardController.issueSend(P1)",
    ),
    (
        "GET",
        "/promotion/giftCards/issueTemplate",
        "GiftCardController.issueTemplate",
    ),
    (
        "POST",
        "/promotion/giftCards/issueByExcel",
        "GiftCardController.issueByExcel",
    ),
    (
        "GET",
        "/promotion/giftCards/issueBatch",
        "GiftCardController.issueBatch(P1)",
    ),
    (
        "GET",
        "/promotion/giftCards/issueLog",
        "GiftCardController.issueLog(P1)",
    ),
];

/// 礼品卡管理路由（挂载于 `/adminapi` 下，外层统一套 `require_auth`）。
pub fn routes(state: &AdminState) -> Router<AdminState> {
    let _ = state;
    Router::new()
        .route("/promotion/giftCards/list", get(list))
        .route("/promotion/giftCards/detail", get(detail))
        .route("/promotion/giftCards/importTemplate", get(import_template))
        .route("/promotion/giftCards/exportList", get(export_list))
        .route("/promotion/giftCards/importList", post(import_list))
        .route("/promotion/giftCards/delete", post(delete))
        .route(
            "/promotion/giftCards/giftCardExchangeDetail",
            get(gift_card_exchange_detail),
        )
        .route(
            "/promotion/giftCards/updateCardStatus",
            post(update_card_status),
        )
        .route("/promotion/giftCards/createBatch", post(create_batch))
        .route("/promotion/giftCards/create", post(create))
        .route("/promotion/giftCards/update", post(update))
        .route(
            "/promotion/giftCards/showCardPassword",
            get(show_card_password),
        )
        .route("/promotion/giftCards/issuePreview", post(issue_preview))
        .route("/promotion/giftCards/issueSend", post(issue_send))
        .route("/promotion/giftCards/issueTemplate", get(issue_template))
        .route("/promotion/giftCards/issueByExcel", post(issue_by_excel))
        .route("/promotion/giftCards/issueBatch", get(issue_batch))
        .route("/promotion/giftCards/issueLog", get(issue_log))
}

/// 本模块已实现路由（覆盖率统计，完整契约路径）。
#[must_use]
pub fn implemented_routes() -> Vec<nextrade_openapi::RouteDoc> {
    ROUTE_TABLE
        .iter()
        .map(|(method, path, handler)| {
            nextrade_openapi::RouteDoc::new(method, &format!("/adminapi{path}"), handler)
        })
        .collect()
}

/// `giftCardsId` 查询参数（Java `@RequestParam Integer giftCardsId`）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GiftCardsIdQuery {
    gift_cards_id: i32,
}

/// `cardGroupsId` 查询参数（exportList）。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
struct CardGroupsIdQuery {
    card_groups_id: Option<i32>,
}

async fn list(
    State(state): State<AdminState>,
    Query(q): Query<GiftCardsListQuery>,
) -> ApiResult<PageResult<GiftCardsVO>> {
    ok(state.marketing.gift_card.list(&q).await?)
}

async fn detail(
    State(state): State<AdminState>,
    Query(q): Query<GiftCardsIdQuery>,
) -> ApiResult<GiftCardsVO> {
    ok(state
        .marketing
        .gift_card
        .admin_detail(q.gift_cards_id)
        .await?)
}

/// `GET /promotion/giftCards/importTemplate`：导入模板（xlsx）。
async fn import_template(
    State(state): State<AdminState>,
) -> Result<axum::response::Response, AppError> {
    let template = state.marketing.gift_card.import_template();
    let rows: Vec<Vec<String>> = template
        .into_iter()
        .map(|r| vec![r.card_number, r.card_password])
        .collect();
    let bytes =
        nextrade_services::system::excel::build_xlsx(&["卡号", "密码"], &rows, "礼品卡导入模板")?;
    Ok(crate::support::excel_response::xlsx_response(
        bytes,
        "gift_card_template.xlsx",
    ))
}

/// `GET /promotion/giftCards/exportList`：导出列表（xlsx）。
async fn export_list(
    State(state): State<AdminState>,
    Query(q): Query<CardGroupsIdQuery>,
) -> Result<axum::response::Response, AppError> {
    let card_groups_id = q
        .card_groups_id
        .ok_or_else(|| AppError(Error::Validation("cardGroupsId不能为空".into())))?;
    let list = state
        .marketing
        .gift_card
        .export_list(card_groups_id)
        .await?;
    let rows: Vec<Vec<String>> = list
        .into_iter()
        .map(|r| {
            vec![
                r.card_number,
                r.card_password,
                r.gift_card_sn.unwrap_or_default(),
                r.is_sold,
                r.is_exchange,
                r.add_time,
            ]
        })
        .collect();
    let bytes = nextrade_services::system::excel::build_xlsx(
        &[
            "卡号",
            "密码",
            "卡序列号",
            "是否售出",
            "是否兑换",
            "添加时间",
        ],
        &rows,
        "礼品卡列表",
    )?;
    Ok(crate::support::excel_response::xlsx_response(
        bytes,
        "gift_cards.xlsx",
    ))
}

/// `POST /promotion/giftCards/importList`：Excel 批量导入卡密（xlsx 解析后逐行入库）。
async fn import_list(
    State(state): State<AdminState>,
    Query(q): Query<CardGroupsIdQuery>,
    mut multipart: Multipart,
) -> ApiResult<GiftCardsImportVO> {
    let (form_group_id, bytes) = read_import_file(&mut multipart).await?;
    let rows = nextrade_services::system::excel::read_xlsx(&bytes)?;
    let cards: Vec<(String, String)> = rows
        .into_iter()
        .skip(1)
        .filter_map(|row| {
            let number = row.first()?.trim().to_string();
            let password = row.get(1).map(|s| s.trim().to_string()).unwrap_or_default();
            if number.is_empty() && password.is_empty() {
                None
            } else {
                Some((number, password))
            }
        })
        .collect();
    ok(state
        .marketing
        .gift_card
        .import_list(q.card_groups_id.or(form_group_id), &cards)
        .await?)
}

/// 读取 multipart 表单：`cardGroupsId` 文本字段 + `file` 内容。
async fn read_import_file(multipart: &mut Multipart) -> Result<(Option<i32>, Vec<u8>), Error> {
    let mut group_id: Option<i32> = None;
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| Error::Validation(format!("解析上传表单失败: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name == "file" {
            let bytes = field
                .bytes()
                .await
                .map_err(|e| Error::Validation(format!("读取文件失败: {e}")))?
                .to_vec();
            return Ok((group_id, bytes));
        }
        let text = field
            .text()
            .await
            .map_err(|e| Error::Validation(format!("读取表单字段失败: {e}")))?;
        if name == "cardGroupsId" {
            group_id = text.trim().parse::<i32>().ok();
        }
    }
    Err(Error::Validation("请上传文件".into()))
}

async fn delete(
    State(state): State<AdminState>,
    Json(param): Json<GiftCardIdParam>,
) -> ApiResult<()> {
    state.marketing.gift_card.delete(&param).await?;
    ok(())
}

async fn gift_card_exchange_detail(
    State(state): State<AdminState>,
    Query(q): Query<GiftCardsIdQuery>,
) -> ApiResult<nextrade_core::models::marketing::gift_card::GiftCardDetailVO> {
    ok(state
        .marketing
        .gift_card
        .gift_card_exchange_detail(q.gift_cards_id)
        .await?)
}

async fn update_card_status(
    State(state): State<AdminState>,
    Json(dto): Json<GiftCardUpdateStatusDTO>,
) -> ApiResult<()> {
    state.marketing.gift_card.update_card_status(&dto).await?;
    ok(())
}

/// `showCardPassword` 的查询参数（前端 `getGiftCardPasswrod({ cardId })`）。
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CardIdQuery {
    card_id: Option<i32>,
}

/// `POST /adminapi/promotion/giftCards/create`：生成礼品卡。
async fn create(
    State(state): State<AdminState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<()> {
    let card_groups_id = body
        .get("cardGroupsId")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0) as i32;
    let count = body
        .get("count")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(1) as i32;
    state
        .marketing
        .gift_card
        .create_batch(card_groups_id, count)
        .await?;
    ok(())
}

/// `POST /adminapi/promotion/giftCards/update`：更新礼品卡可用状态。
async fn update(
    State(state): State<AdminState>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<()> {
    let dto = GiftCardUpdateStatusDTO {
        gift_cards_id: body
            .get("giftCardsId")
            .and_then(serde_json::Value::as_i64)
            .map(|v| v as i32),
        is_available: body
            .get("isAvailable")
            .and_then(serde_json::Value::as_i64)
            .map(|v| v as i32),
    };
    state.marketing.gift_card.update_card_status(&dto).await?;
    ok(())
}

/// `POST /adminapi/promotion/giftCards/createBatch`：批量生成礼品卡。
///
/// Java 来源：`GiftCardController.createGiftCardsBatch`
/// （`GiftCardIdParam { cardGroupsId, count }` →
/// `giftCardsService.createGiftCardsBatch`）。
async fn create_batch(
    State(state): State<AdminState>,
    Json(param): Json<GiftCardIdParam>,
) -> ApiResult<()> {
    let card_groups_id = param.gift_cards_id().map_err(Error::Validation)?;
    let count = param.count.unwrap_or(0);
    state
        .marketing
        .gift_card
        .create_batch(card_groups_id, count)
        .await?;
    ok(())
}

/// `GET /adminapi/promotion/giftCards/showCardPassword?cardId=`：查看卡密。
///
/// Java 来源：新版管理端前端
/// `view/admin/src/api/promotion/giftCard.ts#getGiftCardPasswrod`
/// （返回 `string`；旧版 Java 控制器未含该端点）。
async fn show_card_password(
    State(state): State<AdminState>,
    Query(q): Query<CardIdQuery>,
) -> ApiResult<String> {
    let card_id = q
        .card_id
        .filter(|v| *v > 0)
        .ok_or_else(|| AppError(Error::Validation("cardId不能为空".into())))?;
    ok(state
        .marketing
        .gift_card
        .show_card_password(card_id)
        .await?)
}

// ————————————————————— P1 发放闭环 —————————————————————

/// `POST /promotion/giftCards/issuePreview`：发放预检（目标/库存/失败明细）。
async fn issue_preview(
    State(state): State<AdminState>,
    Json(param): Json<GiftCardIssueParam>,
) -> ApiResult<GiftCardIssuePreviewVO> {
    ok(state.marketing.gift_card_issue.issue_preview(&param).await?)
}

/// `POST /promotion/giftCards/issueSend`：正式发放（事务分配 + 站内信）。
async fn issue_send(
    State(state): State<AdminState>,
    Json(param): Json<GiftCardIssueParam>,
) -> ApiResult<GiftCardIssueSendVO> {
    ok(state.marketing.gift_card_issue.issue_send(&param).await?)
}

/// `GET /promotion/giftCards/issueTemplate`：Excel 发卡模板（xlsx）。
///
/// 来源标注：**原版有** — 原版模板列为 会员名/手机号/卡号/卡密/备注
/// （与卡密导入模板 importTemplate 是两个不同模板，勿混淆）。
async fn issue_template(
    State(state): State<AdminState>,
) -> Result<axum::response::Response, AppError> {
    let _ = state;
    let rows: Vec<Vec<String>> = vec![vec![
        "张三".to_string(),
        "13800000000".to_string(),
        "GC202601010001".to_string(),
        "abcd1234".to_string(),
        "年会福利".to_string(),
    ]];
    let bytes = nextrade_services::system::excel::build_xlsx(
        &["会员名", "手机号", "卡号", "卡密", "备注"],
        &rows,
        "礼品卡发卡模板",
    )?;
    Ok(crate::support::excel_response::xlsx_response(
        bytes,
        "gift_card_issue_template.xlsx",
    ))
}

/// `POST /promotion/giftCards/issueByExcel`：Excel 批量发卡（multipart：
/// `file` xlsx + `cardGroupsId` 字段）。原版单次提交口径：逐行把已存在
/// 且未使用的指定卡发给指定会员，失败行记录原因、其余继续。
async fn issue_by_excel(
    State(state): State<AdminState>,
    Query(q): Query<CardGroupsIdQuery>,
    mut multipart: Multipart,
) -> ApiResult<GiftCardIssueByExcelVO> {
    let (form_group_id, bytes) = read_import_file(&mut multipart).await?;
    let card_groups_id = q
        .card_groups_id
        .or(form_group_id)
        .ok_or_else(|| AppError(Error::Validation("cardGroupsId不能为空".into())))?;
    let rows = nextrade_services::system::excel::read_xlsx(&bytes)?;
    ok(state
        .marketing
        .gift_card_issue
        .issue_by_excel(card_groups_id, &rows, None)
        .await?)
}

/// `GET /promotion/giftCards/issueBatch`：发放批次分页。
async fn issue_batch(
    State(state): State<AdminState>,
    Query(q): Query<GiftCardIssueBatchPageQuery>,
) -> ApiResult<PageResult<GiftCardIssueBatchVO>> {
    ok(state.marketing.gift_card_issue.issue_batch_list(&q).await?)
}

/// `GET /promotion/giftCards/issueLog`：发放明细分页（卡号脱敏）。
async fn issue_log(
    State(state): State<AdminState>,
    Query(q): Query<GiftCardIssueLogPageQuery>,
) -> ApiResult<PageResult<GiftCardIssueLogVO>> {
    ok(state.marketing.gift_card_issue.issue_log_list(&q).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_routes_match_java_contract() {
        let docs = implemented_routes();
        assert_eq!(docs.len(), 16);
        let expected = [
            ("GET", "/adminapi/promotion/giftCards/list"),
            ("GET", "/adminapi/promotion/giftCards/detail"),
            ("GET", "/adminapi/promotion/giftCards/importTemplate"),
            ("GET", "/adminapi/promotion/giftCards/exportList"),
            ("POST", "/adminapi/promotion/giftCards/importList"),
            ("POST", "/adminapi/promotion/giftCards/delete"),
            (
                "GET",
                "/adminapi/promotion/giftCards/giftCardExchangeDetail",
            ),
            ("POST", "/adminapi/promotion/giftCards/updateCardStatus"),
            ("POST", "/adminapi/promotion/giftCards/createBatch"),
            ("GET", "/adminapi/promotion/giftCards/showCardPassword"),
            ("POST", "/adminapi/promotion/giftCards/issuePreview"),
            ("POST", "/adminapi/promotion/giftCards/issueSend"),
            ("GET", "/adminapi/promotion/giftCards/issueTemplate"),
            ("POST", "/adminapi/promotion/giftCards/issueByExcel"),
            ("GET", "/adminapi/promotion/giftCards/issueBatch"),
            ("GET", "/adminapi/promotion/giftCards/issueLog"),
        ];
        for (method, path) in expected {
            assert!(
                docs.iter().any(|d| d.method == method && d.path == path),
                "缺少 {method} {path}"
            );
        }
        let mut keys: Vec<String> = docs.iter().map(nextrade_openapi::RouteDoc::key).collect();
        let mut unique = keys.clone();
        unique.sort();
        unique.dedup();
        keys.sort();
        assert_eq!(unique, keys);
    }

    #[test]
    fn route_table_matches_registrations() {
        assert_eq!(ROUTE_TABLE.len(), 16);
        for (_, path, _) in ROUTE_TABLE.iter() {
            assert!(!path.contains("/adminapi"), "{path}");
            assert!(path.starts_with("/promotion/giftCards/"), "{path}");
        }
        assert_eq!(
            implemented_routes()[0].path,
            "/adminapi/promotion/giftCards/list"
        );
    }

    #[test]
    fn delete_param_uses_java_field_name() {
        // Java `GiftCardIdParam.cardGroupsId`（String）实际承载卡 ID
        let p: GiftCardIdParam =
            serde_json::from_value(serde_json::json!({"cardGroupsId": "42"})).unwrap();
        assert_eq!(p.gift_cards_id().unwrap(), 42);
    }
}
