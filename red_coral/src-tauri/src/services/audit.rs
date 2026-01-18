//! 审计日志服务
//!
//! 用于记录所有关键操作的审计追踪，支持税务合规。

use sqlx::{Pool, Sqlite, query};
use serde_json::Value;
use chrono::Utc;

/// 审计日志严重级别
#[derive(Debug, Clone, PartialEq)]
pub enum AuditSeverity {
    Debug,
    Info,
    Warning,
    Error,
    Critical,
}

impl AuditSeverity {
    /// 转换为数据库存储的字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            AuditSeverity::Debug => "DEBUG",
            AuditSeverity::Info => "INFO",
            AuditSeverity::Warning => "WARNING",
            AuditSeverity::Error => "ERROR",
            AuditSeverity::Critical => "CRITICAL",
        }
    }
}

/// 审计日志结构
#[derive(Debug, Clone)]
pub struct AuditLog {
    pub uuid: String,
    pub timestamp: i64,
    pub category: String,       // SYSTEM | OPERATION | SECURITY | DATA | PAYMENT | PRINT
    pub event_type: String,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub entity_type: Option<String>,
    pub entity_id: Option<String>,
    pub entity_name: Option<String>,
    pub action: String,
    pub description: Option<String>,
    pub severity: AuditSeverity,
    pub metadata: Option<Value>,
    pub source: Option<String>,
    pub source_device: Option<String>,
    pub source_ip: Option<String>,
}

/// 审计上下文
#[derive(Debug, Clone, Default)]
pub struct AuditContext {
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub source: String,
    pub source_device: Option<String>,
    pub source_ip: Option<String>,
}

impl AuditContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_user(user_id: &str, username: &str) -> Self {
        Self {
            user_id: Some(user_id.to_string()),
            username: Some(username.to_string()),
            source: "backend".to_string(),
            source_device: None,
            source_ip: None,
        }
    }
}

/// 审计日志服务
pub struct AuditService;

