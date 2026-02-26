# Name Field Cleanup Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Remove redundant `display_name` fields across all entities, unify to `name` + optional `receipt_name`

**Architecture:** Bottom-up migration: database schema → shared Rust types → edge-server queries → cloud queries → frontend types → forms/UI. Dev stage — no backward compatibility needed.

**Tech Stack:** Rust (shared/edge-server/crab-cloud), TypeScript (red_coral/crab-console), SQLite, PostgreSQL, sqlx

---

### Task 1: Edge Server SQLite Migration

**Files:**
- Create: `edge-server/migrations/0004_name_field_cleanup.sql`

**Step 1: Write the migration**

The migration must handle 6 tables. SQLite 3.35+ supports DROP COLUMN. For `employee`, use RENAME COLUMN.

```sql
-- =============================================================================
-- 0004_name_field_cleanup.sql
-- Unify name fields: remove display_name, make receipt_name optional
-- =============================================================================

-- 1. role: name ← display_name, drop display_name
UPDATE role SET name = display_name WHERE display_name != '';
ALTER TABLE role DROP COLUMN display_name;

-- 2. employee: rename display_name → name
ALTER TABLE employee RENAME COLUMN display_name TO name;

-- 3. price_rule: name ← display_name, drop old name semantics, drop display_name
--    Need temp column since we're replacing name's value with display_name's value
ALTER TABLE price_rule ADD COLUMN new_name TEXT NOT NULL DEFAULT '';
UPDATE price_rule SET new_name = display_name;
ALTER TABLE price_rule DROP COLUMN display_name;
ALTER TABLE price_rule DROP COLUMN name;
ALTER TABLE price_rule RENAME COLUMN new_name TO name;

-- 4. marketing_group: name ← display_name, drop display_name
UPDATE marketing_group SET name = display_name;
ALTER TABLE marketing_group DROP COLUMN display_name;

-- 5. mg_discount_rule: same as price_rule pattern
ALTER TABLE mg_discount_rule ADD COLUMN new_name TEXT NOT NULL DEFAULT '';
UPDATE mg_discount_rule SET new_name = display_name;
ALTER TABLE mg_discount_rule DROP COLUMN display_name;
ALTER TABLE mg_discount_rule DROP COLUMN name;
ALTER TABLE mg_discount_rule RENAME COLUMN new_name TO name;

-- 6. stamp_activity: same pattern
ALTER TABLE stamp_activity ADD COLUMN new_name TEXT NOT NULL DEFAULT '';
UPDATE stamp_activity SET new_name = display_name;
ALTER TABLE stamp_activity DROP COLUMN display_name;
ALTER TABLE stamp_activity DROP COLUMN name;
ALTER TABLE stamp_activity RENAME COLUMN new_name TO name;
```

**Step 2: Verify migration runs**

Run: `sqlx migrate run --source edge-server/migrations`
Expected: Migration applied successfully

**Step 3: Commit**

```bash
git add edge-server/migrations/0004_name_field_cleanup.sql
git commit -m "feat(edge): add migration 0004 - unify name fields"
```

---

### Task 2: Cloud PostgreSQL Migration

**Files:**
- Create: `crab-cloud/migrations/0008_name_field_cleanup.up.sql`
- Create: `crab-cloud/migrations/0008_name_field_cleanup.down.sql`

**Step 1: Write the up migration**

```sql
-- store_price_rules: name ← display_name, drop display_name, receipt_name nullable
UPDATE store_price_rules SET name = display_name;
ALTER TABLE store_price_rules DROP COLUMN display_name;
ALTER TABLE store_price_rules ALTER COLUMN receipt_name DROP NOT NULL;

-- store_employees: rename display_name → name
ALTER TABLE store_employees RENAME COLUMN display_name TO name;
```

Note: MarketingGroup, MgDiscountRule, StampActivity, Role may not have cloud tables yet. Check and add if they exist.

**Step 2: Write the down migration**

```sql
-- store_price_rules: restore display_name
ALTER TABLE store_price_rules ADD COLUMN display_name TEXT NOT NULL DEFAULT '';
UPDATE store_price_rules SET display_name = name;
ALTER TABLE store_price_rules ALTER COLUMN receipt_name SET NOT NULL;

-- store_employees: rename back
ALTER TABLE store_employees RENAME COLUMN name TO display_name;
```

**Step 3: Commit**

