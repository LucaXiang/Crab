# Name Field Cleanup Design

**Date**: 2026-02-26
**Status**: Approved

## Goal

Remove redundant name fields across all entities. Unify to `name` (primary display) + `receipt_name?` (optional print override).

## Principle

- Every entity has exactly ONE primary name: `name`
- `receipt_name` is always `Option<String>`, fallback to `name` when empty
- No more `display_name` or redundant internal `name`

## Entity Changes

| Entity | Before | After | Migration |
|--------|--------|-------|-----------|
| **PriceRule** | name + display_name + receipt_name(required) | name + receipt_name? | name ← display_name; drop display_name; receipt_name nullable |
| **MgDiscountRule** | name + display_name + receipt_name(required) | name + receipt_name? | name ← display_name; drop display_name; receipt_name nullable |
| **Role** | name(UNIQUE) + display_name | name(UNIQUE) | name ← display_name; drop display_name |
| **MarketingGroup** | name(UNIQUE) + display_name | name(UNIQUE) | name ← display_name; drop display_name |
| **StampActivity** | name + display_name | name | name ← display_name; drop display_name |
| **Employee** | username + display_name | username + name | rename display_name → name |
| **Product** | name + receipt_name? | no change | already correct |
| **Attribute** | name + receipt_name? | no change | already correct |
| **AttributeOption** | name + receipt_name? | no change | already correct |
| **AppliedRule** (order event) | name + display_name + receipt_name | name + receipt_name? | struct field removal |

## Fallback Logic

```rust
// Wherever receipt output is needed:
fn effective_receipt_name(&self) -> &str {
    self.receipt_name.as_deref().unwrap_or(&self.name)
}
```

## Database Migration

### Edge Server (SQLite)

SQLite doesn't support DROP COLUMN well — use table recreation pattern for affected tables.

For each affected table:
1. Create new table with correct schema
2. INSERT INTO new SELECT (with `name = display_name` transform)
3. DROP old table
4. ALTER TABLE new RENAME TO original

### Cloud (PostgreSQL)

```sql
-- PriceRule example:
UPDATE store_price_rules SET name = display_name;
ALTER TABLE store_price_rules DROP COLUMN display_name;
ALTER TABLE store_price_rules ALTER COLUMN receipt_name DROP NOT NULL;

-- Employee:
ALTER TABLE store_employees RENAME COLUMN display_name TO name;
```

## Full-Stack Change Scope

1. **shared/** — Model structs: PriceRule, PriceRuleCreate, PriceRuleUpdate, MgDiscountRule*, Role, MarketingGroup, StampActivity, Employee, AppliedRule
2. **edge-server/** — SQLite migration + all repository queries referencing display_name
3. **crab-cloud/** — PostgreSQL migration + repository queries + sync logic
4. **red_coral/** — TypeScript types + forms (remove internal name input, make receipt_name optional)
5. **crab-console/** — TypeScript types + forms (same changes)

## Notes

- Product/Attribute/AttributeOption already follow the target pattern — no changes needed
- `kitchen_print_name` on Product/Attribute is a separate concern, not affected
- AppliedRule in order events: historical events keep old format (deserialization must handle both)
