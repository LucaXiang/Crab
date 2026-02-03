//! Intent 模块 - 基于意图的统一分发系统
//!
//! 实现 tauri_architecture.md 中描述的 DataIntent 分发模式，
//! 提供编译时类型安全的 CRUD 操作。

pub mod dto;
pub mod query;

use serde::{Deserialize, Serialize};

// Re-exports
pub use dto::*;
pub use query::*;

/// 通用 CRUD 操作
///
/// 泛型参数：
/// - `C`: Create 数据类型
/// - `U`: Update 数据类型 (通常字段为 Option)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum CrudAction<C, U> {
    /// 创建
    Create(C),
    /// 更新 (需要 ID 和部分数据)
    Update { id: String, data: U },
    /// 删除 (只需要 ID)
    Delete { id: String },
}

/// 数据意图 - 所有管理后台 CRUD 操作的统一入口
///
/// 每个变体对应一个数据模型，包含该模型的 CRUD 操作。
/// 使用 `#[serde(tag = "model", content = "action")]` 使 JSON 结构清晰：
///
/// ```json
/// {
///   "model": "Tag",
///   "action": { "type": "Create", "data": { "name": "辣" } }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "model", content = "action")]
pub enum DataIntent {
    // ===== 产品域 =====
    /// 标签
    Tag(CrudAction<TagDto, TagUpdateDto>),
    /// 分类
    Category(CrudAction<CategoryDto, CategoryUpdateDto>),
    /// 属性
    Attribute(CrudAction<AttributeDto, AttributeUpdateDto>),

    // ===== 位置域 =====
    /// 区域
    Zone(CrudAction<ZoneDto, ZoneUpdateDto>),
    /// 桌台
    DiningTable(CrudAction<DiningTableDto, DiningTableUpdateDto>),

    // ===== 定价域 =====
    /// 价格规则
    PriceRule(CrudAction<PriceRuleDto, PriceRuleUpdateDto>),
}

/// 数据操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataResult {
    /// 是否成功
    pub success: bool,
    /// 操作消息
    pub message: String,
    /// 返回数据 (创建/更新后的完整对象)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// 受影响的 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
}

impl DataResult {
    /// 创建成功结果
    pub fn ok(message: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            id: None,
        }
    }

    /// 创建成功结果 (带数据)
    pub fn ok_with_data<T: Serialize>(message: impl Into<String>, data: T) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: serde_json::to_value(data).ok(),
            id: None,
        }
    }

    /// 创建成功结果 (带 ID)
    pub fn ok_with_id(message: impl Into<String>, id: impl Into<String>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data: None,
            id: Some(id.into()),
        }
    }

    /// 创建失败结果
    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
            id: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_intent_serialization() {
        let intent = DataIntent::Tag(CrudAction::Create(TagDto {
            name: "辣".to_string(),
            color: Some("#FF0000".to_string()),
            display_order: None,
        }));

        let json = serde_json::to_string_pretty(&intent).unwrap();
        println!("{}", json);

        // 验证反序列化
        let parsed: DataIntent = serde_json::from_str(&json).unwrap();
        match parsed {
            DataIntent::Tag(CrudAction::Create(dto)) => {
                assert_eq!(dto.name, "辣");
            }
            _ => panic!("Unexpected variant"),
        }
    }

    #[test]
    fn test_crud_action_update() {
        let intent = DataIntent::Tag(CrudAction::Update {
            id: "tag_123".to_string(),
            data: TagUpdateDto {
                name: Some("超辣".to_string()),
                color: None,
                display_order: None,
                is_active: None,
            },
        });

        let json = serde_json::to_string(&intent).unwrap();
        assert!(json.contains("\"type\":\"Update\""));
        assert!(json.contains("\"id\":\"tag_123\""));
    }

    #[test]
    fn test_crud_action_delete() {
        let intent = DataIntent::Category(CrudAction::<CategoryDto, CategoryUpdateDto>::Delete {
            id: "cat_456".to_string(),
        });

        let json = serde_json::to_string(&intent).unwrap();
        assert!(json.contains("\"type\":\"Delete\""));
    }
}
