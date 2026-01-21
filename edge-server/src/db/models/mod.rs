//! Database Models

// Serde helpers
pub mod serde_helpers;
pub mod serde_thing;

// Auth
pub mod employee;
pub mod role;

// Product Domain
pub mod tag;
pub mod print_destination;
pub mod category;
pub mod product;
pub mod attribute;

// Location
pub mod zone;
pub mod dining_table;

// Pricing
pub mod price_rule;

// Orders
pub mod order;

// System
pub mod system_state;

// Re-exports
pub use employee::{Employee, EmployeeId, EmployeeCreate, EmployeeUpdate, EmployeeResponse};
pub use role::{Role, RoleCreate, RoleId, RoleUpdate};
pub use tag::{Tag, TagCreate, TagUpdate};
pub use print_destination::{PrintDestination, PrintDestinationCreate, PrintDestinationUpdate, EmbeddedPrinter};
pub use category::{Category, CategoryCreate, CategoryUpdate};
pub use product::{Product, ProductCreate, ProductUpdate, EmbeddedSpec};
pub use attribute::{Attribute, AttributeOption, AttributeCreate, AttributeUpdate, HasAttribute};
pub use zone::{Zone, ZoneCreate, ZoneUpdate};
pub use dining_table::{DiningTable, DiningTableCreate, DiningTableUpdate};
pub use price_rule::{
    PriceRule, PriceRuleCreate, PriceRuleUpdate,
    RuleType, ProductScope, AdjustmentType, TimeMode, ScheduleConfig,
};
pub use order::{
    Order, OrderCreate, OrderStatus,
    OrderItem, OrderItemAttribute, OrderPayment,
    OrderEvent, OrderEventType, HasEvent,
    OrderAddItem, OrderAddPayment,
};
pub use system_state::{SystemState, SystemStateUpdate};