```bash
git add crab-cloud/migrations/0008_name_field_cleanup.up.sql crab-cloud/migrations/0008_name_field_cleanup.down.sql
git commit -m "feat(cloud): add migration 0008 - unify name fields"
```

---

### Task 3: Shared Types — Models

**Files:**
- Modify: `shared/src/models/price_rule.rs` (lines 46-48, 80-82, 103-105)
- Modify: `shared/src/models/employee.rs` (lines 11, 23, 32)
- Modify: `shared/src/models/role.rs` (lines 10-11, 23-24, 32-33)
- Modify: `shared/src/models/marketing_group.rs` (lines 12-13, 25-26, 35-36, 48-50, 63-65, 75-77)
- Modify: `shared/src/models/stamp.rs` (lines 35-36, 50-51, 64-65)

**Step 1: Update PriceRule** (`shared/src/models/price_rule.rs`)

PriceRule struct:
- Remove `display_name: String` (line 47)
- Change `receipt_name: String` → `receipt_name: Option<String>` (line 48)
- Add `#[serde(skip_serializing_if = "Option::is_none")]` on receipt_name

PriceRuleCreate:
- Remove `display_name: String` (line 81)
- Change `receipt_name: String` → `receipt_name: Option<String>` (line 82)

PriceRuleUpdate:
- Remove `display_name: Option<String>` (line 104)
- `receipt_name` is already `Option<String>`, keep it

**Step 2: Update Employee** (`shared/src/models/employee.rs`)

Employee struct:
- Rename `display_name: String` → `name: String` (line 11)

EmployeeCreate:
- Rename `display_name: Option<String>` → `name: Option<String>` (line 23)

EmployeeUpdate:
- Rename `display_name: Option<String>` → `name: Option<String>` (line 32)

**Step 3: Update Role** (`shared/src/models/role.rs`)

Role struct:
- Remove `display_name: String` (line 11) — `name` already exists and will hold the display value

RoleCreate:
- Remove `display_name: Option<String>` (line 24)

RoleUpdate:
- Remove `display_name: Option<String>` (line 33)

**Step 4: Update MarketingGroup** (`shared/src/models/marketing_group.rs`)

MarketingGroup:
- Remove `display_name: String` (line 13)

MarketingGroupCreate:
- Remove `display_name: String` (line 26)

MarketingGroupUpdate:
- Remove `display_name: Option<String>` (line 36)

MgDiscountRule:
- Remove `display_name: String` (line 49)
- Change `receipt_name: String` → `receipt_name: Option<String>` (line 50)

MgDiscountRuleCreate:
- Remove `display_name: String` (line 64)
- Change `receipt_name: String` → `receipt_name: Option<String>` (line 65)

MgDiscountRuleUpdate:
- Remove `display_name: Option<String>` (line 76)

StampActivity:
- Remove `display_name: String` (line 36)

StampActivityCreate:
- Remove `display_name: String` (line 51)

StampActivityUpdate:
- Remove `display_name: Option<String>` (line 65)

**Step 5: Run cargo check**

Run: `cargo check -p shared`
Expected: Compilation errors in dependent crates (expected at this stage)

**Step 6: Commit**

```bash
git add shared/src/models/
git commit -m "feat(shared): remove display_name from all models, receipt_name optional"
```

---

### Task 4: Shared Types — Order Types (AppliedRule, AppliedMgRule, UserInfo)

**Files:**
- Modify: `shared/src/order/applied_rule.rs`
- Modify: `shared/src/order/applied_mg_rule.rs`
- Modify: `shared/src/client.rs` (UserInfo line 31)

**Step 1: Update AppliedRule** (`shared/src/order/applied_rule.rs`)

- Remove `display_name: String` (line 12)
- Change `receipt_name: String` → `receipt_name: Option<String>` (line 13)
- Update `from_rule()` method (line 44): remove display_name, clone receipt_name as Option
- Update all test data to remove display_name and use Option receipt_name

**Step 2: Update AppliedMgRule** (`shared/src/order/applied_mg_rule.rs`)

- Remove `display_name: String` (line 11)
- Change `receipt_name: String` → `receipt_name: Option<String>` (line 12)

**Step 3: Update UserInfo** (`shared/src/client.rs`)

- Rename `display_name: String` → `name: String` (line 31)

**Step 4: Run cargo check**

Run: `cargo check -p shared`
Expected: PASS for shared, errors in dependents

**Step 5: Commit**

