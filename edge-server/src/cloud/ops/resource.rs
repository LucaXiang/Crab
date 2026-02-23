//! Employee + Zone + DiningTable + PriceRule operations (via repository)

use shared::cloud::catalog::{CatalogOpData, CatalogOpResult};

use crate::core::state::ServerState;

// ── Employee ──

pub async fn create_employee(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: shared::models::EmployeeCreate,
) -> CatalogOpResult {
    use crate::db::repository::employee;

    match employee::create(&state.pool, assigned_id, data).await {
        Ok(emp) => {
            state
                .broadcast_sync("employee", "created", &emp.id.to_string(), Some(&emp))
                .await;
            CatalogOpResult::created(emp.id).with_data(CatalogOpData::Employee(emp))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn update_employee(
    state: &ServerState,
    id: i64,
    data: shared::models::EmployeeUpdate,
) -> CatalogOpResult {
    use crate::db::repository::employee;

    match employee::update(&state.pool, id, data).await {
        Ok(emp) => {
            state
                .broadcast_sync("employee", "updated", &emp.id.to_string(), Some(&emp))
                .await;
            CatalogOpResult::ok().with_data(CatalogOpData::Employee(emp))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn delete_employee(state: &ServerState, id: i64) -> CatalogOpResult {
    use crate::db::repository::employee;

    match employee::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>("employee", "deleted", &id.to_string(), None)
                .await;
            CatalogOpResult::ok()
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

// ── Zone ──

pub async fn create_zone(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: shared::models::ZoneCreate,
) -> CatalogOpResult {
    use crate::db::repository::zone;

    match zone::create(&state.pool, assigned_id, data).await {
        Ok(z) => {
            state
                .broadcast_sync("zone", "created", &z.id.to_string(), Some(&z))
                .await;
            CatalogOpResult::created(z.id).with_data(CatalogOpData::Zone(z))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn update_zone(
    state: &ServerState,
    id: i64,
    data: shared::models::ZoneUpdate,
) -> CatalogOpResult {
    use crate::db::repository::zone;

    match zone::update(&state.pool, id, data).await {
        Ok(z) => {
            state
                .broadcast_sync("zone", "updated", &z.id.to_string(), Some(&z))
                .await;
            CatalogOpResult::ok().with_data(CatalogOpData::Zone(z))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn delete_zone(state: &ServerState, id: i64) -> CatalogOpResult {
    use crate::db::repository::zone;

    match zone::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>("zone", "deleted", &id.to_string(), None)
                .await;
            CatalogOpResult::ok()
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

// ── DiningTable ──

pub async fn create_table(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: shared::models::DiningTableCreate,
) -> CatalogOpResult {
    use crate::db::repository::dining_table;

    match dining_table::create(&state.pool, assigned_id, data).await {
        Ok(t) => {
            state
                .broadcast_sync("dining_table", "created", &t.id.to_string(), Some(&t))
                .await;
            CatalogOpResult::created(t.id).with_data(CatalogOpData::Table(t))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn update_table(
    state: &ServerState,
    id: i64,
    data: shared::models::DiningTableUpdate,
) -> CatalogOpResult {
    use crate::db::repository::dining_table;

    match dining_table::update(&state.pool, id, data).await {
        Ok(t) => {
            state
                .broadcast_sync("dining_table", "updated", &t.id.to_string(), Some(&t))
                .await;
            CatalogOpResult::ok().with_data(CatalogOpData::Table(t))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn delete_table(state: &ServerState, id: i64) -> CatalogOpResult {
    use crate::db::repository::dining_table;

    match dining_table::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>("dining_table", "deleted", &id.to_string(), None)
                .await;
            CatalogOpResult::ok()
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

// ── PriceRule ──

pub async fn create_price_rule(
    state: &ServerState,
    assigned_id: Option<i64>,
    data: shared::models::PriceRuleCreate,
) -> CatalogOpResult {
    use crate::db::repository::price_rule;

    match price_rule::create(&state.pool, assigned_id, data).await {
        Ok(rule) => {
            state
                .broadcast_sync("price_rule", "created", &rule.id.to_string(), Some(&rule))
                .await;
            CatalogOpResult::created(rule.id).with_data(CatalogOpData::PriceRule(rule))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn update_price_rule(
    state: &ServerState,
    id: i64,
    data: shared::models::PriceRuleUpdate,
) -> CatalogOpResult {
    use crate::db::repository::price_rule;

    match price_rule::update(&state.pool, id, data).await {
        Ok(rule) => {
            state
                .broadcast_sync("price_rule", "updated", &rule.id.to_string(), Some(&rule))
                .await;
            CatalogOpResult::ok().with_data(CatalogOpData::PriceRule(rule))
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}

pub async fn delete_price_rule(state: &ServerState, id: i64) -> CatalogOpResult {
    use crate::db::repository::price_rule;

    match price_rule::delete(&state.pool, id).await {
        Ok(_) => {
            state
                .broadcast_sync::<()>("price_rule", "deleted", &id.to_string(), None)
                .await;
            CatalogOpResult::ok()
        }
        Err(e) => CatalogOpResult::err(e.to_string()),
    }
}
