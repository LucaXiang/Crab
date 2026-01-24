//! Common serde helpers for handling null values from SurrealDB
//!
//! 支持两种 RecordId 格式的反序列化：
//! - 字符串格式 "table:id" (来自 API JSON)
//! - SurrealDB 原生格式 (来自数据库)

use serde::{Deserialize, Deserializer, Serializer};
use surrealdb::RecordId;

/// Deserialize bool that treats null as true
pub fn bool_true<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<bool>::deserialize(deserializer).map(|opt| opt.unwrap_or(true))
}

/// Deserialize bool that treats null as false
pub fn bool_false<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<bool>::deserialize(deserializer).map(|opt| opt.unwrap_or(false))
}

/// 内部辅助：同时支持字符串和原生 RecordId 格式
#[derive(Debug, Clone)]
struct FlexibleRecordId(RecordId);

impl<'de> Deserialize<'de> for FlexibleRecordId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::{self, Visitor};
        use std::fmt;

        struct FlexibleVisitor;

        impl<'de> Visitor<'de> for FlexibleVisitor {
            type Value = FlexibleRecordId;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string 'table:id' or RecordId")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .parse::<RecordId>()
                    .map(FlexibleRecordId)
                    .map_err(|_| de::Error::custom(format!("invalid RecordId: {}", value)))
            }

            fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
            where
                M: de::MapAccess<'de>,
            {
                // 委托给 RecordId 原生反序列化
                RecordId::deserialize(de::value::MapAccessDeserializer::new(map))
                    .map(FlexibleRecordId)
            }
        }

        deserializer.deserialize_any(FlexibleVisitor)
    }
}

/// RecordId serialization as "table:id" string
pub mod record_id {
    use super::*;

    pub fn serialize<S>(id: &RecordId, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        s.serialize_str(&id.to_string())
    }

    pub fn deserialize<'de, D>(d: D) -> Result<RecordId, D::Error>
    where
        D: Deserializer<'de>,
    {
        FlexibleRecordId::deserialize(d).map(|f| f.0)
    }
}

/// Option<RecordId> serialization
pub mod option_record_id {
    use super::*;

    pub fn serialize<S>(id: &Option<RecordId>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match id {
            Some(id) => s.serialize_some(&id.to_string()),
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<RecordId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<FlexibleRecordId>::deserialize(d).map(|opt| opt.map(|f| f.0))
    }
}

/// Vec<RecordId> serialization
pub mod vec_record_id {
    use super::*;

    pub fn serialize<S>(ids: &[RecordId], s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = s.serialize_seq(Some(ids.len()))?;
        for id in ids {
            seq.serialize_element(&id.to_string())?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Vec<RecordId>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Vec::<FlexibleRecordId>::deserialize(d)
            .map(|v| v.into_iter().map(|f| f.0).collect())
    }
}

/// Option<Vec<RecordId>> serialization
pub mod option_vec_record_id {
    use super::*;

    pub fn serialize<S>(ids: &Option<Vec<RecordId>>, s: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match ids {
            Some(ids) => {
                use serde::ser::SerializeSeq;
                let mut seq = s.serialize_seq(Some(ids.len()))?;
                for id in ids {
                    seq.serialize_element(&id.to_string())?;
                }
                seq.end()
            }
            None => s.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(d: D) -> Result<Option<Vec<RecordId>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<Vec<FlexibleRecordId>>::deserialize(d)
            .map(|opt| opt.map(|v| v.into_iter().map(|f| f.0).collect()))
    }
}