```bash
git add shared/src/order/ shared/src/client.rs
git commit -m "feat(shared): update AppliedRule, AppliedMgRule, UserInfo"
```

---

### Task 5: Edge Server — Auth (JWT Claims)

**Files:**
- Modify: `edge-server/src/auth/jwt.rs` (Claims struct line 71, all references)

**Step 1: Update JWT Claims struct**

In Claims struct (~line 71): rename `display_name` → `name`
In all methods that build Claims: update field name
In UserClaimsData struct (~line 340): rename `display_name` → `name`
In all tests: update field names

**Step 2: Update login handler that constructs Claims**

Search for where Claims is constructed (likely in auth handler when building JWT from Employee data). Update `employee.display_name` → `employee.name`.

**Step 3: Update middleware that extracts current_user**

All `current_user.display_name` references in API handlers become `current_user.name`. There are ~20+ occurrences across:
- `edge-server/src/api/employees/handler.rs` (lines 90, 129, 159, 167)
- `edge-server/src/api/price_rules/handler.rs` (lines 174, 224, 264)
- `edge-server/src/api/role/handler.rs` (lines 129, 181, 227, 283, 303)
- `edge-server/src/api/marketing_groups/handler.rs` (lines 200, 240, 281, 324, 376, 413, 463, 499, 536)

Use find-and-replace: `current_user.display_name` → `current_user.name` across all handler files.

**Step 4: Run cargo check**

Run: `cargo check -p edge-server`
Expected: More errors in repository layer (next task)

**Step 5: Commit**

```bash
git add edge-server/src/auth/
git commit -m "feat(edge): rename display_name to name in JWT claims"
```

---

### Task 6: Edge Server — Repository Layer

**Files:**
- Modify: `edge-server/src/db/repository/employee.rs`
- Modify: `edge-server/src/db/repository/role.rs`
- Modify: `edge-server/src/db/repository/price_rule.rs`
- Modify: `edge-server/src/db/repository/marketing_group.rs`
- Modify: `edge-server/src/db/repository/member.rs`
- Modify: `edge-server/src/db/repository/stamp.rs` (test data)

**Step 1: Update employee.rs**

All SQL queries: `display_name` → `name`
- SELECT queries (lines 47, 56, 65, 75, 89): column name change
- INSERT (line 109): column name change
- UPDATE (line 154): column name change
- Rust code (line 104): `data.display_name` → `data.name`
- Bind (lines 114, 156): `display_name` → `name`

**Step 2: Update role.rs**

Remove `display_name` from all SELECT queries (lines 10, 19, 28, 38)
Remove display_name from INSERT (line 53) and UPDATE (line 87)
Remove the default logic (line 47): `let display_name = data.display_name.unwrap_or_else(...)` — no longer needed
Remove `.bind(&display_name)` / `.bind(data.display_name)` lines

**Step 3: Update price_rule.rs**

Remove `display_name` from all SELECT queries (lines 9, 24, 35, 45, 55)
Remove from INSERT (line 81) and UPDATE (line 120)
Remove `.bind(&data.display_name)` (line 85)
Handle `receipt_name` as Option in queries (use NULL)

**Step 4: Update marketing_group.rs**

Remove `display_name` from all queries for marketing_group, mg_discount_rule, and stamp_activity
This is the largest file — ~15 query changes

**Step 5: Update member.rs**

Line 7: `mg.display_name as marketing_group_name` → `mg.name as marketing_group_name`

**Step 6: Update stamp.rs test data**

Update seed DDL and INSERT statements to remove display_name columns

**Step 7: Run cargo check**

Run: `cargo check -p edge-server`
Expected: Errors in orders/pricing modules (next task)

**Step 8: Commit**

```bash
git add edge-server/src/db/repository/
git commit -m "feat(edge): update all repository queries for name cleanup"
```

---

### Task 7: Edge Server — Orders & Pricing

**Files:**
- Modify: `edge-server/src/orders/manager/mod.rs` (lines 69, 464, 486, 563, 761, 1185)
- Modify: `edge-server/src/pricing/mg_calculator.rs` (lines 70, 117)
- Modify: `edge-server/src/pricing/calculator.rs` (line 169 — receipt_name is now Option)
- Modify: `edge-server/src/pricing/item_calculator.rs` (test data)
- Modify: `edge-server/src/pricing/order_calculator.rs` (test data)
- Modify: `edge-server/src/pricing/matcher.rs` (test data)
- Modify: `edge-server/src/order_money/tests.rs` (test data)
- Modify: `edge-server/src/orders/storage.rs` (test data)
- Modify: `edge-server/src/orders/manager/tests/mod.rs` (test data)

