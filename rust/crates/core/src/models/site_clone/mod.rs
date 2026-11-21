//! 站点搬家（site_clone）模型域：搬家密钥、拉取审计、导入快照、导入任务、
//! 去重映射与跨站序列化包。设计：`~/workspace/site-clone-design/REPORT.md`
//! 第四/五/六节 + `DOMAINS.md` 附册四。
//!
//! 来源标注：**我方新增**（原版 tigshop 无整站/跨站复制功能；原版仅有
//! 装修单页分享，其导入落库由 P2 补齐，不在本模块范围）。

pub mod clone;
