//! Employee + Zone + DiningTable + PriceRule + LabelTemplate operations (via repository)

use shared::cloud::SyncResource;
use shared::cloud::store_op::{StoreOpData, StoreOpResult};
use shared::message::SyncChangeType;

use crate::core::state::ServerState;

// ── Employee ──

pub async fn create_employee(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: shared::models::EmployeeCreate,
) -> StoreOpResult {
    use crate::db::repository::employee;

    match employee::create(&state.pool, assigned_id, data).await {
        Ok(emp) => {
            state
                .broadcast_sync(
                    SyncResource::Employee,
                    SyncChangeType::Created,
                    &emp.id.to_string(),
                    Some(&emp),
                    true,
                )
                .await;
            StoreOpResult::created(emp.id).with_data(StoreOpData::Employee(emp))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn update_employee(
    state: &ServerState,
    id: i64,
    data: shared::models::EmployeeUpdate,
) -> StoreOpResult {
    use crate::db::repository::employee;

    match employee::update(&state.pool, id, data).await {
        Ok(emp) => {
            state
                .broadcast_sync(
                    SyncResource::Employee,
                    SyncChangeType::Updated,
                    &emp.id.to_string(),
                    Some(&emp),
                    true,
                )
                .await;
            StoreOpResult::ok().with_data(StoreOpData::Employee(emp))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn delete_employee(state: &ServerState, id: i64) -> StoreOpResult {
    use crate::db::repository::employee;

    match employee::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>(
                    SyncResource::Employee,
                    SyncChangeType::Deleted,
                    &id.to_string(),
                    None,
                    true,
                )
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

// ── Zone ──

pub async fn create_zone(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: shared::models::ZoneCreate,
) -> StoreOpResult {
    use crate::db::repository::zone;

    match zone::create(&state.pool, assigned_id, data).await {
        Ok(z) => {
            state
                .broadcast_sync(
                    SyncResource::Zone,
                    SyncChangeType::Created,
                    &z.id.to_string(),
                    Some(&z),
                    true,
                )
                .await;
            StoreOpResult::created(z.id).with_data(StoreOpData::Zone(z))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn update_zone(
    state: &ServerState,
    id: i64,
    data: shared::models::ZoneUpdate,
) -> StoreOpResult {
    use crate::db::repository::zone;

    match zone::update(&state.pool, id, data).await {
        Ok(z) => {
            state
                .broadcast_sync(
                    SyncResource::Zone,
                    SyncChangeType::Updated,
                    &z.id.to_string(),
                    Some(&z),
                    true,
                )
                .await;
            StoreOpResult::ok().with_data(StoreOpData::Zone(z))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn delete_zone(state: &ServerState, id: i64) -> StoreOpResult {
    use crate::db::repository::zone;

    match zone::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>(
                    SyncResource::Zone,
                    SyncChangeType::Deleted,
                    &id.to_string(),
                    None,
                    true,
                )
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

// ── DiningTable ──

pub async fn create_table(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: shared::models::DiningTableCreate,
) -> StoreOpResult {
    use crate::db::repository::dining_table;

    match dining_table::create(&state.pool, assigned_id, data).await {
        Ok(t) => {
            state
                .broadcast_sync(
                    SyncResource::DiningTable,
                    SyncChangeType::Created,
                    &t.id.to_string(),
                    Some(&t),
                    true,
                )
                .await;
            StoreOpResult::created(t.id).with_data(StoreOpData::Table(t))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn update_table(
    state: &ServerState,
    id: i64,
    data: shared::models::DiningTableUpdate,
) -> StoreOpResult {
    use crate::db::repository::dining_table;

    match dining_table::update(&state.pool, id, data).await {
        Ok(t) => {
            state
                .broadcast_sync(
                    SyncResource::DiningTable,
                    SyncChangeType::Updated,
                    &t.id.to_string(),
                    Some(&t),
                    true,
                )
                .await;
            StoreOpResult::ok().with_data(StoreOpData::Table(t))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn delete_table(state: &ServerState, id: i64) -> StoreOpResult {
    use crate::db::repository::dining_table;

    match dining_table::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>(
                    SyncResource::DiningTable,
                    SyncChangeType::Deleted,
                    &id.to_string(),
                    None,
                    true,
                )
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

// ── PriceRule ──

pub async fn create_price_rule(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: shared::models::PriceRuleCreate,
) -> StoreOpResult {
    use crate::db::repository::price_rule;

    match price_rule::create(&state.pool, assigned_id, data).await {
        Ok(rule) => {
            state
                .broadcast_sync(
                    SyncResource::PriceRule,
                    SyncChangeType::Created,
                    &rule.id.to_string(),
                    Some(&rule),
                    true,
                )
                .await;
            StoreOpResult::created(rule.id).with_data(StoreOpData::PriceRule(rule))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn update_price_rule(
    state: &ServerState,
    id: i64,
    data: shared::models::PriceRuleUpdate,
) -> StoreOpResult {
    use crate::db::repository::price_rule;

    match price_rule::update(&state.pool, id, data).await {
        Ok(rule) => {
            state
                .broadcast_sync(
                    SyncResource::PriceRule,
                    SyncChangeType::Updated,
                    &rule.id.to_string(),
                    Some(&rule),
                    true,
                )
                .await;
            StoreOpResult::ok().with_data(StoreOpData::PriceRule(rule))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn delete_price_rule(state: &ServerState, id: i64) -> StoreOpResult {
    use crate::db::repository::price_rule;

    match price_rule::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>(
                    SyncResource::PriceRule,
                    SyncChangeType::Deleted,
                    &id.to_string(),
                    None,
                    true,
                )
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

// ── LabelTemplate ──

pub async fn create_label_template(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: shared::models::label_template::LabelTemplateCreate,
) -> StoreOpResult {
    use crate::db::repository::label_template;

    match label_template::create(&state.pool, assigned_id, data).await {
        Ok(tpl) => {
            state
                .broadcast_sync(
                    SyncResource::LabelTemplate,
                    SyncChangeType::Created,
                    &tpl.id.to_string(),
                    Some(&tpl),
                    true,
                )
                .await;
            StoreOpResult::created(tpl.id).with_data(StoreOpData::LabelTemplate(tpl))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn update_label_template(
    state: &ServerState,
    id: i64,
    data: shared::models::label_template::LabelTemplateUpdate,
) -> StoreOpResult {
    use crate::db::repository::label_template;

    match label_template::update(&state.pool, id, data).await {
        Ok(tpl) => {
            state
                .broadcast_sync(
                    SyncResource::LabelTemplate,
                    SyncChangeType::Updated,
                    &tpl.id.to_string(),
                    Some(&tpl),
                    true,
                )
                .await;
            StoreOpResult::ok().with_data(StoreOpData::LabelTemplate(tpl))
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}

pub async fn delete_label_template(state: &ServerState, id: i64) -> StoreOpResult {
    use crate::db::repository::label_template;

    match label_template::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>(
                    SyncResource::LabelTemplate,
                    SyncChangeType::Deleted,
                    &id.to_string(),
                    None,
                    true,
                )
                .await;
            StoreOpResult::ok()
        }
        Err(e) => StoreOpResult::err(e.to_string()),
    }
}
