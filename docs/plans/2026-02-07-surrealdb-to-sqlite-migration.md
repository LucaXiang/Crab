# SurrealDB to SQLite + sqlx Migration Design

## Summary

Replace SurrealDB with SQLite (via sqlx) across the entire workspace. This is a clean rebuild (no data migration) that also fixes accumulated design debt.

## Key Decisions

| Decision | Choice |
|----------|--------|
| Database | SQLite + sqlx |
| Query mode | `sqlx::query_as!` macro (compile-time checked), runtime `sqlx::query()` for archive dynamic queries |
| ID system | `INTEGER PRIMARY KEY` everywhere, drop `"table:id"` format |
| Money storage | Integer cents (`i64`), full `rust_decimal` in Rust code |
| Model layer | Single `shared::models` with `#[cfg_attr(feature = "db", derive(sqlx::FromRow))]` |
| Data migration | Clean rebuild, no legacy data |
| Migration scope | Full workspace |

## What Gets Deleted (~1400 lines)

| File/Module | Lines | Reason |
|-------------|-------|--------|
| `db/models/serde_helpers.rs` | 166 | RecordId serialization shim |
| `db/models/*.rs` (16 files) | ~800 | Duplicate DB models (RecordId variants) |
| `db/models/mod.rs` | 71 | Re-exports for duplicate models |
| `db/repository/mod.rs` (BaseRepository + RepoError string matching) | 155 | Wrapper layer |
| 17 Repository struct boilerplate | ~85 | `new()` constructors |
| `From` conversion implementations | ~100 | Type bridge between db::models and shared::models |
| Custom JSON string deserializers | ~50 | SurrealDB storage format hack |

## Architecture Changes

### Before

```
Frontend <-> shared::models (String ID) <-> db::models (RecordId) <-> SurrealDB
                                             ↑ serde_helpers.rs
                                             ↑ From conversions
                                             ↑ BaseRepository wrapper
```

### After

```
Frontend <-> shared::models (i64 ID + sqlx::FromRow) <-> SQLite
```

### Database Layer

```rust
// ServerState
pub struct ServerState {
    pub pool: SqlitePool,  // replaces Surreal<Db>
    // ...other fields unchanged
}

// No more BaseRepository/XxxRepository structs
// Module functions directly accept &SqlitePool
// db/employee.rs
pub async fn find_all(pool: &SqlitePool) -> Result<Vec<Employee>> {
    sqlx::query_as!(Employee, "SELECT ... FROM employee WHERE is_active = 1 ORDER BY username")
        .fetch_all(pool)
        .await
        .map_err(Into::into)
}
```

### Model Unification

```rust
// shared/src/models/employee.rs — single definition
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "db", derive(sqlx::FromRow))]
pub struct Employee {
    pub id: i64,
    pub username: String,
    pub display_name: String,
    pub role_id: i64,
    pub is_system: bool,
    pub is_active: bool,
}
```

Feature gate in shared:
```toml
# shared/Cargo.toml
[dependencies]
sqlx = { workspace = true, features = ["derive"], optional = true }

[features]
db = ["sqlx"]
```

### Error Handling

```rust
// Before: string matching to guess error type
if lower.contains("already exists") || lower.contains("duplicate") { ... }

// After: typed sqlx errors
impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => AppError::not_found("Record not found"),
            sqlx::Error::Database(e) if e.is_unique_violation() =>
                AppError::already_exists(e.message()),
            other => AppError::database(other.to_string()),
        }
    }
}
```

### Money: f64 -> Integer Cents

All amounts stored as integer cents in SQLite. Rust code uses `rust_decimal` throughout.

```
10.50 EUR -> stored as 1050 (INTEGER)
Rust: Decimal::new(1050, 2) or from_i64_with_scale(1050, 2)
Frontend: receives cents, divides by 100 for display
```

## Schema Design

### Reference Data

```sql
-- See full DDL in migration files
-- Key tables: role, employee, category, product, product_spec,
-- tag, product_tag, attribute, attribute_option, attribute_binding,
-- zone, dining_table, price_rule, store_info, system_state,
-- print_destination, label_template, image_ref, shift, daily_report,
-- system_issue
```

Key structural changes from SurrealDB:
- `Product.specs` embedded array -> `product_spec` table
- `Product.tags` embedded `Vec<RecordId>` -> `product_tag` junction table
- `Attribute.options` embedded array -> `attribute_option` table
- `has_attribute` graph edge -> `attribute_binding` table with `owner_type` + `owner_id`

### Archive Data

