# Catalog Export Round-Trip Consistency Fix — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Fix export_zip() to include inactive categories, products, specs, attributes, and tag associations so export→import round-trip preserves all data.

**Architecture:** Export bypasses CatalogService runtime cache and queries SQLite directly. Two new helper functions (`export_all_categories`, `export_all_products`) perform full-table queries without `is_active` filtering. A new `attribute::find_all_with_inactive()` mirrors the existing `find_all()` without the active filter. `broadcast_catalog_sync()` also switches to DB queries for categories/products/attributes.

**Tech Stack:** Rust, sqlx (SQLite), shared models (Category, ProductFull, ProductSpec, Tag, Attribute)

---

### Task 1: Add `find_all_with_inactive()` to attribute repository

**Files:**
- Modify: `edge-server/src/db/repository/attribute.rs:15-24`

**Step 1: Add the new function after existing `find_all()`**

Add this function right after `find_all()` (after line 24):

```rust
/// Find all attributes including inactive ones (for export)
pub async fn find_all_with_inactive(pool: &SqlitePool) -> RepoResult<Vec<Attribute>> {
    let mut attrs = sqlx::query_as::<_, Attribute>(
        "SELECT id, name, is_multi_select, max_selections, COALESCE(default_option_ids, 'null') as default_option_ids, display_order, is_active, show_on_receipt, receipt_name, show_on_kitchen_print, kitchen_print_name FROM attribute ORDER BY display_order",
    )
    .fetch_all(pool)
    .await?;

    batch_load_options(pool, &mut attrs).await?;
    Ok(attrs)
}
```

**Step 2: Verify compilation**

Run: `cargo check -p edge-server`
Expected: success

**Step 3: Commit**

```
git add edge-server/src/db/repository/attribute.rs
git commit -m "feat(edge): add attribute::find_all_with_inactive() for export"
```

---

### Task 2: Rewrite `export_zip()` to query DB directly

**Files:**
- Modify: `edge-server/src/api/data_transfer/handler.rs:57-111`

**Step 1: Add `export_all_categories()` helper at bottom of file (before `broadcast_catalog_sync`)**

Add before `broadcast_catalog_sync` function (before line 546):

```rust
// =============================================================================
// Export helpers — direct DB queries (include inactive records)
// =============================================================================

/// Load ALL categories from DB (including inactive) for export.
/// Unlike CatalogService.warmup() which filters is_active=1, this returns everything.
async fn export_all_categories(pool: &sqlx::SqlitePool) -> Result<Vec<shared::models::Category>, AppError> {
    let rows: Vec<shared::models::Category> = sqlx::query_as(
        "SELECT id, name, sort_order, is_kitchen_print_enabled, is_label_print_enabled, is_active, is_virtual, match_mode, is_display FROM category ORDER BY sort_order",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    let mut categories = Vec::with_capacity(rows.len());
    for mut cat in rows {
        let cat_id = cat.id;

        cat.kitchen_print_destinations = sqlx::query_scalar!(
            "SELECT cpd.print_destination_id FROM category_print_dest cpd JOIN print_destination pd ON pd.id = cpd.print_destination_id WHERE cpd.category_id = ? AND pd.purpose = 'kitchen'",
            cat_id
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        cat.label_print_destinations = sqlx::query_scalar!(
            "SELECT cpd.print_destination_id FROM category_print_dest cpd JOIN print_destination pd ON pd.id = cpd.print_destination_id WHERE cpd.category_id = ? AND pd.purpose = 'label'",
            cat_id
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        cat.tag_ids = sqlx::query_scalar!(
            "SELECT tag_id FROM category_tag WHERE category_id = ?",
            cat_id
        )
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        categories.push(cat);
    }

    Ok(categories)
}

/// Load ALL products from DB (including inactive) for export.
/// Loads all specs (including inactive) and all tag associations (including inactive tags).
/// Sets `attributes: vec![]` since export uses top-level `attribute_bindings`.
async fn export_all_products(pool: &sqlx::SqlitePool) -> Result<Vec<shared::models::ProductFull>, AppError> {
    let products: Vec<shared::models::Product> = sqlx::query_as(
        "SELECT id, name, image, category_id, sort_order, tax_rate, receipt_name, kitchen_print_name, is_kitchen_print_enabled, is_label_print_enabled, is_active, external_id FROM product ORDER BY sort_order",
    )
    .fetch_all(pool)
    .await
    .map_err(|e| AppError::database(e.to_string()))?;

    let mut result = Vec::with_capacity(products.len());
    for product in products {
        let product_id = product.id;

        // All specs (no is_active filter)
        let specs: Vec<shared::models::ProductSpec> = sqlx::query_as(
            "SELECT id, product_id, name, price, display_order, is_default, is_active, receipt_name, is_root FROM product_spec WHERE product_id = ? ORDER BY display_order",
        )
        .bind(product_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        // All tag associations (no t.is_active filter)
        let tags: Vec<shared::models::Tag> = sqlx::query_as(
            "SELECT t.id, t.name, t.color, t.display_order, t.is_active, t.is_system FROM tag t JOIN product_tag pt ON t.id = pt.tag_id WHERE pt.product_id = ?",
        )
        .bind(product_id)
        .fetch_all(pool)
        .await
        .unwrap_or_default();

        result.push(shared::models::ProductFull {
            id: product.id,
            name: product.name,
            image: product.image,
            category_id: product.category_id,
            sort_order: product.sort_order,
            tax_rate: product.tax_rate,
            receipt_name: product.receipt_name,
            kitchen_print_name: product.kitchen_print_name,
            is_kitchen_print_enabled: product.is_kitchen_print_enabled,
            is_label_print_enabled: product.is_label_print_enabled,
            is_active: product.is_active,
            external_id: product.external_id,
            specs,
            attributes: vec![],
            tags,
        });
    }

    Ok(result)
}
```

