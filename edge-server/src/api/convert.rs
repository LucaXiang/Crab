//! 类型转换模块
//!
//! 将数据库模型 (db::models) 转换为 API 响应模型 (shared::models)

use crate::db::models as db;
use shared::models as api;

// ============ Helper ============

pub fn thing_to_string(thing: &surrealdb::sql::Thing) -> String {
    thing.to_string()
}

pub fn option_thing_to_string(thing: &Option<surrealdb::sql::Thing>) -> Option<String> {
    thing.as_ref().map(thing_to_string)
}

pub fn things_to_strings(things: &[surrealdb::sql::Thing]) -> Vec<String> {
    things.iter().map(thing_to_string).collect()
}

pub fn datetime_to_string(dt: &Option<chrono::DateTime<chrono::Utc>>) -> Option<String> {
    dt.map(|d| d.to_rfc3339())
}

// ============ Tag ============

impl From<db::Tag> for api::Tag {
    fn from(t: db::Tag) -> Self {
        Self {
            id: option_thing_to_string(&t.id),
            name: t.name,
            color: t.color,
            display_order: t.display_order,
            is_active: t.is_active,
            is_system: t.is_system,
        }
    }
}

// ============ Category ============

impl From<db::Category> for api::Category {
    fn from(c: db::Category) -> Self {
        Self {
            id: option_thing_to_string(&c.id),
            name: c.name,
            sort_order: c.sort_order,
            kitchen_print_destinations: things_to_strings(&c.kitchen_print_destinations),
            label_print_destinations: things_to_strings(&c.label_print_destinations),
            is_kitchen_print_enabled: c.is_kitchen_print_enabled,
            is_label_print_enabled: c.is_label_print_enabled,
            is_active: c.is_active,
            is_virtual: c.is_virtual,
            tag_ids: things_to_strings(&c.tag_ids),
            match_mode: c.match_mode,
        }
    }
}

// ============ Product ============

impl From<db::EmbeddedSpec> for api::EmbeddedSpec {
    fn from(s: db::EmbeddedSpec) -> Self {
        Self {
            name: s.name,
            price: s.price,
            display_order: s.display_order,
            is_default: s.is_default,
            is_active: s.is_active,
            external_id: s.external_id,
            receipt_name: s.receipt_name,
            is_root: s.is_root,
        }
    }
}

impl From<db::Product> for api::Product {
    fn from(p: db::Product) -> Self {
        Self {
            id: option_thing_to_string(&p.id),
            name: p.name,
            image: p.image,
            category: thing_to_string(&p.category),
            sort_order: p.sort_order,
            tax_rate: p.tax_rate,
            receipt_name: p.receipt_name,
            kitchen_print_name: p.kitchen_print_name,
            kitchen_print_destinations: things_to_strings(&p.kitchen_print_destinations),
            label_print_destinations: things_to_strings(&p.label_print_destinations),
            is_kitchen_print_enabled: p.is_kitchen_print_enabled,
            is_label_print_enabled: p.is_label_print_enabled,
            is_active: p.is_active,
            tags: things_to_strings(&p.tags),
            specs: p.specs.into_iter().map(Into::into).collect(),
        }
    }
}

// ============ Attribute ============

impl From<db::AttributeOption> for api::AttributeOption {
    fn from(o: db::AttributeOption) -> Self {
        Self {
            name: o.name,
            price_modifier: o.price_modifier,
            display_order: o.display_order,
            is_active: o.is_active,
            receipt_name: o.receipt_name,
            kitchen_print_name: o.kitchen_print_name,
        }
    }
}

