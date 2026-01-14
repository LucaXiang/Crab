use serde::{Deserialize, Serialize};
use std::fmt;

// ==================== Domain Types ====================

/// Strong-typed Table ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TableId(String);

impl TableId {
    pub fn new(id: impl Into<String>) -> Result<Self, InvalidId> {
        let id = id.into();
        if id.is_empty() {
            return Err(InvalidId::Empty("table_id"));
        }
        Ok(Self(id))
    }

    /// Create without validation (use with caution)
    pub fn new_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Strong-typed Order ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OrderId(String);

impl OrderId {
    pub fn new(id: impl Into<String>) -> Result<Self, InvalidId> {
        let id = id.into();
        if id.is_empty() {
            return Err(InvalidId::Empty("order_id"));
        }
        Ok(Self(id))
    }

    pub fn new_unchecked(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OrderId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Strong-typed Operator ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperatorId(String);

impl OperatorId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OperatorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Strong-typed Dish ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct DishId(String);

impl DishId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DishId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum InvalidId {
    #[error("ID cannot be empty: {0}")]
    Empty(&'static str),
}

// ==================== Order Actions ====================

/// Type-safe order actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum OrderAction {
    /// Add dishes to an order
    AddDish { dishes: Vec<DishItem> },

    /// Remove a dish from an order
    RemoveDish {
        dish_id: DishId,
        quantity: u32,
    },

    /// Update dish quantity
    UpdateQuantity {
        dish_id: DishId,
        new_quantity: u32,
    },

    /// Request checkout
    Checkout { payment_method: PaymentMethod },

    /// Cancel order
    Cancel { reason: Option<String> },

    /// Split bill
    SplitBill { split_type: SplitType },

    /// Update dish notes
    UpdateNotes {
        dish_id: DishId,
        notes: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DishItem {
    pub dish_id: DishId,
    pub quantity: u32,
    pub notes: Option<String>,
    pub customizations: Vec<Customization>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Customization {
    pub option_id: String,
    pub choice: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    Cash,
    Card,
    Alipay,
    Wechat,
    MemberCard,
}

impl fmt::Display for PaymentMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cash => write!(f, "cash"),
            Self::Card => write!(f, "card"),
            Self::Alipay => write!(f, "alipay"),
            Self::Wechat => write!(f, "wechat"),
            Self::MemberCard => write!(f, "member_card"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "params")]
pub enum SplitType {
    /// Split evenly among N people
    EvenSplit { num_people: u32 },
    /// Split by dish
    ByDish,
    /// Custom split
    Custom,
}

// ==================== Order Status ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderStatus {
    /// Order created, awaiting confirmation
    Pending,

    /// Order confirmed, kitchen preparing
    Confirmed,

    /// Dishes ready for serving
    Ready,

    /// Customer dining
    Serving,

    /// Checkout requested
    CheckoutRequested,

    /// Payment completed
    Paid,

    /// Order completed and table cleared
    Completed,

    /// Order cancelled
    Cancelled,

    /// Error occurred
    Error,
}

impl fmt::Display for OrderStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Confirmed => write!(f, "confirmed"),
            Self::Ready => write!(f, "ready"),
            Self::Serving => write!(f, "serving"),
            Self::CheckoutRequested => write!(f, "checkout_requested"),
            Self::Paid => write!(f, "paid"),
            Self::Completed => write!(f, "completed"),
            Self::Cancelled => write!(f, "cancelled"),
            Self::Error => write!(f, "error"),
        }
    }
}

// ==================== Dish Status ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DishStatus {
    Pending,
    Preparing,
    Ready,
    Served,
    Cancelled,
}

impl fmt::Display for DishStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending => write!(f, "pending"),
            Self::Preparing => write!(f, "preparing"),
            Self::Ready => write!(f, "ready"),
            Self::Served => write!(f, "served"),
            Self::Cancelled => write!(f, "cancelled"),
        }
    }
}

// ==================== Notification Level ====================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Critical,
}

impl fmt::Display for NotificationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    System,
    Printer,
    Network,
    Order,
    Payment,
}

// ==================== Server Commands ====================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", content = "params")]
pub enum ServerCommand {
    /// Activate the edge server (receive certificates and metadata)
    Activate {
        tenant_id: String,
        tenant_name: String,
        edge_id: String,
        edge_name: String,
        tenant_ca_pem: String,
        edge_cert_pem: String,
        edge_key_pem: String,
    },

    /// Update server configuration
    ConfigUpdate {
        key: String,
        value: serde_json::Value,
    },

    /// Sync base data (dishes, prices, etc.)
    SyncData {
        data_type: DataSyncType,
        force: bool,
    },

    /// Remote restart
    Restart {
        delay_seconds: u32,
        reason: Option<String>,
    },