**Step 1: Update manager/mod.rs**

- Line 69: `mg_display_name: String` → `mg_name: String`
- Line 464: `mg_display_name: mg.display_name` → `mg_name: mg.name`
- Line 761: `marketing_group_name: lm.mg_display_name` → `marketing_group_name: lm.mg_name`
- Line 486, 563: queries referencing `display_name` → `name`
- Line 1185: `stamp_activity_name: activity.display_name.clone()` → `activity.name.clone()`

**Step 2: Update mg_calculator.rs**

- Line 70: Remove `display_name: rule.display_name.clone()` from AppliedMgRule construction
- Handle `receipt_name` as Option
- Line 117: Update test data

**Step 3: Update calculator.rs**

- Line 169: `applied_rules.push(rule.receipt_name.clone())` → `applied_rules.push(rule.receipt_name.clone().unwrap_or_else(|| rule.name.clone()))`

**Step 4: Update all test data**

In every test file that constructs PriceRule or AppliedRule:
- Remove `display_name` field
- Change `receipt_name` from `String` to `Option<String>` (`Some("...")`)

Test files to update:
- `edge-server/src/pricing/calculator.rs` tests
- `edge-server/src/pricing/item_calculator.rs` tests
- `edge-server/src/pricing/order_calculator.rs` tests
- `edge-server/src/pricing/matcher.rs` tests
- `edge-server/src/order_money/tests.rs`
- `edge-server/src/orders/storage.rs` tests
- `edge-server/src/orders/manager/tests/mod.rs`

**Step 5: Run cargo check + tests**

Run: `cargo check -p edge-server && cargo test -p edge-server --lib`
Expected: All pass

**Step 6: Commit**

```bash
git add edge-server/src/orders/ edge-server/src/pricing/ edge-server/src/order_money/
git commit -m "feat(edge): update orders and pricing for name cleanup"
```

---

### Task 8: Edge Server — API Handlers

**Files:**
- Modify: `edge-server/src/api/employees/handler.rs`
- Modify: `edge-server/src/api/price_rules/handler.rs`
- Modify: `edge-server/src/api/role/handler.rs`
- Modify: `edge-server/src/api/marketing_groups/handler.rs`

**Step 1: Update validation calls**

Replace all `validate_*_text(&payload.display_name, "display_name", ...)` with:
- For entities that removed display_name entirely: delete the validation line
- For Employee: change to `validate_optional_text(&payload.name, "name", ...)`

**Step 2: Update field references**

- price_rules/handler.rs: remove display_name validation, update receipt_name handling for Option
- role/handler.rs: remove display_name references
- marketing_groups/handler.rs: remove display_name validation for all sub-entities

**Step 3: Run cargo check**

Run: `cargo check -p edge-server`
Expected: PASS (or remaining errors to fix)

**Step 4: Commit**

```bash
git add edge-server/src/api/
git commit -m "feat(edge): update API handlers for name cleanup"
```

---

### Task 9: Edge Server — Cloud Ops & Remaining

**Files:**
- Modify: `edge-server/src/cloud/ops/resource.rs` (PriceRule/Employee sync handlers)
- Modify: `edge-server/src/cloud/ops/catalog.rs` (if references display_name)
- Modify: Any remaining files with `display_name` references

**Step 1: Search and fix remaining references**

Run: `grep -rn "display_name" edge-server/src/` to find any remaining references.
Fix each one according to the entity's change pattern.

**Step 2: Run full check + tests**

Run: `cargo check -p edge-server && cargo test -p edge-server --lib`
Expected: All pass, zero warnings about display_name

**Step 3: Commit**

```bash
git add edge-server/src/
git commit -m "feat(edge): fix remaining display_name references"
```

---

### Task 10: Crab Cloud — Repository Layer

**Files:**
- Modify: `crab-cloud/src/db/store/price_rule.rs`
- Modify: `crab-cloud/src/db/store/employee.rs`
- Modify: Any other store modules with display_name

**Step 1: Update price_rule.rs**

- PriceRuleFromDb struct (line 83): remove `display_name: String`
- All UPSERT/SELECT/INSERT queries: remove `display_name` column
- Handle `receipt_name` as Option (nullable in queries)
- Conversion methods: remove display_name mapping

