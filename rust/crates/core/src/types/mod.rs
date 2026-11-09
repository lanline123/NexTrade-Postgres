//! 通用类型定义：分页、查询条件、状态枚举等。
//!
//! 替代原 Java `nextrade-common` 中的 `PageQuery` / `PageResult` / 通用枚举。

use serde::{Deserialize, Serialize};

/// 分页请求，字段与原 Java `PageDTO` 保持一致。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PageQuery {
    /// 页码，从 1 开始。
    #[serde(default = "default_page")]
    pub page: u64,
    /// 每页大小。
    #[serde(default = "default_page_size", rename = "pageSize", alias = "size")]
    pub page_size: u64,
}

fn default_page() -> u64 {
    1
}

fn default_page_size() -> u64 {
    20
}

impl Default for PageQuery {
    fn default() -> Self {
        Self { page: default_page(), page_size: default_page_size() }
    }
}

impl PageQuery {
    /// SQL LIMIT/OFFSET 计算。
    pub fn limit_offset(&self) -> (i64, i64) {
        let size = self.page_size.clamp(1, 200) as i64;
        let page = self.page.max(1) as i64;
        (size, (page - 1) * size)
    }
}

/// 分页响应。
///
/// 序列化同时输出：
/// - 原 Java MyBatis-Plus `Page` 形状（前端实际读取）：`records/total/size/current/pages`
/// - Rust 历史字段：`list/totalPage/pageSize/page`（保持内部/测试兼容）
#[derive(Debug, Clone, Deserialize)]
pub struct PageResult<T> {
    pub total: u64,
    pub list: Vec<T>,
    pub total_page: u64,
    #[serde(rename = "pageSize")]
    pub page_size: u64,
    pub page: u64,
}

impl<T: Serialize> Serialize for PageResult<T> {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut st = serializer.serialize_struct("PageResult", 9)?;
        st.serialize_field("records", &self.list)?;
        st.serialize_field("total", &self.total)?;
        st.serialize_field("size", &self.page_size)?;
        st.serialize_field("current", &self.page)?;
        st.serialize_field("pages", &self.total_page)?;
        st.serialize_field("list", &self.list)?;
        st.serialize_field("totalPage", &self.total_page)?;
        st.serialize_field("pageSize", &self.page_size)?;
        st.serialize_field("page", &self.page)?;
        st.end()
    }
}

impl<T> PageResult<T> {
    pub fn new(total: u64, list: Vec<T>, query: &PageQuery) -> Self {
        let page_size = query.page_size.clamp(1, 200);
        Self {
            total,
            total_page: total.div_ceil(page_size),
            list,
            page_size,
            page: query.page.max(1),
        }
    }

    pub fn empty(query: &PageQuery) -> Self {
        Self::new(0, Vec::new(), query)
    }
}

/// 通用状态枚举。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(into = "i32", try_from = "i32")]
pub enum Status {
    /// 禁用 / 关闭
    Disabled = 0,
    /// 启用 / 开启
    Enabled = 1,
}

impl From<Status> for i32 {
    fn from(s: Status) -> Self {
        s as i32
    }
}

impl TryFrom<i32> for Status {
    type Error = String;

    fn try_from(v: i32) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Disabled),
            1 => Ok(Self::Enabled),
            other => Err(format!("非法状态值: {other}")),
        }
    }
}

/// 通用批量操作请求。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BatchOperate {
    pub ids: Vec<i64>,
    pub status: Option<Status>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limit_offset() {
        let q = PageQuery { page: 3, page_size: 10 };
        assert_eq!(q.limit_offset(), (10, 20));
    }

    #[test]
    fn page_result_total_page() {
        let q = PageQuery { page: 1, page_size: 7 };
        let r: PageResult<u8> = PageResult::new(15, vec![1, 2], &q);
        assert_eq!(r.total_page, 3);
    }
}