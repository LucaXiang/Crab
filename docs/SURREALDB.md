# SurrealDB & SurrealQL 指南

本文档记录 SurrealDB 2.x 的关键特性、最佳实践，以及本项目的使用约定。

## 版本信息

- **项目使用版本**: SurrealDB 2.4 (`surrealdb = "2.4"`)
- **存储引擎**: RocksDB (`kv-rocksdb` feature)
- **模式**: 嵌入式数据库

## SurrealQL 核心语法

### DEFINE TABLE

```surql
DEFINE TABLE [ OVERWRITE | IF NOT EXISTS ] @name
    [ DROP ]
    [ SCHEMAFULL | SCHEMALESS ]
    [ TYPE [ ANY | NORMAL | RELATION [ IN | FROM ] @table [ OUT | TO ] @table [ ENFORCED ]]]
    [ AS SELECT ... ]
    [ CHANGEFEED @duration [ INCLUDE ORIGINAL ] ]
    [ PERMISSIONS ... ]
    [ COMMENT @string ]
```

**关键点**:
- `SCHEMAFULL` 表默认为 `TYPE NORMAL`
- `TYPE RELATION` 用于图边表
- `ENFORCED` (2.0+) 确保关联记录必须存在

### DEFINE FIELD

```surql
DEFINE FIELD [ OVERWRITE | IF NOT EXISTS ] @name ON [ TABLE ] @table
    [ TYPE @type ]
    [ REFERENCE [ ON DELETE CASCADE | REJECT | IGNORE | UNSET ] ]  -- 2.2+
    [ DEFAULT [ALWAYS] @expression ]
    [ READONLY ]
    [ VALUE @expression ]
    [ ASSERT @expression ]
    [ PERMISSIONS ... ]
```

**类型示例**:
```surql
-- 基本类型
TYPE string
TYPE int
TYPE bool
TYPE datetime
TYPE option<string>          -- 可选类型

-- 记录链接
TYPE record<category>        -- 单个记录引用
TYPE array<record<tag>>      -- 记录数组

-- 联合类型
TYPE record<product|category>

-- 嵌套对象
TYPE array<object>
```

### RELATE (图关系)

```surql
RELATE [ ONLY ] @from -> @table -> @to
    [ CONTENT @value | SET @field = @value ... ]
    [ RETURN NONE | BEFORE | AFTER | DIFF ]
    [ TIMEOUT @duration ]
```

**示例**:
```surql
-- 创建关系
RELATE product:1->has_attribute->attribute:spicy
    SET is_required = true, display_order = 0;

-- 查询关系 (图遍历)
SELECT ->has_attribute->attribute.* FROM product:1;

-- 双向查询
SELECT <-has_attribute<-product.* FROM attribute:spicy;
```

### UPSERT (2.0+)

```surql
UPSERT [ ONLY ] @targets
    [ CONTENT @value | MERGE @value | SET @field = @value ]
    [ WHERE @condition ]
    [ RETURN NONE | BEFORE | AFTER | DIFF ]
```

**行为**:
- 无 WHERE: 直接插入
- 有 WHERE: 匹配则更新，不匹配则插入

### 常用函数

```surql
-- 字符串
string::len($value)
string::lowercase($value)
string::trim($value)

-- 时间
time::now()
time::format($datetime, "%Y-%m-%d")

-- 数组
array::len($arr)
array::distinct($arr)
array::concat($arr1, $arr2)
array::remove($arr, $idx)

-- 类型转换
type::thing("table", $id)    -- 构造 Thing ID
```

## 项目约定

### 表命名

| 表类型 | 命名规则 | 示例 |
|--------|----------|------|
| 实体表 | 单数名词 | `product`, `category`, `order` |
| 关系表 | 动词或描述性 | `has_attribute`, `has_event` |

### ID 格式

SurrealDB 使用 `Thing` 类型: `"table:id"`

```rust
// Rust 中构造 Thing
use surrealdb::sql::Thing;

fn make_thing(table: &str, id: &str) -> Thing {
    Thing::from((table, id))
}

// 从完整 ID 提取纯 ID
fn strip_table_prefix(table: &str, id: &str) -> &str {
    id.strip_prefix(&format!("{}:", table)).unwrap_or(id)
}
```

### 关系表定义