**Step 2: Update employee.rs**

- EmployeeFromDb struct (line 57): `display_name` → `name`
- All queries: `display_name` → `name`
- Fallback logic (line 98): `data.display_name` → `data.name`

**Step 3: Search for other cloud files**

Run: `grep -rn "display_name" crab-cloud/src/` and fix all remaining references.

**Step 4: Run cargo check**

Run: `cargo check -p crab-cloud`
Expected: PASS

**Step 5: Commit**

```bash
git add crab-cloud/src/
git commit -m "feat(cloud): update repository queries for name cleanup"
```

---

### Task 11: Frontend Types — red_coral

**Files:**
- Modify: `red_coral/src/core/domain/types/api/models.ts`
- Modify: `red_coral/src/core/domain/types/orderEvent.ts` (AppliedRule, AppliedMgRule)

**Step 1: Update models.ts**

For each interface, apply the entity's change pattern:

PriceRule/PriceRuleCreate/PriceRuleUpdate:
- Remove `display_name` field
- Change `receipt_name: string` → `receipt_name: string | null` (or `receipt_name?: string`)

Employee/EmployeeCreate/EmployeeUpdate:
- Rename `display_name` → `name`

Role/RoleCreate/RoleUpdate:
- Remove `display_name`

MarketingGroup/MarketingGroupCreate/MarketingGroupUpdate:
- Remove `display_name`

MgDiscountRule/MgDiscountRuleCreate/MgDiscountRuleUpdate:
- Remove `display_name`
- `receipt_name` → optional

StampActivity/StampActivityCreate/StampActivityUpdate:
- Remove `display_name`

AppliedMgRule:
- Remove `display_name`
- `receipt_name` → optional

User:
- Rename `display_name` → `name`

**Step 2: Update orderEvent.ts**

AppliedRule interface:
- Remove `display_name`
- `receipt_name` → `receipt_name: string | null`

**Step 3: Run type check**

Run: `cd red_coral && npx tsc --noEmit`
Expected: Errors in component files (fixed in next task)

**Step 4: Commit**

```bash
git add red_coral/src/core/domain/types/
git commit -m "feat(red_coral): update TypeScript types for name cleanup"
```

---

### Task 12: Frontend Components — red_coral

**Files:**
- Modify: `red_coral/src/features/price-rule/` (all files referencing display_name)
- Modify: `red_coral/src/features/user/UserFormModal.tsx`
- Modify: `red_coral/src/features/user/UserManagement.tsx`
- Modify: `red_coral/src/features/role/RolePermissionsEditor.tsx`
- Modify: `red_coral/src/features/marketing-group/` (all files)
- Modify: `red_coral/src/presentation/components/cart/CartItem.tsx`
- Modify: `red_coral/src/core/services/order/receiptBuilder.ts`
- Modify: `red_coral/src/screens/Login/index.tsx`
- Modify: `red_coral/src/screens/Checkout/CompItemMode.tsx`

**Step 1: Update receipt fallback logic**

`receiptBuilder.ts` lines 73, 238:
```typescript
// Before: entry.rule.receipt_name || entry.rule.display_name || entry.rule.name
// After:  entry.rule.receipt_name || entry.rule.name
name: entry.rule.receipt_name || entry.rule.name,
```

`CartItem.tsx` line 108:
```typescript
// Before: rule.receipt_name || rule.display_name
// After:  rule.receipt_name || rule.name
```

**Step 2: Update PriceRule forms**

`Step5Naming.tsx`:
- Remove `display_name` input field (lines 98-110)
- The `name` field becomes the primary display name input (was "Nombre interno", now "Nombre")
- `receipt_name` auto-fill from `name` instead of `display_name`
- `receipt_name` becomes optional (can be cleared)

`RuleDetailPanel.tsx`, `RuleListPanel.tsx`:
- `.display_name` → `.name`
- Remove any `rule.name` that was showing the internal name

`useRuleEditor.ts`:
- Remove `displayName` from editor state

**Step 3: Update Employee/User forms**

`UserFormModal.tsx`:
- `formData.displayName` → `formData.name`
- `editingUser.display_name` → `editingUser.name`
- Payload: `display_name: formData.displayName` → `name: formData.name`

`UserManagement.tsx`:
- `u.display_name` → `u.name`
- `user.display_name || user.username` → `user.name || user.username`