impl AuditService {
    /// 记录审计事件
    pub async fn log(
        pool: &Pool<Sqlite>,
        category: &str,
        event_type: &str,
        action: &str,
        description: &str,
        severity: AuditSeverity,
        entity_type: Option<&str>,
        entity_id: Option<&str>,
        entity_name: Option<&str>,
        metadata: Option<Value>,
        ctx: &AuditContext,
    ) {
        let uuid = format!("audit_{}", uuid::Uuid::new_v4());
        let timestamp = Utc::now().timestamp();

        if let Err(e) = query!(
            r#"
            INSERT INTO audit_logs (
                uuid, timestamp, category, event_type, user_id, username,
                entity_type, entity_id, entity_name, action, description,
                severity, metadata_json, source, source_device, source_ip
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            uuid,
            timestamp,
            category,
            event_type,
            ctx.user_id,
            ctx.username,
            entity_type,
            entity_id,
            entity_name,
            action,
            description,
            severity.as_str(),
            metadata.map(|m| m.to_string()),
            ctx.source,
            ctx.source_device,
            ctx.source_ip,
        )
        .execute(pool)
        .await
        {
            log::error!("Failed to write audit log: {}", e);
        }
    }

    /// 记录订单创建
    pub async fn log_order_created(
        pool: &Pool<Sqlite>,
        order_id: i64,
        receipt_number: &str,
        total: i64,
        ctx: &AuditContext,
    ) {
        let metadata = serde_json::json!({
            "order_id": order_id,
            "receipt_number": receipt_number,
            "total": total,
        });

        Self::log(
            pool,
            "PAYMENT",
            "order_created",
            "Crear pedido",
            &format!("Nuevo pedido creado: {}", receipt_number),
            AuditSeverity::Info,
            Some("order"),
            Some(&order_id.to_string()),
            Some(receipt_number),
            Some(metadata),
            ctx,
        ).await;
    }

    /// 记录支付完成
    pub async fn log_payment_completed(
        pool: &Pool<Sqlite>,
        order_id: i64,
        amount: i64,
        method: &str,
        ctx: &AuditContext,
    ) {
        let metadata = serde_json::json!({
            "order_id": order_id,
            "amount": amount,
            "method": method,
        });

        Self::log(
            pool,
            "PAYMENT",
            "payment_completed",
            "Pago completado",
            &format!("Pago de {} completado", amount),
            AuditSeverity::Info,
            Some("payment"),
            Some(&order_id.to_string()),
            None,
            Some(metadata),
            ctx,
        ).await;
    }

    /// 记录软删除操作
    pub async fn log_soft_delete(
        pool: &Pool<Sqlite>,
        entity_type: &str,
        entity_id: &str,
        entity_name: &str,
        ctx: &AuditContext,
    ) {
        Self::log(
            pool,
            "DATA",
            "soft_delete",
            "Eliminar suavemente",
            &format!("{} {} eliminado suavemente", entity_type, entity_name),
            AuditSeverity::Info,
            Some(entity_type),
            Some(entity_id),
            Some(entity_name),
            None,
            ctx,
        ).await;
    }

    /// 记录用户登录
    pub async fn log_user_login(
        pool: &Pool<Sqlite>,
        user_id: &str,
        username: &str,
        success: bool,
        ctx: &AuditContext,
    ) {
        let severity = if success { AuditSeverity::Info } else { AuditSeverity::Warning };
        let action = if success { "Inicio de sesión" } else { "Fallo de inicio de sesión" };
        let description = if success {
            format!("Usuario {} inició sesión", username)
        } else {
            format!("Intento de inicio de sesión fallido para {}", username)
        };

        let metadata = serde_json::json!({
            "user_id": user_id,
            "username": username,
            "success": success,
        });

        Self::log(
            pool,
            "SECURITY",
            "user_login",
            action,
            &description,
            severity,
            Some("user"),
            Some(user_id),
            Some(username),
            Some(metadata),
            ctx,
        ).await;
    }

    /// 记录产品变更
    pub async fn log_product_change(
        pool: &Pool<Sqlite>,
        product_id: &str,
        product_name: &str,
        change_type: &str,
        changes: Option<Value>,
        ctx: &AuditContext,
    ) {
        Self::log(
            pool,
            "DATA",
            change_type,
            "Cambio de producto",
            &format!("Producto {} modificado: {}", product_name, change_type),
            AuditSeverity::Info,
            Some("product"),
            Some(product_id),
            Some(product_name),
            changes,
            ctx,
        ).await;
    }

    /// 获取审计日志
    pub async fn get_logs(
        pool: &Pool<Sqlite>,
        category: Option<&str>,
        start_time: Option<i64>,
        end_time: Option<i64>,
        limit: i64,
    ) -> Result<Vec<AuditLog>, sqlx::Error> {
        let mut conditions = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(cat) = category {
            conditions.push("category = ?");
            params.push(cat.to_string());
        }

        if let Some(start) = start_time {
            conditions.push("timestamp >= ?");
            params.push(start.to_string());
        }

        if let Some(end) = end_time {
            conditions.push("timestamp <= ?");
            params.push(end.to_string());
        }

        let where_clause = if conditions.is_empty() {
            "".to_string()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let sql = format!(
            "SELECT * FROM audit_logs{} ORDER BY timestamp DESC LIMIT {}",
            where_clause, limit
        );

        let rows = sqlx::query(&sql)
            .fetch_all(pool)
            .await?;

        let logs: Vec<AuditLog> = rows
            .into_iter()
            .map(|r| {
                let severity_str: String = r.get("severity");
                let severity = match severity_str.as_str() {
                    "DEBUG" => AuditSeverity::Debug,
                    "WARNING" => AuditSeverity::Warning,
                    "ERROR" => AuditSeverity::Error,
                    "CRITICAL" => AuditSeverity::Critical,
                    _ => AuditSeverity::Info,
                };

                let metadata: Option<String> = r.try_get("metadata_json").ok();
                let metadata_value = metadata
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok());

                AuditLog {
                    uuid: r.get("uuid"),
                    timestamp: r.get("timestamp"),
                    category: r.get("category"),
                    event_type: r.get("event_type"),
                    user_id: r.try_get("user_id").ok(),
                    username: r.try_get("username").ok(),
                    entity_type: r.try_get("entity_type").ok(),
                    entity_id: r.try_get("entity_id").ok(),
                    entity_name: r.try_get("entity_name").ok(),
                    action: r.get("action"),
                    description: r.try_get("description").ok(),
                    severity,
                    metadata: metadata_value,
                    source: r.try_get("source").ok(),
                    source_device: r.try_get("source_device").ok(),
                    source_ip: r.try_get("source_ip").ok(),
                }
            })
            .collect();

        Ok(logs)
    }
}