```surql
-- 标准关系表模板
DEFINE TABLE OVERWRITE @name TYPE RELATION
    FROM @source_table
    TO @target_table
    SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 关系字段
DEFINE FIELD OVERWRITE @field ON @name TYPE @type
    DEFAULT @default
    PERMISSIONS FULL;

-- 唯一约束 (防止重复关系)
DEFINE INDEX OVERWRITE @name_unique ON @name FIELDS in, out UNIQUE;
```

### 嵌套对象字段

```surql
-- 数组字段
DEFINE FIELD OVERWRITE specs ON product TYPE array<object>
    DEFAULT []
    PERMISSIONS FULL;

-- 嵌套字段定义 (使用 *.*)
DEFINE FIELD OVERWRITE specs.*.name ON product TYPE string PERMISSIONS FULL;
DEFINE FIELD OVERWRITE specs.*.price ON product TYPE int DEFAULT 0 PERMISSIONS FULL;
```

### 查询模式

**基本 CRUD**:
```rust
// Select all
let items: Vec<T> = db.select(TABLE).await?;

// Select by ID
let item: Option<T> = db.select((TABLE, id)).await?;

// Create
let created: Option<T> = db.create(TABLE).content(data).await?;

// Update with MERGE
db.query("UPDATE $thing MERGE $data")
    .bind(("thing", thing))
    .bind(("data", data))
    .await?;

// Delete
db.query("DELETE $thing").bind(("thing", thing)).await?;
```

**图遍历**:
```rust
// 获取产品的所有属性
db.query("SELECT ->has_attribute->attribute.* FROM $prod")
    .bind(("prod", make_thing("product", id)))
    .await?;

// 带条件的图查询
db.query(r#"
    SELECT *, out.* as attr_data
    FROM has_attribute
    WHERE in = $prod AND out.is_active = true
    ORDER BY display_order
"#)
.bind(("prod", make_thing("product", id)))
.await?;
```

## 本项目的图关系

```
┌──────────┐    has_attribute    ┌───────────┐
│ product  │ ───────────────────────>│ attribute │
└──────────┘                         └───────────┘
      │
      │ (category field - record link)
      ▼
┌──────────┐    has_attribute    ┌───────────┐
│ category │ ───────────────────────>│ attribute │
└──────────┘                         └───────────┘

┌──────────┐      has_event          ┌─────────────┐
│  order   │ ───────────────────────>│ order_event │
└──────────┘                         └─────────────┘
```

### 删除时的边清理

**必须手动清理的关系**:
```rust
// 删除 Product 前
db.query("DELETE has_attribute WHERE in = $product")
    .bind(("product", thing))
    .await?;

// 删除 Category 前
db.query("DELETE has_attribute WHERE in = $category")
    .bind(("category", thing))
    .await?;

// 删除 Attribute 前 (已在 repository 中实现)
db.query("DELETE has_attribute WHERE out = $attr")
    .bind(("attr", thing))
    .await?;
```

**不需要清理的关系**:
- `has_event`: Order 禁止删除，无需清理

## SurrealDB 2.x 新特性

### ENFORCED 关系 (2.0+)

确保关联的记录必须存在:
```surql
DEFINE TABLE likes TYPE RELATION
    FROM user TO post
    ENFORCED;  -- 如果 user 或 post 不存在，RELATE 会失败
```

### REFERENCE 外键 (2.2+)

自动级联删除:
```surql
DEFINE FIELD hometown ON person TYPE record<city>
    REFERENCE ON DELETE CASCADE;  -- 删除 city 时自动删除关联 person
```

可选行为: `CASCADE` | `REJECT` | `IGNORE` | `UNSET`

### 递归图查询 (2.1+)

```surql
-- 查询 N 层深度
SELECT ->knows.{1..3}->person FROM person:start;

-- 无限深度 (谨慎使用)
SELECT ->knows.{..}->person FROM person:start;
```

## 性能建议

1. **使用索引**: 为常用查询字段创建索引
2. **FETCH 谨慎**: 只在需要时使用 `FETCH`，避免过度加载
3. **分页查询**: 大数据集使用 `LIMIT` 和 `START`
4. **避免 SELECT ***: 只查询需要的字段

## 参考资源

- [SurrealDB 官方文档](https://surrealdb.com/docs)
- [SurrealQL 语句参考](https://surrealdb.com/docs/surrealql/statements)
- [SurrealDB 2.0 发布说明](https://surrealdb.com/blog/challenge-accepted-announcing-surrealdb-2-0)
- [SurrealDB 2.2 外键约束](https://surrealdb.com/blog/surrealdb-2-2-benchmarking-graph-path-algorithms-and-foreign-key-constraints)