**Step 2: Replace `export_zip()` lines 57-111**

Replace the existing `export_zip()` body. Change lines 58-59 and 64:

```rust
/// Build catalog export ZIP bytes
pub(super) async fn export_zip(state: &ServerState) -> Result<Vec<u8>, AppError> {
    // Direct DB queries — include inactive records (unlike CatalogService cache)
    let categories = export_all_categories(&state.pool).await?;
    let products = export_all_products(&state.pool).await?;
    let mut tags = tag::find_all_with_inactive(&state.pool)
        .await
        .map_err(AppError::from)?;
    tags.sort_by_key(|t| t.display_order);
    let attributes = attribute::find_all_with_inactive(&state.pool)
        .await
        .map_err(AppError::from)?;
    let all_bindings = attribute::find_all_bindings(&state.pool)
        .await
        .map_err(AppError::from)?;
    let price_rules = price_rule::find_all_with_inactive(&state.pool)
        .await
        .map_err(AppError::from)?;
    let zones = zone::find_all_with_inactive(&state.pool)
        .await
        .map_err(AppError::from)?;
    let dining_tables = dining_table::find_all_with_inactive(&state.pool)
        .await
        .map_err(AppError::from)?;

    // Filter bindings: only include those referencing exported entities + attributes
    let exported_category_ids: std::collections::HashSet<i64> =
        categories.iter().map(|c| c.id).collect();
    let exported_product_ids: std::collections::HashSet<i64> =
        products.iter().map(|p| p.id).collect();
    let exported_attribute_ids: std::collections::HashSet<i64> =
        attributes.iter().map(|a| a.id).collect();

    let bindings: Vec<_> = all_bindings
        .into_iter()
        .filter(|b| {
            let owner_valid = match b.owner_type.as_str() {
                "product" => exported_product_ids.contains(&b.owner_id),
                "category" => exported_category_ids.contains(&b.owner_id),
                _ => false,
            };
            owner_valid && exported_attribute_ids.contains(&b.attribute_id)
        })
        .collect();

    let catalog = CatalogExport {
        version: 1,
        exported_at: shared::util::now_millis(),
        tags,
        categories,
        products,
        attributes,
        attribute_bindings: bindings,
        price_rules,
        zones,
        dining_tables,
    };
```

The rest of `export_zip()` (ZIP building from line 113 onwards) stays unchanged.

**Step 3: Verify compilation**

Run: `cargo check -p edge-server`
Expected: success

**Step 4: Commit**

```
git add edge-server/src/api/data_transfer/handler.rs
git commit -m "fix(edge): export_zip() queries DB directly, includes inactive records"
```

---

### Task 3: Fix `broadcast_catalog_sync()` to query DB directly

**Files:**
- Modify: `edge-server/src/api/data_transfer/handler.rs:546-671`

**Step 1: Replace categories and products sections in `broadcast_catalog_sync()`**

Replace lines 562-588 (categories from cache + products from cache) with direct DB queries:

```rust
    // Categories (direct DB query — includes inactive after import)
    if let Ok(categories) = export_all_categories(&state.pool).await {
        for c in &categories {
            state
                .broadcast_sync(
                    SyncResource::Category,
                    SyncChangeType::Updated,
                    c.id,
                    Some(c),
                    false,
                )
                .await;
        }
    }

    // Products (direct DB query — includes inactive after import)
    if let Ok(products) = export_all_products(&state.pool).await {
        for p in &products {
            state
                .broadcast_sync(
                    SyncResource::Product,
                    SyncChangeType::Updated,
                    p.id,
                    Some(p),
                    false,
                )
                .await;
        }
    }
```

**Step 2: Replace attributes section (line 591) to use `find_all_with_inactive`**

```rust
    // Attributes (includes inactive after import)
    if let Ok(attrs) = attribute::find_all_with_inactive(&state.pool).await {
```

**Step 3: Verify compilation**

Run: `cargo check -p edge-server`
Expected: success

**Step 4: Commit**

```
git add edge-server/src/api/data_transfer/handler.rs
git commit -m "fix(edge): broadcast_catalog_sync() queries DB directly for full sync"
```

---

### Task 4: Verify — clippy + tests

**Step 1: Run clippy**

Run: `cargo clippy --workspace`
Expected: zero warnings, zero errors

**Step 2: Run tests**

Run: `cargo test --workspace --lib`
Expected: all tests pass

**Step 3: Final commit (if any clippy fixes needed)**

---

### Task 5: Commit all and verify

**Step 1: Check git status**

Run: `git diff --stat`
Expected: 2 files changed:
- `edge-server/src/db/repository/attribute.rs` (new function)
- `edge-server/src/api/data_transfer/handler.rs` (export_zip + helpers + broadcast_catalog_sync)
