//! API Response wrapper
//!
//! 统一的 API 响应格式，与前端 TypeScript ApiResponse<T> 类型对齐

use serde::Serialize;

/// 统一的 API 响应格式
#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// 错误码，null 表示成功
    pub error_code: Option<String>,
    /// 消息
    pub message: String,
    /// 数据
    pub data: Option<T>,
}

impl<T: Serialize> ApiResponse<T> {
    /// 创建成功响应
    pub fn success(data: T) -> Self {
        Self {
            error_code: None,
            message: "success".to_string(),
            data: Some(data),
        }
    }

    /// 创建成功响应（带自定义消息）
    #[allow(dead_code)]
    pub fn success_with_message(data: T, message: impl Into<String>) -> Self {
        Self {
            error_code: None,
            message: message.into(),
            data: Some(data),
        }
    }

    /// 创建错误响应
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error_code: Some(code.into()),
            message: message.into(),
            data: None,
        }
    }
}

/// 从 Result 转换为 ApiResponse
impl<T: Serialize> From<Result<T, String>> for ApiResponse<T> {
    fn from(result: Result<T, String>) -> Self {
        match result {
            Ok(data) => ApiResponse::success(data),
            Err(e) => ApiResponse::error("ERROR", e),
        }
    }
}

// ============ 列表数据包装 (与前端类型对齐) ============

/// Tags 列表
#[derive(Debug, Clone, Serialize)]
pub struct TagListData {
    pub tags: Vec<shared::models::Tag>,
}

/// Categories 列表
#[derive(Debug, Clone, Serialize)]
pub struct CategoryListData {
    pub categories: Vec<shared::models::Category>,
}

/// 单个 Category
#[derive(Debug, Clone, Serialize)]
pub struct CategoryData {
    pub category: shared::models::Category,
}

/// Products 列表
#[derive(Debug, Clone, Serialize)]
pub struct ProductListData {
    pub products: Vec<shared::models::Product>,
}

/// 单个 Product
#[derive(Debug, Clone, Serialize)]
pub struct ProductData {
    pub product: shared::models::Product,
}

/// Specifications 列表
#[derive(Debug, Clone, Serialize)]
pub struct SpecListData {
    pub specs: Vec<shared::models::ProductSpecification>,
}

/// Attributes 列表
#[derive(Debug, Clone, Serialize)]
pub struct AttributeListData {
    pub templates: Vec<shared::models::Attribute>,
}

/// 单个 Attribute (template)
#[derive(Debug, Clone, Serialize)]
pub struct AttributeData {
    pub template: shared::models::Attribute,
}

/// Kitchen Printers 列表
#[derive(Debug, Clone, Serialize)]
pub struct PrinterListData {
    pub printers: Vec<shared::models::KitchenPrinter>,
}

/// 单个 Printer
#[derive(Debug, Clone, Serialize)]
pub struct PrinterData {
    pub printer: shared::models::KitchenPrinter,
}

/// 删除响应
#[derive(Debug, Clone, Serialize)]
pub struct DeleteData {
    pub deleted: bool,
}

impl DeleteData {
    pub fn success() -> Self {
        Self { deleted: true }
    }
}

/// Product Attributes 列表
#[derive(Debug, Clone, Serialize)]
pub struct ProductAttributeListData {
    pub attributes: Vec<shared::models::HasAttribute>,
}