impl From<db::Attribute> for api::Attribute {
    fn from(a: db::Attribute) -> Self {
        Self {
            id: option_thing_to_string(&a.id),
            name: a.name,
            is_multi_select: a.is_multi_select,
            max_selections: a.max_selections,
            default_option_idx: a.default_option_idx,
            display_order: a.display_order,
            is_active: a.is_active,
            show_on_receipt: a.show_on_receipt,
            receipt_name: a.receipt_name,
            show_on_kitchen_print: a.show_on_kitchen_print,
            kitchen_print_name: a.kitchen_print_name,
            options: a.options.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<db::AttributeBinding> for api::AttributeBinding {
    fn from(h: db::AttributeBinding) -> Self {
        Self {
            id: option_thing_to_string(&h.id),
            from: thing_to_string(&h.from),
            to: thing_to_string(&h.to),
            is_required: h.is_required,
            display_order: h.display_order,
            default_option_idx: h.default_option_idx,
        }
    }
}

// ============ Print Destination ============

impl From<db::EmbeddedPrinter> for api::EmbeddedPrinter {
    fn from(p: db::EmbeddedPrinter) -> Self {
        Self {
            printer_type: p.printer_type,
            printer_format: p.printer_format,
            ip: p.ip,
            port: p.port,
            driver_name: p.driver_name,
            priority: p.priority,
            is_active: p.is_active,
        }
    }
}

impl From<db::PrintDestination> for api::PrintDestination {
    fn from(p: db::PrintDestination) -> Self {
        Self {
            id: option_thing_to_string(&p.id),
            name: p.name,
            description: p.description,
            printers: p.printers.into_iter().map(Into::into).collect(),
            is_active: p.is_active,
        }
    }
}

// ============ Zone ============

impl From<db::Zone> for api::Zone {
    fn from(z: db::Zone) -> Self {
        Self {
            id: option_thing_to_string(&z.id),
            name: z.name,
            description: z.description,
            is_active: z.is_active,
        }
    }
}

// ============ Dining Table ============

impl From<db::DiningTable> for api::DiningTable {
    fn from(t: db::DiningTable) -> Self {
        Self {
            id: option_thing_to_string(&t.id),
            name: t.name,
            zone: thing_to_string(&t.zone),
            capacity: t.capacity,
            is_active: t.is_active,
        }
    }
}

// ============ Price Rule ============

impl From<db::RuleType> for api::RuleType {
    fn from(r: db::RuleType) -> Self {
        match r {
            db::RuleType::Discount => api::RuleType::Discount,
            db::RuleType::Surcharge => api::RuleType::Surcharge,
        }
    }
}

impl From<db::ProductScope> for api::ProductScope {
    fn from(s: db::ProductScope) -> Self {
        match s {
            db::ProductScope::Global => api::ProductScope::Global,
            db::ProductScope::Category => api::ProductScope::Category,
            db::ProductScope::Tag => api::ProductScope::Tag,
            db::ProductScope::Product => api::ProductScope::Product,
        }
    }
}

impl From<db::AdjustmentType> for api::AdjustmentType {
    fn from(a: db::AdjustmentType) -> Self {
        match a {
            db::AdjustmentType::Percentage => api::AdjustmentType::Percentage,
            db::AdjustmentType::FixedAmount => api::AdjustmentType::FixedAmount,
        }
    }
}

impl From<db::TimeMode> for api::TimeMode {
    fn from(t: db::TimeMode) -> Self {
        match t {
            db::TimeMode::Always => api::TimeMode::Always,
            db::TimeMode::Schedule => api::TimeMode::Schedule,
            db::TimeMode::Onetime => api::TimeMode::Onetime,
        }
    }
}

impl From<db::ScheduleConfig> for api::ScheduleConfig {
    fn from(s: db::ScheduleConfig) -> Self {
        Self {
            days_of_week: s.days_of_week,
            start_time: s.start_time,
            end_time: s.end_time,
        }
    }
}

impl From<db::PriceRule> for api::PriceRule {
    fn from(r: db::PriceRule) -> Self {
        Self {
            id: option_thing_to_string(&r.id),
            name: r.name,
            display_name: r.display_name,
            receipt_name: r.receipt_name,
            description: r.description,
            rule_type: r.rule_type.into(),
            product_scope: r.product_scope.into(),
            target: option_thing_to_string(&r.target),
            zone_scope: r.zone_scope,
            adjustment_type: r.adjustment_type.into(),
            adjustment_value: r.adjustment_value,
            priority: r.priority,
            is_stackable: r.is_stackable,
            is_exclusive: r.is_exclusive,
            time_mode: r.time_mode.into(),
            start_time: r.start_time,
            end_time: r.end_time,
            schedule_config: r.schedule_config.map(Into::into),
            valid_from: r.valid_from,
            valid_until: r.valid_until,
            active_days: r.active_days,
            active_start_time: r.active_start_time,
            active_end_time: r.active_end_time,
            is_active: r.is_active,
            created_by: option_thing_to_string(&r.created_by),
            created_at: r.created_at,
        }
    }
}

// ============ Employee ============

impl From<db::Employee> for api::Employee {
    fn from(e: db::Employee) -> Self {
        Self {
            id: option_thing_to_string(&e.id),
            username: e.username,
            display_name: e.display_name,
            role: thing_to_string(&e.role),
            is_system: e.is_system,
            is_active: e.is_active,
        }
    }
}

// ============ Order ============

impl From<db::OrderStatus> for api::OrderStatus {
    fn from(s: db::OrderStatus) -> Self {
        match s {
            db::OrderStatus::Open => api::OrderStatus::Open,
            db::OrderStatus::Paid => api::OrderStatus::Paid,
            db::OrderStatus::Void => api::OrderStatus::Void,
        }
    }
}

impl From<db::OrderItemAttribute> for api::OrderItemAttribute {
    fn from(a: db::OrderItemAttribute) -> Self {
        Self {
            attr_id: thing_to_string(&a.attr_id),
            option_idx: a.option_idx,
            name: a.name,
            price: a.price,
        }
    }
}

impl From<db::OrderItem> for api::OrderItem {
    fn from(i: db::OrderItem) -> Self {
        Self {
            spec: thing_to_string(&i.spec),
            name: i.name,
            spec_name: i.spec_name,
            price: i.price,
            quantity: i.quantity,
            attributes: i.attributes.into_iter().map(Into::into).collect(),
            discount_amount: i.discount_amount,
            surcharge_amount: i.surcharge_amount,
            note: i.note,
            is_sent: i.is_sent,
        }
    }
}

impl From<db::OrderPayment> for api::OrderPayment {
    fn from(p: db::OrderPayment) -> Self {
        Self {
            method: p.method,
            amount: p.amount,
            time: p.time,
            reference: p.reference,
        }
    }
}

impl From<db::Order> for api::Order {
    fn from(o: db::Order) -> Self {
        Self {
            id: option_thing_to_string(&o.id),
            receipt_number: o.receipt_number,
            zone_name: o.zone_name,
            table_name: o.table_name,
            status: o.status.into(),
            start_time: o.start_time,
            end_time: o.end_time,
            guest_count: o.guest_count,
            total_amount: o.total_amount,
            paid_amount: o.paid_amount,
            discount_amount: o.discount_amount,
            surcharge_amount: o.surcharge_amount,
            items: o.items.into_iter().map(Into::into).collect(),
            payments: o.payments.into_iter().map(Into::into).collect(),
            prev_hash: o.prev_hash,
            curr_hash: o.curr_hash,
            created_at: o.created_at,
        }
    }
}

impl From<db::OrderEventType> for api::OrderEventType {
    fn from(t: db::OrderEventType) -> Self {
        match t {
            db::OrderEventType::Created => api::OrderEventType::Created,
            db::OrderEventType::ItemAdded => api::OrderEventType::ItemAdded,
            db::OrderEventType::ItemRemoved => api::OrderEventType::ItemRemoved,
            db::OrderEventType::ItemUpdated => api::OrderEventType::ItemUpdated,
            db::OrderEventType::Paid => api::OrderEventType::Paid,
            db::OrderEventType::PartialPaid => api::OrderEventType::PartialPaid,
            db::OrderEventType::Void => api::OrderEventType::Void,
            db::OrderEventType::Refund => api::OrderEventType::Refund,
            db::OrderEventType::TableChanged => api::OrderEventType::TableChanged,
            db::OrderEventType::GuestCountChanged => api::OrderEventType::GuestCountChanged,
        }
    }
}

impl From<db::OrderEvent> for api::OrderEvent {
    fn from(e: db::OrderEvent) -> Self {
        Self {
            id: option_thing_to_string(&e.id),
            event_type: e.event_type.into(),
            timestamp: e.timestamp,
            data: e.data,
            prev_hash: e.prev_hash,
            curr_hash: e.curr_hash,
        }
    }
}

// ============ System State ============

impl From<db::SystemState> for api::SystemState {
    fn from(s: db::SystemState) -> Self {
        Self {
            id: option_thing_to_string(&s.id),
            genesis_hash: s.genesis_hash,
            last_order: option_thing_to_string(&s.last_order),
            last_order_hash: s.last_order_hash,
            synced_up_to: option_thing_to_string(&s.synced_up_to),
            synced_up_to_hash: s.synced_up_to_hash,
            last_sync_time: s.last_sync_time,
            order_count: s.order_count,
            created_at: s.created_at,
            updated_at: s.updated_at,
        }
    }
}
