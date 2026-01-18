//! 查询请求类型
//!
//! 提供统一的列表查询和详情查询接口。

use serde::{Deserialize, Serialize};

/// 查询请求 - 用于列表查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRequest {
    /// 模型名称: "tag", "category", "product", etc.
    pub model: String,
    /// 过滤条件 (JSON 对象)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<serde_json::Value>,
    /// 排序字段 (如: "name", "created_at_desc")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<String>,
    /// 页码 (从 1 开始)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    /// 每页数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    /// 是否包含已删除 (软删除) 的记录
    #[serde(default)]
    pub include_inactive: bool,
}

impl QueryRequest {
    /// 创建简单查询 (获取所有活跃记录)
    pub fn all(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            filter: None,
            sort: None,
            page: None,
            limit: None,
            include_inactive: false,
        }
    }

    /// 创建带过滤的查询
    pub fn with_filter(model: impl Into<String>, filter: serde_json::Value) -> Self {
        Self {
            model: model.into(),
            filter: Some(filter),
            sort: None,
            page: None,
            limit: None,
            include_inactive: false,
        }
    }

    /// 添加分页
    pub fn paginate(mut self, page: u32, limit: u32) -> Self {
        self.page = Some(page);
        self.limit = Some(limit);
        self
    }

    /// 添加排序
    pub fn order_by(mut self, sort: impl Into<String>) -> Self {
        self.sort = Some(sort.into());
        self
    }
}

/// 分页响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// 数据列表
    pub data: Vec<T>,
    /// 总记录数
    pub total: u64,
    /// 当前页码
    pub page: u32,
    /// 每页数量
    pub limit: u32,
    /// 总页数
    pub total_pages: u32,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, total: u64, page: u32, limit: u32) -> Self {
        let total_pages = if limit > 0 {
            ((total as f64) / (limit as f64)).ceil() as u32
        } else {
            1
        };

        Self {
            data,
            total,
            page,
            limit,
            total_pages,
        }
    }

    /// 创建单页响应 (不分页时使用)
    pub fn single_page(data: Vec<T>) -> Self {
        let total = data.len() as u64;
        Self {
            data,
            total,
            page: 1,
            limit: total as u32,
            total_pages: 1,
        }
    }
}

/// 查询结果 - 用于单条查询
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult<T> {
    /// 是否成功
    pub success: bool,
    /// 数据 (成功时)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// 错误消息 (失败时)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl<T> QueryResult<T> {
    pub fn ok(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_request_builder() {
        let req = QueryRequest::all("tag")
            .order_by("display_order")
            .paginate(1, 20);

        assert_eq!(req.model, "tag");
        assert_eq!(req.page, Some(1));
        assert_eq!(req.limit, Some(20));
        assert_eq!(req.sort, Some("display_order".to_string()));
    }

    #[test]
    fn test_paginated_response() {
        let items = vec!["a", "b", "c"];
        let resp = PaginatedResponse::new(items, 100, 2, 10);

        assert_eq!(resp.total, 100);
        assert_eq!(resp.page, 2);
        assert_eq!(resp.total_pages, 10);
    }
}
