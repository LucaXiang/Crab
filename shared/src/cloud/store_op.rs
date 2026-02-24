//! Cloud ↔ Edge store RPC protocol types
//!
//! StoreOp: 强类型 store 操作，通过 CloudMessage::Rpc 传输
//! 方向: Cloud → Edge (编辑权威在 Cloud)
//! 幂等性: 每个 RPC 带唯一 id，接收方缓存已执行结果

use serde::{Deserialize, Serialize};

use crate::models::{
    attribute::{Attribute, AttributeCreate, AttributeUpdate},
    category::{Category, CategoryCreate, CategoryUpdate},
    dining_table::{DiningTable, DiningTableCreate, DiningTableUpdate},
    employee::{Employee, EmployeeCreate, EmployeeUpdate},
    label_template::{LabelTemplate, LabelTemplateCreate, LabelTemplateUpdate},
    price_rule::{PriceRule, PriceRuleCreate, PriceRuleUpdate},
    product::{ProductCreate, ProductFull, ProductUpdate},
    store_info::{StoreInfo, StoreInfoUpdate},
    tag::{Tag, TagCreate, TagUpdate},
    zone::{Zone, ZoneCreate, ZoneUpdate},
};

/// Store 操作枚举 — Cloud → Edge
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
pub enum StoreOp {
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
    BatchUpdateProductSortOrder {
        items: Vec<SortOrderItem>,
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

    // ── LabelTemplate ──
    CreateLabelTemplate {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        id: Option<i64>,
        data: LabelTemplateCreate,
    },
    UpdateLabelTemplate {
        id: i64,
        data: LabelTemplateUpdate,
    },
    DeleteLabelTemplate {
        id: i64,
    },

    // ── StoreInfo (singleton) ──
    UpdateStoreInfo {
        data: StoreInfoUpdate,
    },

    // ── Batch (首次供给 / 全量推送) ──
    FullSync {
        snapshot: StoreSnapshot,
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

/// 全量快照 — 用于首次激活时推送默认 store 数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreSnapshot {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortOrderItem {
    pub id: i64,
    pub sort_order: i32,
}

/// Store 操作结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreOpResult {
    pub success: bool,
    /// 创建操作返回 edge 本地 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_id: Option<i64>,
    /// 创建/更新操作返回完整数据
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<StoreOpData>,
    /// 失败时的错误信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 操作返回的完整数据（供 console 显示）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind")]
pub enum StoreOpData {
    Product(ProductFull),
    Category(Category),
    Tag(Tag),
    Attribute(Attribute),
    PriceRule(PriceRule),
    Employee(Employee),
    Zone(Zone),
    Table(DiningTable),
    LabelTemplate(LabelTemplate),
    StoreInfo(StoreInfo),
}

impl StoreOpResult {
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

    pub fn with_data(mut self, data: StoreOpData) -> Self {
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
