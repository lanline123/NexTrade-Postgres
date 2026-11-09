//! 微信菜单服务（content::wechat_menu）。
//!
//! 对应原 Java `com.nextrade.service.setting.wechat.impl.WechatMenuServiceImpl`。
//!
//! 构造签名固定：`pub fn new(repos: Arc<Repositories>) -> Self`。
//!
//! ## 已迁移（2）
//! - `get`：读 `wechat_menu` 单例并解析 `menu_json`；空配置/解析失败 → 空树
//! - `saveAndPublish`：全量本地校验后落库 `wechat_menu`；微信配置可用时推送
//!   `menu/create`，不可用时保存为草稿并返回提示
//!
//! ## 与 Java 的差异
//! 见 [`crate::models::content::wechat_menu`] 模块头。

use std::sync::Arc;

use nextrade_core::models::content::wechat_menu::{
    parse_menu_tree, validate_menu, SaveAndPublishVO, WxMenuSaveDTO, WxMenuVO,
};
use nextrade_core::{Error, Result};
use nextrade_infrastructure::db::Repositories;

/// 微信菜单服务。
pub struct WechatMenuService {
    repos: Arc<Repositories>,
}

impl WechatMenuService {
    #[must_use]
    pub fn new(repos: Arc<Repositories>) -> Self {
        Self { repos }
    }

    /// 本片仓储（NESTED 访问：`self.repos.content.wechat_menu`）。
    #[must_use]
    pub fn repo(
        &self,
    ) -> &nextrade_infrastructure::db::repo::content::wechat_menu::WechatMenuRepository {
        &self.repos.content.wechat_menu
    }

    /// 获取菜单详情（Java `get`）。
    ///
    /// # Errors
    /// 数据库失败 → `Error::Database`。
    pub async fn get(&self) -> Result<WxMenuVO> {
        let Some(entity) = self.repo().find_first().await? else {
            return Ok(WxMenuVO {
                id: None,
                tree: Default::default(),
                published_at: None,
                updated_at: None,
            });
        };
        Ok(WxMenuVO {
            id: Some(entity.id),
            tree: parse_menu_tree(Some(&entity.menu_json)),
            published_at: entity.published_at,
            updated_at: Some(entity.updated_at),
        })
    }

    /// 保存并发布（Java `saveAndPublish`）：本地全量校验 → 微信 API 接入与落库。
    pub async fn save_and_publish(&self, dto: &WxMenuSaveDTO) -> Result<SaveAndPublishVO> {
        validate_menu(dto).map_err(Error::biz_msg)?;
        let tree_json = serde_json::to_string(&dto.tree.clone().unwrap_or_default())
            .map_err(|e| Error::Internal(e.to_string()))?;
        let now_str = chrono::Utc::now().naive_utc().format("%Y-%m-%d %H:%M:%S").to_string();

        let buttons: Vec<serde_json::Value> = dto
            .tree
            .as_ref()
            .map(|t| t.buttons.iter().map(convert_node_to_wechat).collect())
            .unwrap_or_default();

        match crate::content::wechat_client::WechatClient::from_repos(&self.repos).await {
            Ok(client) => match client.access_token().await {
                Ok(token) => {
                    let body = serde_json::json!({ "button": buttons });
                    let url = format!("https://api.weixin.qq.com/cgi-bin/menu/create?access_token={token}");
                    match client.post_json(&url, &body).await {
                        Ok(text) => match crate::content::wechat_client::parse_wechat_response(&text) {
                            Ok(_) => {
                                self.repo().save_menu(&tree_json, Some(&now_str)).await?;
                                Ok(SaveAndPublishVO {
                                    published: true,
                                    err_msg: None,
                                    published_at: Some(now_str),
                                })
                            }
                            Err(e) => {
                                self.repo().save_menu(&tree_json, None).await?;
                                Ok(SaveAndPublishVO {
                                    published: false,
                                    err_msg: Some(e.to_string()),
                                    published_at: None,
                                })
                            }
                        },
                        Err(e) => {
                            self.repo().save_menu(&tree_json, None).await?;
                            Ok(SaveAndPublishVO {
                                published: false,
                                err_msg: Some(e.to_string()),
                                published_at: None,
                            })
                        }
                    }
                }
                Err(e) => {
                    self.repo().save_menu(&tree_json, None).await?;
                    Ok(SaveAndPublishVO {
                        published: false,
                        err_msg: Some(format!("获取微信 token 失败: {e}")),
                        published_at: None,
                    })
                }
            },
            Err(_) => {
                self.repo().save_menu(&tree_json, None).await?;
                Ok(SaveAndPublishVO {
                    published: false,
                    err_msg: Some("微信配置未配置，菜单已保存为草稿".into()),
                    published_at: None,
                })
            }
        }
    }
}

fn convert_node_to_wechat(node: &nextrade_core::models::content::wechat_menu::WxMenuNodeDTO) -> serde_json::Value {
    let name = node.name.as_deref().unwrap_or("").trim();
    if !node.children.is_empty() {
        let subs: Vec<serde_json::Value> = node.children.iter().map(convert_node_to_wechat).collect();
        serde_json::json!({
            "name": name,
            "sub_button": subs
        })
    } else if let Some(rule) = &node.rule {
        let r_type = rule.r#type.as_deref().unwrap_or("click").to_ascii_lowercase();
        match r_type.as_str() {
            "view" => serde_json::json!({
                "type": "view",
                "name": name,
                "url": rule.url.as_deref().unwrap_or("")
            }),
            "miniprogram" => serde_json::json!({
                "type": "miniprogram",
                "name": name,
                "url": rule.miniprogram_url.as_deref().unwrap_or(""),
                "appid": rule.appid.as_deref().unwrap_or(""),
                "pagepath": rule.pagepath.as_deref().unwrap_or("")
            }),
            _ => serde_json::json!({
                "type": "click",
                "name": name,
                "key": rule.event_key.as_deref().unwrap_or("")
            }),
        }
    } else {
        serde_json::json!({
            "type": "click",
            "name": name,
            "key": ""
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nextrade_core::models::content::wechat_menu::{click_rule, leaf_node, WxMenuTree};

    #[test]
    fn save_and_publish_validates_menu_structure() {
        // 空菜单 → 校验错误（不是未迁移）
        let err = validate_menu(&WxMenuSaveDTO::default()).unwrap_err();
        assert_eq!(err, "菜单结构不能为空");
        // 合法菜单 → 本地校验通过
        let dto = WxMenuSaveDTO {
            tree: Some(WxMenuTree {
                buttons: vec![leaf_node("资讯", click_rule("MENU_1", "欢迎"))],
            }),
        };
        assert!(validate_menu(&dto).is_ok());
    }


    #[test]
    fn get_vo_defaults_when_missing() {
        // 不触库：仅验证空 VO 结构与序列化
        let vo = WxMenuVO {
            id: None,
            tree: WxMenuTree::default(),
            published_at: None,
            updated_at: None,
        };
        let json = serde_json::to_value(&vo).unwrap();
        assert!(json["id"].is_null());
        assert!(json["tree"]["buttons"].as_array().unwrap().is_empty());
        assert!(json["publishedAt"].is_null());
    }

    #[test]
    fn save_vo_shape() {
        let vo = SaveAndPublishVO {
            published: false,
            err_msg: Some("微信配置未配置，菜单已保存为草稿".into()),
            published_at: None,
        };
        let json = serde_json::to_value(&vo).unwrap();
        assert_eq!(json["published"], false);
        assert_eq!(json["errMsg"], "微信配置未配置，菜单已保存为草稿");
        assert!(json["publishedAt"].is_null());
    }
}
