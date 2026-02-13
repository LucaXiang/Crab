# 测试规范

- **命名**: `test_<action>_<scenario>` (如 `test_add_items_with_discount_rule`)
- **运行**: `cargo test --workspace --lib` (只跑单元测试，不跑 doc tests)
- **组织**: 按职责拆分测试文件，单文件不超过 500 行 (参考 `orders/manager/tests/`)
- **断言**: 用 `assert_eq!` / `assert!(matches!(..))` 而非 `unwrap()` 后比较
- **金额**: 测试中的金额断言使用 `rust_decimal::dec!()` 宏
