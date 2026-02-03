//! 审计日志 JSON diff 计算
//!
//! 通过比较更新前后的 JSON 值，自动生成变更差异。
//! 支持嵌套对象和数组的递归比较。
//! 浮点数使用容差比较避免精度问题。

use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;

/// 浮点数比较容差 (用于处理序列化/反序列化精度损失)
const FLOAT_EPSILON: f64 = 1e-9;

/// 递归比较两个 JSON 值是否相等（浮点数使用容差比较）
fn values_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::String(a), Value::String(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => {
            // 浮点数容差比较
            match (a.as_f64(), b.as_f64()) {
                (Some(fa), Some(fb)) => (fa - fb).abs() < FLOAT_EPSILON,
                _ => a == b,
            }
        }
        (Value::Array(a), Value::Array(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter().zip(b.iter()).all(|(va, vb)| values_equal(va, vb))
        }
        (Value::Object(a), Value::Object(b)) => {
            if a.len() != b.len() {
                return false;
            }
            a.iter().all(|(key, va)| {
                b.get(key).map_or(false, |vb| values_equal(va, vb))
            })
        }
        _ => false,
    }
}

/// 字段变更记录
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FieldChange {
    /// 字段名
    pub field: String,
    /// 变更前的值
    pub from: Value,
    /// 变更后的值
    pub to: Value,
}

/// 审计快照配置
#[derive(Debug, Clone)]
pub struct AuditConfig {
    /// 要排除的字段（如 "id", "hash_pass"）
    pub exclude_fields: &'static [&'static str],
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            exclude_fields: &["id"],
        }
    }
}

// ============================================================================
// 资源配置
// ============================================================================

/// 获取资源的审计配置
pub fn get_config(resource_type: &str) -> AuditConfig {
    match resource_type {
        "employee" => AuditConfig {
            exclude_fields: &["id", "hash_pass", "is_system"],
        },
        "role" => AuditConfig {
            exclude_fields: &["id", "is_system"],
        },
        "product" => AuditConfig {
            exclude_fields: &["id"],
        },
        "category" => AuditConfig {
            exclude_fields: &["id"],
        },
        "tag" => AuditConfig {
            exclude_fields: &["id", "is_system"],
        },
        "attribute" => AuditConfig {
            exclude_fields: &["id"],
        },
        "zone" => AuditConfig {
            exclude_fields: &["id"],
        },
        "dining_table" => AuditConfig {
            exclude_fields: &["id"],
        },
        "price_rule" => AuditConfig {
            exclude_fields: &["id", "created_by", "created_at"],
        },
        "print_destination" => AuditConfig {
            exclude_fields: &["id"],
        },
        "label_template" => AuditConfig {
            exclude_fields: &["id"],
        },
        "shift" => AuditConfig {
            exclude_fields: &["id"],
        },
        _ => AuditConfig::default(),
    }
}

// ============================================================================
// JSON Diff 算法
// ============================================================================

/// 计算两个 JSON 值的差异（递归）
fn diff_json_recursive(from: &Value, to: &Value, path: &str, changes: &mut Vec<FieldChange>) {
    match (from, to) {
        // 两者都是对象 → 递归比较字段
        (Value::Object(from_obj), Value::Object(to_obj)) => {
            let mut all_keys: HashSet<&String> = from_obj.keys().collect();
            all_keys.extend(to_obj.keys());

            for key in all_keys {
                let field_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };

                match (from_obj.get(key), to_obj.get(key)) {
                    (Some(f), Some(t)) => {
                        diff_json_recursive(f, t, &field_path, changes);
                    }
                    (Some(f), None) => {
                        changes.push(FieldChange {
                            field: field_path,
                            from: f.clone(),
                            to: Value::Null,
                        });
                    }
                    (None, Some(t)) => {
                        changes.push(FieldChange {
                            field: field_path,
                            from: Value::Null,
                            to: t.clone(),
                        });
                    }
                    (None, None) => unreachable!(),
                }
            }
        }

        // 两者都是数组 → 使用容差比较
        (Value::Array(_), Value::Array(_)) => {
            if !values_equal(from, to) {
                changes.push(FieldChange {
                    field: path.to_string(),
                    from: from.clone(),
                    to: to.clone(),
                });
            }
        }

        // 两者都是数字 → 使用容差比较 (处理浮点数精度问题)
        (Value::Number(from_num), Value::Number(to_num)) => {
            let are_equal = match (from_num.as_f64(), to_num.as_f64()) {
                (Some(f), Some(t)) => (f - t).abs() < FLOAT_EPSILON,
                _ => from_num == to_num, // 整数直接比较
            };
            if !are_equal {
                changes.push(FieldChange {
                    field: path.to_string(),
                    from: from.clone(),
                    to: to.clone(),
                });
            }
        }

        // 其他基本类型 → 直接比较值
        (f, t) => {
            if f != t {
                changes.push(FieldChange {
                    field: path.to_string(),
                    from: f.clone(),
                    to: t.clone(),
                });
            }
        }
    }
}

/// 过滤 JSON 对象中的敏感字段
fn filter_fields(value: &mut Value, exclude: &[&str]) {
    if let Value::Object(obj) = value {
        for field in exclude {
            obj.remove(*field);
        }
    }
}

