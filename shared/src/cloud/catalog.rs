//! Cloud ↔ Edge catalog RPC protocol types
//!
//! CatalogOp: 强类型 catalog 操作，通过 CloudMessage::Rpc 传输
//! 方向: Cloud → Edge (编辑权威在 Cloud)
//! 幂等性: 每个 RPC 带唯一 id，接收方缓存已执行结果

use serde::{Deserialize, Serialize};

use crate::models::{
    attribute::{AttributeCreate, AttributeUpdate},
    category::{Category, CategoryCreate, CategoryUpdate},
    dining_table::{DiningTable, DiningTableCreate, DiningTableUpdate},
    employee::{Employee, EmployeeCreate, EmployeeUpdate},
    price_rule::{PriceRule, PriceRuleCreate, PriceRuleUpdate},
    product::{ProductCreate, ProductFull, ProductUpdate},
    tag::{Tag, TagCreate, TagUpdate},
    zone::{Zone, ZoneCreate, ZoneUpdate},
};

/// Catalog 操作枚举 — Cloud → Edge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum CatalogOp {
    // ── Product ──
    CreateProduct {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        data: ProductCreate,
    },
    UpdateProduct {
        id: i64,
        data: ProductUpdate,
    },
    DeleteProduct {
        id: i64,
    },

    // ── Category ──
    CreateCategory {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        data: CategoryCreate,
    },
    UpdateCategory {
        id: i64,
        data: CategoryUpdate,
    },
    DeleteCategory {
        id: i64,
    },

    // ── Attribute ──
    CreateAttribute {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        data: AttributeCreate,
    },
    UpdateAttribute {
        id: i64,
        data: AttributeUpdate,
    },
    DeleteAttribute {
        id: i64,
    },

    // ── Attribute Binding ──
    BindAttribute {
        owner: BindingOwner,
        attribute_id: i64,
        is_required: bool,
        #[serde(default)]
        display_order: i32,
        default_option_ids: Option<Vec<i32>>,
    },
    UnbindAttribute {
        binding_id: i64,
    },

    // ── Tag ──
    CreateTag {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        data: TagCreate,
    },
    UpdateTag {
        id: i64,
        data: TagUpdate,
    },
    DeleteTag {
        id: i64,
    },

    // ── Price Rule ──
    CreatePriceRule {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        data: PriceRuleCreate,
    },
    UpdatePriceRule {
        id: i64,
        data: PriceRuleUpdate,
    },
    DeletePriceRule {
        id: i64,
    },

    // ── Employee ──
    CreateEmployee {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        data: EmployeeCreate,
    },
    UpdateEmployee {
        id: i64,
        data: EmployeeUpdate,
    },
    DeleteEmployee {
        id: i64,
    },

    // ── Zone ──
    CreateZone {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        data: ZoneCreate,
    },
    UpdateZone {
        id: i64,
        data: ZoneUpdate,
    },
    DeleteZone {
        id: i64,
    },

    // ── DiningTable ──
    CreateTable {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        data: DiningTableCreate,
    },
    UpdateTable {
        id: i64,
        data: DiningTableUpdate,
    },
    DeleteTable {
        id: i64,
    },

    // ── Batch (首次供给 / 全量推送) ──
    FullSync {
        snapshot: CatalogSnapshot,
    },

    // ── Image ──
    /// 确保 edge 本地有指定图片文件 (fire-and-forget)
    EnsureImage {
        /// S3 presigned GET URL
        presigned_url: String,
        /// SHA256 content hash
        hash: String,
    },
}

/// Attribute binding 的多态所有者
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "id")]
pub enum BindingOwner {
    Product(i64),
    Category(i64),
}

impl BindingOwner {
    pub fn owner_type(&self) -> &'static str {
        match self {
            Self::Product(_) => "product",
            Self::Category(_) => "category",
        }
    }

    pub fn owner_id(&self) -> i64 {
        match self {
            Self::Product(id) | Self::Category(id) => *id,
        }
    }
}

/// 全量快照 — 用于首次激活时推送默认 catalog
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogSnapshot {
    pub tags: Vec<TagCreate>,
    pub categories: Vec<CategorySnapshotItem>,
    pub products: Vec<ProductSnapshotItem>,
    pub attributes: Vec<AttributeSnapshotItem>,
}

/// 快照中的分类项（含关联数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategorySnapshotItem {
    pub data: CategoryCreate,
    /// 创建后需要绑定的 attribute IDs
    #[serde(default)]
    pub attribute_bindings: Vec<SnapshotBinding>,
}

/// 快照中的商品项（含关联数据）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductSnapshotItem {
    /// 所属分类在快照 categories 数组中的索引
    pub category_index: usize,
    pub data: ProductCreate,
    /// 创建后需要绑定的 attribute IDs
    #[serde(default)]
    pub attribute_bindings: Vec<SnapshotBinding>,
}

/// 快照中的属性项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttributeSnapshotItem {
    pub data: AttributeCreate,
}

/// 快照中的 binding 引用（用索引引用快照内的 attribute）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotBinding {
    /// attribute 在快照 attributes 数组中的索引
    pub attribute_index: usize,
    pub is_required: bool,
    #[serde(default)]
    pub display_order: i32,
    pub default_option_ids: Option<Vec<i32>>,
}

/// Catalog 操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogOpResult {
    pub success: bool,
    /// 创建操作返回 edge 本地 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_id: Option<i64>,
    /// 创建/更新操作返回完整数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<CatalogOpData>,
    /// 失败时的错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 操作返回的完整数据（供 console 显示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum CatalogOpData {
    Product(ProductFull),
    Category(Category),
    Tag(Tag),
    PriceRule(PriceRule),
    Employee(Employee),
    Zone(Zone),
    Table(DiningTable),
}

impl CatalogOpResult {
    pub fn ok() -> Self {
        Self {
            success: true,
            created_id: None,
            data: None,
            error: None,
        }
    }

    pub fn created(id: i64) -> Self {
        Self {
            success: true,
            created_id: Some(id),
            data: None,
            error: None,
        }
    }

    pub fn with_data(mut self, data: CatalogOpData) -> Self {
        self.data = Some(data);
        self
    }

    pub fn err(msg: impl Into<String>) -> Self {
        Self {
            success: false,
            created_id: None,
            data: None,
            error: Some(msg.into()),
        }
    }
}

/// FullSync 的批量结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FullSyncResult {
    pub tags_created: usize,
    pub categories_created: usize,
    pub products_created: usize,
    pub attributes_created: usize,
    pub bindings_created: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}