**Step 4: Update Role forms**

`RolePermissionsEditor.tsx`:
- `newRoleDisplayName` → `newRoleName`
- Remove display_name from create/update payloads

**Step 5: Update Marketing Group forms**

All files in `features/marketing-group/`:
- Remove display_name fields from forms
- Remove display_name from payloads
- `receipt_name` becomes optional

**Step 6: Update auth/login**

`Login/index.tsx` line 88:
- `display_name: userInfo.display_name` → `name: userInfo.name`

`CompItemMode.tsx`:
- `user.display_name` → `user.name`

**Step 7: Run type check**

Run: `cd red_coral && npx tsc --noEmit`
Expected: PASS

**Step 8: Commit**

```bash
git add red_coral/src/
git commit -m "feat(red_coral): update all components for name cleanup"
```

---

### Task 13: Frontend — crab-console

**Files:**
- Modify: `crab-console/src/core/types/store.ts`
- Modify: `crab-console/src/features/price-rule/PriceRuleManagement.tsx`
- Modify: `crab-console/src/features/price-rule/PriceRuleWizard.tsx`
- Modify: `crab-console/src/features/employee/EmployeeManagement.tsx`

**Step 1: Update store.ts types**

Same changes as red_coral models.ts:
- PriceRule/Employee/Role: remove or rename display_name
- receipt_name → optional where applicable

**Step 2: Update PriceRuleManagement.tsx**

- Search filter (line 111): `r.display_name` → `r.name`
- Edit load (line 136): remove `setFormDisplayName`
- Update payload (line 176): remove `display_name`
- Form field (lines 242-244): remove display_name input, use name as primary

**Step 3: Update PriceRuleWizard.tsx**

- Payload (line 74): remove `display_name`
- Form field (lines 106-108): remove display_name field

**Step 4: Update EmployeeManagement.tsx**

- All `e.display_name` → `e.name`
- All `emp.display_name` → `emp.name`
- Form state and payload: `displayName` → `name`

**Step 5: Run type check**

Run: `cd crab-console && npx tsc --noEmit`
Expected: PASS

**Step 6: Commit**

```bash
git add crab-console/src/
git commit -m "feat(console): update all components for name cleanup"
```

---

### Task 14: Full Workspace Verification

**Step 1: Run full Rust check**

Run: `cargo check --workspace`
Expected: PASS

**Step 2: Run clippy**

Run: `cargo clippy --workspace`
Expected: PASS (zero warnings related to this change)

**Step 3: Run tests**

Run: `cargo test --workspace --lib`
Expected: All tests pass

**Step 4: Run frontend type checks**

Run: `cd red_coral && npx tsc --noEmit`
Run: `cd crab-console && npx tsc --noEmit`
Expected: Both pass

**Step 5: Final grep — ensure no stale references**

Run: `grep -rn "display_name" shared/src/ edge-server/src/ crab-cloud/src/ red_coral/src/ crab-console/src/ --include="*.rs" --include="*.ts" --include="*.tsx"`
Expected: Zero matches (except possibly in migrations/comments)

**Step 6: Prepare sqlx offline metadata**

Run: `cargo sqlx prepare --workspace`

---

### Task 15: Update CLAUDE.md Files

**Files:**
- Modify: `shared/CLAUDE.md`
- Modify: `edge-server/CLAUDE.md`
- Modify: `crab-cloud/CLAUDE.md` (if it references display_name)
- Modify: `CLAUDE.md` (root, if needed)
- Modify: `red_coral/CLAUDE.md` (if it references display_name)

**Step 1: Update shared/CLAUDE.md**

In the module structure section, update any references to model field names.
If there's documentation about the naming convention, update to reflect the new `name` + `receipt_name?` pattern.

**Step 2: Update edge-server/CLAUDE.md**

Update any field references in the RBAC or model documentation sections.

**Step 3: Update other CLAUDE.md files as needed**

Search each CLAUDE.md for `display_name` references and update.

**Step 4: Commit**

```bash
git add */CLAUDE.md CLAUDE.md
git commit -m "docs: update CLAUDE.md files for name field cleanup"
```

---

### Task 16: Final Commit

**Step 1: Verify clean git state**

Run: `git status`
Expected: No unstaged changes related to this feature

**Step 2: Optional — squash or tag**

If the feature is complete, consider a summary commit or tag for reference.