// ============================================================================
// 公共 API
// ============================================================================

/// 创建 CREATE 操作的审计详情（快照）
///
/// # 参数
/// - `value`: 新创建的对象
/// - `resource_type`: 资源类型（用于获取配置）
///
/// # 返回
/// JSON 对象，包含过滤后的完整快照
pub fn create_snapshot<T: Serialize>(value: &T, resource_type: &str) -> Value {
    let config = get_config(resource_type);

    match serde_json::to_value(value) {
        Ok(mut json) => {
            filter_fields(&mut json, config.exclude_fields);
            json
        }
        Err(e) => {
            tracing::error!("Failed to serialize audit snapshot: {:?}", e);
            json!({"error": "serialization_failed"})
        }
    }
}

/// 创建 UPDATE 操作的审计详情（差异）
///
/// # 参数
/// - `from`: 更新前的对象
/// - `to`: 更新后的对象
/// - `resource_type`: 资源类型（用于获取配置）
///
/// # 返回
/// JSON 对象，格式：`{"changes": [{"field": "name", "from": "A", "to": "B"}, ...]}`
pub fn create_diff<T: Serialize>(from: &T, to: &T, resource_type: &str) -> Value {
    let config = get_config(resource_type);

    let from_json = match serde_json::to_value(from) {
        Ok(mut v) => {
            filter_fields(&mut v, config.exclude_fields);
            v
        }
        Err(e) => {
            tracing::error!("Failed to serialize 'from' for diff: {:?}", e);
            return json!({"error": "serialization_failed"});
        }
    };

    let to_json = match serde_json::to_value(to) {
        Ok(mut v) => {
            filter_fields(&mut v, config.exclude_fields);
            v
        }
        Err(e) => {
            tracing::error!("Failed to serialize 'to' for diff: {:?}", e);
            return json!({"error": "serialization_failed"});
        }
    };

    let mut changes = Vec::new();
    diff_json_recursive(&from_json, &to_json, "", &mut changes);

    if changes.is_empty() {
        json!({"changes": [], "note": "no_changes_detected"})
    } else {
        json!({"changes": changes})
    }
}

/// 创建 DELETE 操作的审计详情（标识符）
///
/// # 参数
/// - `name`: 被删除对象的名称/标识
///
/// # 返回
/// JSON 对象，格式：`{"name": "xxx"}`
pub fn create_delete_details(name: &str) -> Value {
    json!({"name": name})
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;

    #[derive(Serialize)]
    struct TestProduct {
        id: String,
        name: String,
        price: f64,
        is_active: bool,
    }

    #[derive(Serialize)]
    struct TestEmployee {
        id: String,
        username: String,
        hash_pass: String,
        role: String,
    }

    #[test]
    fn test_create_snapshot_filters_id() {
        let product = TestProduct {
            id: "product:123".to_string(),
            name: "Coffee".to_string(),
            price: 10.0,
            is_active: true,
        };

        let snapshot = create_snapshot(&product, "product");
        let obj = snapshot.as_object().unwrap();

        assert!(obj.contains_key("name"));
        assert!(obj.contains_key("price"));
        assert!(!obj.contains_key("id")); // id 被过滤
    }

    #[test]
    fn test_create_snapshot_filters_sensitive_fields() {
        let employee = TestEmployee {
            id: "employee:1".to_string(),
            username: "admin".to_string(),
            hash_pass: "$argon2$secret".to_string(),
            role: "role:admin".to_string(),
        };

        let snapshot = create_snapshot(&employee, "employee");
        let obj = snapshot.as_object().unwrap();

        assert!(obj.contains_key("username"));
        assert!(obj.contains_key("role"));
        assert!(!obj.contains_key("id"));
        assert!(!obj.contains_key("hash_pass")); // 密码被过滤
    }

    #[test]
    fn test_create_diff_simple_fields() {
        let from = TestProduct {
            id: "product:1".to_string(),
            name: "Coffee".to_string(),
            price: 10.0,
            is_active: true,
        };
        let to = TestProduct {
            id: "product:1".to_string(),
            name: "Espresso".to_string(),
            price: 12.0,
            is_active: true,
        };

        let diff = create_diff(&from, &to, "product");
        let changes = diff["changes"].as_array().unwrap();

        assert_eq!(changes.len(), 2);

        let fields: Vec<&str> = changes
            .iter()
            .map(|c| c["field"].as_str().unwrap())
            .collect();
        assert!(fields.contains(&"name"));
        assert!(fields.contains(&"price"));
    }

    #[test]
    fn test_create_diff_no_changes() {
        let product = TestProduct {
            id: "product:1".to_string(),
            name: "Coffee".to_string(),
            price: 10.0,
            is_active: true,
        };

        let diff = create_diff(&product, &product, "product");
        let changes = diff["changes"].as_array().unwrap();

        assert!(changes.is_empty());
        assert!(diff.get("note").is_some());
    }

    #[test]
    fn test_create_delete_details() {
        let details = create_delete_details("Coffee");
        assert_eq!(details["name"], "Coffee");
    }
}
