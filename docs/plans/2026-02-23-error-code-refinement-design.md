# Error Code Refinement Design

Date: 2026-02-23

## Problem

Backend uses generic error codes (ValidationFailed=2, NotFound=3) in ~65 places where specific ErrorCodes already exist or should be defined. Users see "验证失败" or "资源不存在" instead of actionable messages like "分类下存在菜品，无法删除".

## Solution

### 1. RepoError::Business variant

Add `Business(ErrorCode, String)` to `RepoError` so repo/service layers can specify exact error codes at the source:

```rust
pub enum RepoError {
    NotFound(String),
    Duplicate(String),
    Database(String),
    Validation(String),          // generic field validation (code 2)
    Business(ErrorCode, String), // specific business rule (carries exact code)
}
```

### 2. New ErrorCodes (12 total)

| Code | Name | Description |
|------|------|-------------|
| 3020 | DeviceIdMismatch | Hardware ID doesn't match certificate |
| 3021 | CertificateMissingDeviceId | Certificate missing device_id extension |
| 6204 | ProductCategoryInvalid | Product cannot belong to virtual category |
| 6303 | AttributeInUse | Attribute bound to products/categories |
| 6304 | AttributeDuplicateBinding | Binding inherited from category |
| 6401 | TagNotFound | Tag not found |
| 6402 | TagInUse | Tag in use by products |
| 6511 | PrintDestinationNotFound | Print destination not found |
| 6512 | PrintDestinationInUse | Print destination referenced by categories |
| 7104 | TableHasOrders | Table has active orders |
| 8004 | EmployeeIsSystem | System employee cannot be modified |
| 8104 | RoleIsSystem | System role cannot be modified |

### 3. Fix 19 generic not_found → specific resource codes

Replace `AppError::not_found(msg)` with `AppError::with_message(ErrorCode::XxxNotFound, msg)` in all handlers.

### 4. Fix 11 business validation → RepoError::Business

Replace `RepoError::Validation(msg)` with `RepoError::Business(ErrorCode::Xxx, msg)` for all business rule checks.

### 5. Fix cert/tenant validation → specific codes

Replace `AppError::validation(msg)` with `AppError::with_message(ErrorCode::CertificateInvalid, msg)` etc.

### 6. Remove handler-layer string matching

Revert the `.map_err()` string-matching hacks in categories/handler.rs and zones/handler.rs.

### 7. Frontend i18n

Add zh-CN, en, es-ES translations for all 12 new codes + missing 3010, 3019.

## Not Changed

26 instances of generic field validation (empty, too long, non-finite, range) stay as ValidationFailed (2). These are standard form validation with field names in error message.
