//! Category database operations

use serde::{Deserialize, Serialize};
use shared::cloud::store_op::StoreOpData;
use shared::models::category::{Category, CategoryCreate};
use sqlx::PgPool;

use super::BoxError;

// ── Edge Sync ──

pub async fn upsert_category_from_sync(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
    data: &serde_json::Value,
    _version: i64,
    now: i64,
) -> Result<(), BoxError> {
    let cat: Category = serde_json::from_value(data.clone())?;

    let mut tx = pool.begin().await?;

    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        INSERT INTO store_categories (
            store_id, source_id, name, sort_order,
            is_kitchen_print_enabled, is_label_print_enabled,
            is_active, is_virtual, match_mode, is_display, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
        ON CONFLICT (store_id, source_id)
        DO UPDATE SET
            name = EXCLUDED.name, sort_order = EXCLUDED.sort_order,
            is_kitchen_print_enabled = EXCLUDED.is_kitchen_print_enabled,
            is_label_print_enabled = EXCLUDED.is_label_print_enabled,
            is_active = EXCLUDED.is_active, is_virtual = EXCLUDED.is_virtual,
            match_mode = EXCLUDED.match_mode, is_display = EXCLUDED.is_display,
            updated_at = EXCLUDED.updated_at
        WHERE store_categories.updated_at <= EXCLUDED.updated_at
        RETURNING id
        "#,
    )
    .bind(store_id)
    .bind(source_id)
    .bind(&cat.name)
    .bind(cat.sort_order)
    .bind(cat.is_kitchen_print_enabled)
    .bind(cat.is_label_print_enabled)
    .bind(cat.is_active)
    .bind(cat.is_virtual)
    .bind(&cat.match_mode)
    .bind(cat.is_display)
    .bind(now)
    .fetch_optional(&mut *tx)
    .await?;

    let Some((pg_id,)) = row else {
        tx.commit().await?;
        return Ok(());
    };

    // Replace print destinations
    sqlx::query("DELETE FROM store_category_print_dest WHERE category_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    {
        let mut cat_ids = Vec::new();
        let mut dest_ids = Vec::new();
        let mut purposes = Vec::new();
        for dest_id in &cat.kitchen_print_destinations {
            cat_ids.push(pg_id);
            dest_ids.push(*dest_id);
            purposes.push("kitchen".to_string());
        }
        for dest_id in &cat.label_print_destinations {
            cat_ids.push(pg_id);
            dest_ids.push(*dest_id);
            purposes.push("label".to_string());
        }
        if !cat_ids.is_empty() {
            sqlx::query(
                "INSERT INTO store_category_print_dest (category_id, dest_source_id, purpose) SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[])",
            )
            .bind(&cat_ids)
            .bind(&dest_ids)
            .bind(&purposes)
            .execute(&mut *tx)
            .await?;
        }
    }

    // Replace tag associations
    sqlx::query("DELETE FROM store_category_tag WHERE category_id = $1")
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;

    if !cat.tag_ids.is_empty() {
        let cat_ids: Vec<i64> = cat.tag_ids.iter().map(|_| pg_id).collect();
        sqlx::query(
            "INSERT INTO store_category_tag (category_id, tag_source_id) SELECT * FROM UNNEST($1::bigint[], $2::bigint[])",
        )
        .bind(&cat_ids)
        .bind(&cat.tag_ids)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn delete_category(pool: &PgPool, store_id: i64, source_id: i64) -> Result<(), BoxError> {
    sqlx::query("DELETE FROM store_categories WHERE store_id = $1 AND source_id = $2")
        .bind(store_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    Ok(())
}

// ── Console Read Types ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreCategory {
    pub source_id: i64,
    pub name: String,
    pub sort_order: i32,
    pub is_kitchen_print_enabled: bool,
    pub is_label_print_enabled: bool,
    pub is_active: bool,
    pub is_virtual: bool,
    pub match_mode: String,
    pub is_display: bool,
    pub kitchen_print_destinations: Vec<i64>,
    pub label_print_destinations: Vec<i64>,
    pub tag_ids: Vec<i64>,
}

// ── Console Read ──

pub async fn list_categories(pool: &PgPool, store_id: i64) -> Result<Vec<StoreCategory>, BoxError> {
    #[derive(sqlx::FromRow)]
    struct Row {
        id: i64,
        source_id: i64,
        name: String,
        sort_order: i32,
        is_kitchen_print_enabled: bool,
        is_label_print_enabled: bool,
        is_active: bool,
        is_virtual: bool,
        match_mode: String,
        is_display: bool,
    }

    let rows: Vec<Row> = sqlx::query_as(
        r#"
        SELECT id, source_id, name, sort_order,
               is_kitchen_print_enabled, is_label_print_enabled,
               is_active, is_virtual, match_mode, is_display
        FROM store_categories
        WHERE store_id = $1
        ORDER BY sort_order, source_id
        "#,
    )
    .bind(store_id)
    .fetch_all(pool)
    .await?;

    let pg_ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    if pg_ids.is_empty() {
        return Ok(vec![]);
    }

    let dest_rows: Vec<(i64, i64, String)> = sqlx::query_as(
        "SELECT category_id, dest_source_id, purpose FROM store_category_print_dest WHERE category_id = ANY($1)",
    )
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    let tag_rows: Vec<(i64, i64)> = sqlx::query_as(
        "SELECT category_id, tag_source_id FROM store_category_tag WHERE category_id = ANY($1)",
    )
    .bind(&pg_ids)
    .fetch_all(pool)
    .await?;

    let mut kitchen_dest_map: std::collections::HashMap<i64, Vec<i64>> =
        std::collections::HashMap::new();
    let mut label_dest_map: std::collections::HashMap<i64, Vec<i64>> =
        std::collections::HashMap::new();
    for (cat_id, dest_id, purpose) in dest_rows {
        match purpose.as_str() {
            "kitchen" => kitchen_dest_map.entry(cat_id).or_default().push(dest_id),
            "label" => label_dest_map.entry(cat_id).or_default().push(dest_id),
            _ => {}
        }
    }

    let mut tag_map: std::collections::HashMap<i64, Vec<i64>> = std::collections::HashMap::new();
    for (cat_id, tag_id) in tag_rows {
        tag_map.entry(cat_id).or_default().push(tag_id);
    }

    Ok(rows
        .into_iter()
        .map(|r| StoreCategory {
            source_id: r.source_id,
            name: r.name,
            sort_order: r.sort_order,
            is_kitchen_print_enabled: r.is_kitchen_print_enabled,
            is_label_print_enabled: r.is_label_print_enabled,
            is_active: r.is_active,
            is_virtual: r.is_virtual,
            match_mode: r.match_mode,
            is_display: r.is_display,
            kitchen_print_destinations: kitchen_dest_map.remove(&r.id).unwrap_or_default(),
            label_print_destinations: label_dest_map.remove(&r.id).unwrap_or_default(),
            tag_ids: tag_map.remove(&r.id).unwrap_or_default(),
        })
        .collect())
}

// ── Console CRUD ──

pub async fn create_category_direct(
    pool: &PgPool,
    store_id: i64,
    data: &CategoryCreate,
) -> Result<(i64, StoreOpData), BoxError> {
    let now = shared::util::now_millis();
    let sort_order = data.sort_order.unwrap_or(0);
    let is_kitchen_print_enabled = data.is_kitchen_print_enabled.unwrap_or(false);
    let is_label_print_enabled = data.is_label_print_enabled.unwrap_or(false);
    let is_virtual = data.is_virtual.unwrap_or(false);
    let match_mode = data.match_mode.as_deref().unwrap_or("any");
    let is_display = data.is_display.unwrap_or(true);

    let source_id = super::snowflake_id();
    let mut tx = pool.begin().await?;

    let (pg_id,): (i64,) = sqlx::query_as(
        r#"
        INSERT INTO store_categories (
            store_id, source_id, name, sort_order,
            is_kitchen_print_enabled, is_label_print_enabled,
            is_active, is_virtual, match_mode, is_display, updated_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, TRUE, $7, $8, $9, $10)
        RETURNING id
        "#,
    )
    .bind(store_id)
    .bind(source_id)
    .bind(&data.name)
    .bind(sort_order)
    .bind(is_kitchen_print_enabled)
    .bind(is_label_print_enabled)
    .bind(is_virtual)
    .bind(match_mode)
    .bind(is_display)
    .bind(now)
    .fetch_one(&mut *tx)
    .await?;

    {
        let mut cat_ids = Vec::new();
        let mut dest_ids = Vec::new();
        let mut purposes = Vec::new();
        for dest_id in &data.kitchen_print_destinations {
            cat_ids.push(pg_id);
            dest_ids.push(*dest_id);
            purposes.push("kitchen".to_string());
        }
        for dest_id in &data.label_print_destinations {
            cat_ids.push(pg_id);
            dest_ids.push(*dest_id);
            purposes.push("label".to_string());
        }
        if !cat_ids.is_empty() {
            sqlx::query(
                "INSERT INTO store_category_print_dest (category_id, dest_source_id, purpose) SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[])",
            )
            .bind(&cat_ids)
            .bind(&dest_ids)
            .bind(&purposes)
            .execute(&mut *tx)
            .await?;
        }
    }
    if !data.tag_ids.is_empty() {
        let cat_ids: Vec<i64> = data.tag_ids.iter().map(|_| pg_id).collect();
        sqlx::query(
            "INSERT INTO store_category_tag (category_id, tag_source_id) SELECT * FROM UNNEST($1::bigint[], $2::bigint[])",
        )
        .bind(&cat_ids)
        .bind(&data.tag_ids)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;

    let cat = Category {
        id: source_id,
        name: data.name.clone(),
        sort_order,
        is_kitchen_print_enabled,
        is_label_print_enabled,
        is_active: true,
        is_virtual,
        match_mode: match_mode.to_string(),
        is_display,
        kitchen_print_destinations: data.kitchen_print_destinations.clone(),
        label_print_destinations: data.label_print_destinations.clone(),
        tag_ids: data.tag_ids.clone(),
    };
    Ok((source_id, StoreOpData::Category(cat)))
}

pub async fn update_category_direct(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
    data: &shared::models::category::CategoryUpdate,
) -> Result<(), BoxError> {
    let now = shared::util::now_millis();
    let mut tx = pool.begin().await?;

    let pg_id: i64 = sqlx::query_scalar(
        "SELECT id FROM store_categories WHERE store_id = $1 AND source_id = $2",
    )
    .bind(store_id)
    .bind(source_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or("Category not found")?;

    sqlx::query(
        r#"
        UPDATE store_categories SET
            name = COALESCE($1, name),
            sort_order = COALESCE($2, sort_order),
            is_kitchen_print_enabled = COALESCE($3, is_kitchen_print_enabled),
            is_label_print_enabled = COALESCE($4, is_label_print_enabled),
            is_virtual = COALESCE($5, is_virtual),
            match_mode = COALESCE($6, match_mode),
            is_active = COALESCE($7, is_active),
            is_display = COALESCE($8, is_display),
            updated_at = $9
        WHERE id = $10
        "#,
    )
    .bind(&data.name)
    .bind(data.sort_order)
    .bind(data.is_kitchen_print_enabled)
    .bind(data.is_label_print_enabled)
    .bind(data.is_virtual)
    .bind(&data.match_mode)
    .bind(data.is_active)
    .bind(data.is_display)
    .bind(now)
    .bind(pg_id)
    .execute(&mut *tx)
    .await?;

    if let Some(ref dests) = data.kitchen_print_destinations {
        sqlx::query(
            "DELETE FROM store_category_print_dest WHERE category_id = $1 AND purpose = 'kitchen'",
        )
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;
        if !dests.is_empty() {
            let cat_ids: Vec<i64> = dests.iter().map(|_| pg_id).collect();
            let purposes: Vec<String> = dests.iter().map(|_| "kitchen".to_string()).collect();
            sqlx::query(
                "INSERT INTO store_category_print_dest (category_id, dest_source_id, purpose) SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[])",
            )
            .bind(&cat_ids)
            .bind(dests)
            .bind(&purposes)
            .execute(&mut *tx)
            .await?;
        }
    }
    if let Some(ref dests) = data.label_print_destinations {
        sqlx::query(
            "DELETE FROM store_category_print_dest WHERE category_id = $1 AND purpose = 'label'",
        )
        .bind(pg_id)
        .execute(&mut *tx)
        .await?;
        if !dests.is_empty() {
            let cat_ids: Vec<i64> = dests.iter().map(|_| pg_id).collect();
            let purposes: Vec<String> = dests.iter().map(|_| "label".to_string()).collect();
            sqlx::query(
                "INSERT INTO store_category_print_dest (category_id, dest_source_id, purpose) SELECT * FROM UNNEST($1::bigint[], $2::bigint[], $3::text[])",
            )
            .bind(&cat_ids)
            .bind(dests)
            .bind(&purposes)
            .execute(&mut *tx)
            .await?;
        }
    }

    if let Some(ref tags) = data.tag_ids {
        sqlx::query("DELETE FROM store_category_tag WHERE category_id = $1")
            .bind(pg_id)
            .execute(&mut *tx)
            .await?;
        if !tags.is_empty() {
            let cat_ids: Vec<i64> = tags.iter().map(|_| pg_id).collect();
            sqlx::query(
                "INSERT INTO store_category_tag (category_id, tag_source_id) SELECT * FROM UNNEST($1::bigint[], $2::bigint[])",
            )
            .bind(&cat_ids)
            .bind(tags)
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

pub async fn batch_update_sort_order_categories(
    pool: &PgPool,
    store_id: i64,
    items: &[shared::cloud::store_op::SortOrderItem],
) -> Result<(), BoxError> {
    if items.is_empty() {
        return Ok(());
    }
    let now = shared::util::now_millis();
    let ids: Vec<i64> = items.iter().map(|i| i.id).collect();
    let orders: Vec<i32> = items.iter().map(|i| i.sort_order).collect();
    let nows: Vec<i64> = items.iter().map(|_| now).collect();
    sqlx::query(
        r#"UPDATE store_categories SET sort_order = u.sort_order, updated_at = u.updated_at
        FROM (SELECT * FROM UNNEST($1::bigint[], $2::integer[], $3::bigint[])) AS u(source_id, sort_order, updated_at)
        WHERE store_categories.store_id = $4 AND store_categories.source_id = u.source_id"#,
    )
    .bind(&ids)
    .bind(&orders)
    .bind(&nows)
    .bind(store_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_category_direct(
    pool: &PgPool,
    store_id: i64,
    source_id: i64,
) -> Result<(), BoxError> {
    let rows = sqlx::query("DELETE FROM store_categories WHERE store_id = $1 AND source_id = $2")
        .bind(store_id)
        .bind(source_id)
        .execute(pool)
        .await?;
    if rows.rows_affected() == 0 {
        return Err("Category not found".into());
    }
    Ok(())
}