```sql
-- archived_order (from redb snapshot)
-- archived_order_item (FK: order_pk -> archived_order.id)
-- archived_order_item_option (FK: item_pk -> archived_order_item.id)
-- archived_order_payment (FK: order_pk -> archived_order.id)
-- archived_order_event (FK: order_pk -> archived_order.id)
-- payment (independent table for statistics)
-- archive_verification
```

Replaces SurrealDB graph model (RELATE edges) with standard foreign keys.

### Archive Write (replaces dynamic RELATE query building)

```rust
async fn archive_order_internal(&self, snapshot: &OrderSnapshot, events: &[OrderEvent]) -> Result<()> {
    let mut tx = self.pool.begin().await?;

    // 1. INSERT archived_order, get back id
    let order_pk = sqlx::query_scalar!(
        "INSERT INTO archived_order (...) VALUES (...) RETURNING id",
        ...
    ).fetch_one(&mut *tx).await?;

    // 2. INSERT items with order_pk FK
    for item in &snapshot.items {
        let item_pk = sqlx::query_scalar!(
            "INSERT INTO archived_order_item (order_pk, ...) VALUES (?, ...) RETURNING id",
            order_pk, ...
        ).fetch_one(&mut *tx).await?;

        // 3. INSERT item options with item_pk FK
        for opt in &item.selected_options {
            sqlx::query!("INSERT INTO archived_order_item_option (item_pk, ...) VALUES (?, ...)",
                item_pk, ...
            ).execute(&mut *tx).await?;
        }
    }

    // 4. INSERT payments, events similarly...
    tx.commit().await?;
    Ok(())
}
```

## Statistics Queries

Replace SurrealQL (LET variables, graph traversal, math:: functions) with standard SQL:

```sql
-- Overview (replaces 18 LET statements + magic take(18))
SELECT
    COALESCE(SUM(CASE WHEN status='COMPLETED' THEN total_cents ELSE 0 END), 0) AS revenue_cents,
    COUNT(CASE WHEN status='COMPLETED' THEN 1 END) AS order_count,
    -- ...
FROM archived_order WHERE end_time >= ? AND end_time < ?;

-- Category sales (replaces FROM has_item graph traversal)
SELECT oi.category_name AS name, SUM(oi.line_total_cents) AS value_cents
FROM archived_order_item oi
JOIN archived_order o ON o.id = oi.order_pk
WHERE o.status = 'COMPLETED' AND o.end_time >= ? AND o.end_time < ?
GROUP BY name ORDER BY value_cents DESC LIMIT 10;
```

## Frontend Impact

| Change | Frontend Action |
|--------|----------------|
| ID `"table:xxx"` -> number | TypeScript `id: string` -> `id: number` globally |
| Money f64 -> cents integer | `Currency` class receives cents, divides by 100 |
| `AttributeBinding.in/out` -> `owner_id/attribute_id` | Field rename |
| Tags from embedded array -> relation | API response format unchanged (backend assembles) |

## SQLite Configuration

- WAL mode enabled (concurrent reads + single writer)
- Connection pool: read 4 + write 1
- Migrations via `sqlx::migrate!("./migrations")`
- `PRAGMA foreign_keys = ON;`
- `PRAGMA journal_mode = WAL;`

## Migration Phases

### Phase 1: Infrastructure
- Add sqlx dependency to workspace Cargo.toml
- Create `DbService` with `SqlitePool`
- Write SQLite DDL migration files
- Add `db` feature to shared crate

### Phase 2: Unify Models
- Rewrite `shared::models` with `i64` IDs + `sqlx::FromRow`
- Delete `edge-server/src/db/models/` entirely
- Delete `serde_helpers.rs`

### Phase 3: Rewrite Repositories
- Replace 17 Repository structs with module functions
- Delete `BaseRepository`, `RepoError` string matching
- Implement `From<sqlx::Error> for AppError`

### Phase 4: Archive System
- Rewrite `archive.rs` with INSERT + foreign keys (no more RELATE)
- Rewrite `order.rs` detail query with JOINs
- Update hash chain logic

### Phase 5: Statistics
- Rewrite overview/trend/category/product queries in standard SQL
- Remove magic number `take(18)`

### Phase 6: Cleanup
- Remove `surrealdb` + `surrealdb-migrations` from Cargo.toml
- Delete `.surql` migration files
- Update `ServerState` (`Surreal<Db>` -> `SqlitePool`)
- Update all handler imports

### Phase 7: Frontend
- Update TypeScript types (`id: string` -> `id: number`)
- Update `Currency` to accept cents from API
- Remove `"table:id"` parse logic
- Rename `in/out` fields to `owner_id/attribute_id`

### Phase 8: Documentation
- Update CLAUDE.md files
- Update shared/CLAUDE.md ID convention
- Update docs/SURREALDB.md -> docs/SQLITE.md
