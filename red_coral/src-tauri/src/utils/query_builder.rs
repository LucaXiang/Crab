use serde_json::Value;
use sqlx::{query::Query, Sqlite};

/// Query builder for constructing SQL queries with dynamic WHERE conditions
pub struct QueryBuilder {
    conditions: Vec<String>,
    bindings: Vec<QueryValue>,
}

#[derive(Clone)]
pub enum QueryValue {
    Text(String),
    Integer(i64),
    Float(f64),
}

impl QueryBuilder {
    pub fn new() -> Self {
        Self {
            conditions: Vec::new(),
            bindings: Vec::new(),
        }
    }

    /// Add a condition with bindings
    pub fn add_condition(&mut self, condition: &str) -> &mut Self {
        self.conditions.push(condition.to_string());
        self
    }

    /// Add a text binding
    pub fn bind_text(&mut self, value: String) -> &mut Self {
        self.bindings.push(QueryValue::Text(value));
        self
    }

    /// Add an integer binding
    pub fn bind_i64(&mut self, value: i64) -> &mut Self {
        self.bindings.push(QueryValue::Integer(value));
        self
    }

    /// Add a float binding
    pub fn bind_f64(&mut self, value: f64) -> &mut Self {
        self.bindings.push(QueryValue::Float(value));
        self
    }

    /// Add LIKE search condition for multiple fields
    pub fn add_search_condition(&mut self, fields: &[&str], search: &str) -> &mut Self {
        let field_conditions: Vec<String> = fields
            .iter()
            .map(|field| format!("{} LIKE ?", field))
            .collect();

        let condition = format!("({})", field_conditions.join(" OR "));
        self.conditions.push(condition);

        // Add binding for each field
        let search_pattern = format!("%{}%", search);
        for _ in fields {
            self.bindings.push(QueryValue::Text(search_pattern.clone()));
        }

        self
    }

    /// Add IN condition for status values
    pub fn add_in_condition(&mut self, field: &str, values: &[&str]) -> &mut Self {
        let placeholders: Vec<&str> = values.iter().map(|_| "?").collect();
        let condition = format!("{} IN ({})", field, placeholders.join(", "));
        self.conditions.push(condition);

        for val in values {
            self.bindings.push(QueryValue::Text(val.to_string()));
        }

        self
    }

    /// Build WHERE clause (empty if no conditions)
    pub fn build_where_clause(&self) -> String {
        if self.conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", self.conditions.join(" AND "))
        }
    }

    /// Apply bindings to a SQLx query
    pub fn apply_bindings<'a, 'b>(
        &'b self,
        mut query: Query<'a, Sqlite, <Sqlite as sqlx::Database>::Arguments<'a>>,
    ) -> Query<'a, Sqlite, <Sqlite as sqlx::Database>::Arguments<'a>>
    where
        'b: 'a,
    {
        for binding in &self.bindings {
            query = match binding {
                QueryValue::Text(s) => query.bind(s),
                QueryValue::Integer(i) => query.bind(*i),
                QueryValue::Float(f) => query.bind(*f),
            };
        }
        query
    }

    /// Apply bindings to a SQLx query_scalar
    pub fn apply_bindings_scalar<'a, 'b, O>(
        &'b self,
        mut query: sqlx::query::QueryScalar<'a, Sqlite, O, <Sqlite as sqlx::Database>::Arguments<'a>>,
    ) -> sqlx::query::QueryScalar<'a, Sqlite, O, <Sqlite as sqlx::Database>::Arguments<'a>>
    where
        O: Send + Unpin,
        'b: 'a,
    {
        for binding in &self.bindings {
            query = match binding {
                QueryValue::Text(s) => query.bind(s),
                QueryValue::Integer(i) => query.bind(*i),
                QueryValue::Float(f) => query.bind(*f),
            };
        }
        query
    }

    /// Convert JSON number to i64 or f64
    pub fn json_to_i64(value: &Value) -> Option<i64> {
        value.as_number().and_then(|n| n.as_i64())
    }

    pub fn json_to_f64(value: &Value) -> Option<f64> {
        value.as_number().and_then(|n| n.as_f64())
    }
}

impl Default for QueryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_where_clause() {
        let builder = QueryBuilder::new();
        assert_eq!(builder.build_where_clause(), "");
    }

    #[test]
    fn test_single_condition() {
        let mut builder = QueryBuilder::new();
        builder.add_condition("status = ?").bind_text("COMPLETED".to_string());
        assert_eq!(builder.build_where_clause(), " WHERE status = ?");
    }

    #[test]
    fn test_multiple_conditions() {
        let mut builder = QueryBuilder::new();
        builder
            .add_condition("status = ?")
            .bind_text("COMPLETED".to_string())
            .add_condition("total > ?")
            .bind_f64(100.0);
        assert_eq!(builder.build_where_clause(), " WHERE status = ? AND total > ?");
    }

    #[test]
    fn test_search_condition() {
        let mut builder = QueryBuilder::new();
        builder.add_search_condition(&["table_name", "receipt_number"], "test");
        assert_eq!(
            builder.build_where_clause(),
            " WHERE (table_name LIKE ? OR receipt_number LIKE ?)"
        );
    }

    #[test]
    fn test_in_condition() {
        let mut builder = QueryBuilder::new();
        builder.add_in_condition("status", &["COMPLETED", "VOID"]);
        assert_eq!(builder.build_where_clause(), " WHERE status IN (?, ?)");
    }
}
