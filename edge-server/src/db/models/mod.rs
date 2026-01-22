//! Database Models

// Serde helpers
pub mod serde_helpers;
pub mod serde_thing;

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
pub mod system_state;

// Re-exports
pub use attribute::{Attribute, AttributeBinding, AttributeCreate, AttributeOption, AttributeUpdate};
pub use category::{Category, CategoryCreate, CategoryUpdate};
pub use dining_table::{DiningTable, DiningTableCreate, DiningTableUpdate};
pub use employee::{Employee, EmployeeCreate, EmployeeId, EmployeeUpdate};
pub use order::{
    HasEvent, Order, OrderAddItem, OrderAddPayment, OrderCreate, OrderEvent, OrderEventType,
    OrderItem, OrderItemAttribute, OrderPayment, OrderStatus,
};
pub use price_rule::{
    AdjustmentType, PriceRule, PriceRuleCreate, PriceRuleUpdate, ProductScope, RuleType,
    ScheduleConfig, TimeMode,
};
pub use print_destination::{
    EmbeddedPrinter, PrintDestination, PrintDestinationCreate, PrintDestinationUpdate,
};
pub use product::{EmbeddedSpec, Product, ProductCreate, ProductUpdate};
pub use role::{Role, RoleCreate, RoleId, RoleUpdate};
pub use system_state::{SystemState, SystemStateUpdate};
pub use tag::{Tag, TagCreate, TagUpdate};
pub use zone::{Zone, ZoneCreate, ZoneUpdate};