    /// Health check ping
    Ping,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSyncType {
    Dishes,
    Prices,
    Categories,
    All,
}

// ==================== Data Sync Payload ====================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "sync_type", content = "data")]
pub enum DataSyncPayload {
    /// Dish price update
    DishPrice {
        dish_id: DishId,
        old_price: u64,
        new_price: u64,
    },

    /// Dish sold out
    DishSoldOut {
        dish_id: DishId,
        available: bool,
    },

    /// New dish added
    DishAdded {
        dish_id: DishId,
        name: String,
        price: u64,
        category: String,
    },

    /// Dish removed
    DishRemoved { dish_id: DishId },

    /// Category update
    CategoryUpdate {
        category_id: String,
        name: String,
        sort_order: u32,
    },

    /// Batch sync
    BatchSync { items: Vec<DataSyncPayload> },
}

// ==================== Payloads ====================

/// Payload for OrderIntent (Client -> Server)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderIntentPayload {
    pub action: OrderAction,
    pub table_id: TableId,
    pub order_id: Option<OrderId>,
    pub operator: Option<OperatorId>,
}

/// Payload for OrderSync (Server -> Client)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderSyncPayload {
    pub action: OrderAction,
    pub table_id: TableId,
    pub order_id: Option<OrderId>,
    pub status: OrderStatus,
    pub source: OperatorId,
    pub data: Option<OrderData>,
}

/// Type-safe order data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderData {
    /// Total amount in cents (分)
    pub total_amount: u64,
    pub items: Vec<OrderItem>,
    /// ISO 8601 timestamp
    pub created_at: String,
    /// ISO 8601 timestamp
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderItem {
    pub dish_id: DishId,
    pub dish_name: String,
    pub quantity: u32,
    /// Unit price in cents (分)
    pub unit_price: u64,
    /// Subtotal in cents (分)
    pub subtotal: u64,
    pub status: DishStatus,
}

/// Payload for Notification (Server -> Client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    pub title: String,
    pub message: String,
    pub level: NotificationLevel,
    pub category: NotificationCategory,
    pub data: Option<serde_json::Value>,
}

/// Payload for ServerCommand (Upstream -> Server)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerCommandPayload {
    pub command: ServerCommand,
}

// ==================== Builders ====================

impl OrderIntentPayload {
    pub fn builder() -> OrderIntentBuilder {
        OrderIntentBuilder::default()
    }

    /// Quick constructor for adding dishes
    pub fn add_dish(
        table_id: TableId,
        dishes: Vec<DishItem>,
        operator: Option<OperatorId>,
    ) -> Self {
        Self {
            action: OrderAction::AddDish { dishes },
            table_id,
            order_id: None,
            operator,
        }
    }

    /// Quick constructor for checkout
    pub fn checkout(
        table_id: TableId,
        order_id: OrderId,
        payment_method: PaymentMethod,
        operator: Option<OperatorId>,
    ) -> Self {
        Self {
            action: OrderAction::Checkout { payment_method },
            table_id,
            order_id: Some(order_id),
            operator,
        }
    }
}

#[derive(Default)]
pub struct OrderIntentBuilder {
    action: Option<OrderAction>,
    table_id: Option<TableId>,
    order_id: Option<OrderId>,
    operator: Option<OperatorId>,
}

impl OrderIntentBuilder {
    pub fn action(mut self, action: OrderAction) -> Self {
        self.action = Some(action);
        self
    }

    pub fn table_id(mut self, table_id: TableId) -> Self {
        self.table_id = Some(table_id);
        self
    }

    pub fn order_id(mut self, order_id: OrderId) -> Self {
        self.order_id = Some(order_id);
        self
    }

    pub fn operator(mut self, operator: OperatorId) -> Self {
        self.operator = Some(operator);
        self
    }

    pub fn build(self) -> Result<OrderIntentPayload, BuilderError> {
        Ok(OrderIntentPayload {
            action: self.action.ok_or(BuilderError::MissingField("action"))?,
            table_id: self.table_id.ok_or(BuilderError::MissingField("table_id"))?,
            order_id: self.order_id,
            operator: self.operator,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub enum BuilderError {
    #[error("Missing required field: {0}")]
    MissingField(&'static str),
}

// ==================== Convenience Constructors ====================

impl DishItem {
    pub fn simple(dish_id: impl Into<String>, quantity: u32) -> Self {
        Self {
            dish_id: DishId::new(dish_id),
            quantity,
            notes: None,
            customizations: vec![],
        }
    }

    pub fn with_notes(dish_id: impl Into<String>, quantity: u32, notes: String) -> Self {
        Self {
            dish_id: DishId::new(dish_id),
            quantity,
            notes: Some(notes),
            customizations: vec![],
        }
    }
}

impl NotificationPayload {
    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: NotificationLevel::Info,
            category: NotificationCategory::System,
            data: None,
        }
    }

    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: NotificationLevel::Warning,
            category: NotificationCategory::System,
            data: None,
        }
    }

    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: NotificationLevel::Error,
            category: NotificationCategory::System,
            data: None,
        }
    }
}
