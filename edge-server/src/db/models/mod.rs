//! Database Models

// Serde helpers
pub mod serde_helpers;

// Auth
pub mod employee;
pub mod role;

// Product Domain
pub mod attribute;
pub mod category;
pub mod print_destination;
pub mod product;
pub mod tag;

// Location
pub mod dining_table;
pub mod zone;

// Pricing
pub mod price_rule;

// Orders
pub mod order;

// System
pub mod store_info;
pub mod label_template;
pub mod system_state;

// Image
pub mod image_ref;

// Re-exports
pub use attribute::{Attribute, AttributeBinding, AttributeBindingFull, AttributeCreate, AttributeOption, AttributeUpdate};
pub use category::{Category, CategoryCreate, CategoryUpdate};
pub use dining_table::{DiningTable, DiningTableCreate, DiningTableUpdate};
pub use employee::{Employee, EmployeeCreate, EmployeeId, EmployeeUpdate};
pub use order::{
    HasEvent, Order, OrderAddItem, OrderAddPayment, OrderCreate, OrderEvent, OrderEventType,
    OrderItem, OrderItemAttribute, OrderItemOption, OrderPayment, OrderStatus, SplitItem,
    // API response types
    OrderDetail, OrderEventDetail, OrderItemDetail, OrderItemOptionDetail, OrderPaymentDetail,
    OrderSummary,
};
pub use price_rule::{
    AdjustmentType, PriceRule, PriceRuleCreate, PriceRuleUpdate, ProductScope, RuleType,
    ZONE_SCOPE_ALL, ZONE_SCOPE_RETAIL,
};
pub use print_destination::{
    EmbeddedPrinter, PrintDestination, PrintDestinationCreate, PrintDestinationUpdate,
};
pub use product::{EmbeddedSpec, Product, ProductCreate, ProductFull, ProductUpdate};
pub use role::{Role, RoleCreate, RoleId, RoleUpdate};
pub use store_info::{StoreInfo, StoreInfoUpdate};
pub use label_template::{
    LabelField, LabelFieldAlignment, LabelFieldType, LabelTemplate, LabelTemplateCreate,
    LabelTemplateUpdate,
};
pub use system_state::{SystemState, SystemStateUpdate};
pub use tag::{Tag, TagCreate, TagUpdate};
pub use zone::{Zone, ZoneCreate, ZoneUpdate};
pub use image_ref::{ImageRef, ImageRefCreate, ImageRefEntityType};
