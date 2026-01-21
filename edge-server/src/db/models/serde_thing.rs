//! Serde helpers for SurrealDB Thing type
//!
//! 支持从字符串格式 "table:id" 反序列化为 Thing
//! 同时兼容 SurrealDB 原生格式和 JSON 字符串格式

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use surrealdb::sql::Thing;
use std::fmt;

/// 从字符串 "table:id" 解析为 Thing
fn parse_thing_from_string(s: &str) -> Thing {
    if let Some((tb, id)) = s.split_once(':') {
        Thing::from((tb.to_string(), id.to_string()))
    } else {
        // 没有冒号时，整个字符串作为 id，table 为空
        Thing::from(("".to_string(), s.to_string()))
    }
}

/// 自定义 Visitor，支持 Thing 原生格式和字符串格式
struct ThingVisitor;

impl<'de> de::Visitor<'de> for ThingVisitor {
    type Value = Thing;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a Thing or a string like 'table:id'")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(parse_thing_from_string(v))
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(parse_thing_from_string(&v))
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        // 委托给 Thing 的默认反序列化
        Thing::deserialize(de::value::MapAccessDeserializer::new(map))
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        Thing::deserialize(deserializer)
    }
}

/// 反序列化 Thing，支持字符串格式 "table:id" 和 SurrealDB 原生格式
pub fn deserialize<'de, D>(deserializer: D) -> Result<Thing, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(ThingVisitor)
}

/// 序列化 Thing 为字符串格式
pub fn serialize<S>(thing: &Thing, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&thing.to_string())
}

/// Option<Thing> 的反序列化
pub mod option {
    use super::*;

    struct OptionThingVisitor;

    impl<'de> de::Visitor<'de> for OptionThingVisitor {
        type Value = Option<Thing>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("null, a Thing, or a string like 'table:id'")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(ThingVisitor).map(Some)
        }

        fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_empty() {
                Ok(None)
            } else {
                Ok(Some(parse_thing_from_string(v)))
            }
        }

        fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            if v.is_empty() {
                Ok(None)
            } else {
                Ok(Some(parse_thing_from_string(&v)))
            }
        }

        fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            Thing::deserialize(de::value::MapAccessDeserializer::new(map)).map(Some)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Thing>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_option(OptionThingVisitor)
    }

    pub fn serialize<S>(thing: &Option<Thing>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match thing {
            Some(t) => serializer.serialize_some(&t.to_string()),
            None => serializer.serialize_none(),
        }
    }
}

/// Vec<Thing> 的反序列化
pub mod vec {
    use super::*;

    /// 包装类型用于反序列化单个 Thing 元素
    struct ThingWrapper(Thing);

    impl<'de> Deserialize<'de> for ThingWrapper {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_any(ThingVisitor).map(ThingWrapper)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Thing>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wrappers: Vec<ThingWrapper> = Vec::deserialize(deserializer)?;
        Ok(wrappers.into_iter().map(|w| w.0).collect())
    }

    pub fn serialize<S>(things: &[Thing], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        use serde::ser::SerializeSeq;
        let mut seq = serializer.serialize_seq(Some(things.len()))?;
        for thing in things {
            seq.serialize_element(&thing.to_string())?;
        }
        seq.end()
    }
}

/// Option<Vec<Thing>> 的反序列化
pub mod option_vec {
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<Thing>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<Vec<serde_json::Value>>::deserialize(deserializer)?
            .map(|values| {
                values
                    .into_iter()
                    .map(|v| {
                        if let Some(s) = v.as_str() {
                            Ok(parse_thing_from_string(s))
                        } else {
                            serde_json::from_value::<Thing>(v)
                                .map_err(de::Error::custom)
                        }
                    })
                    .collect::<Result<Vec<Thing>, _>>()
            })
            .transpose()
    }

    pub fn serialize<S>(things: &Option<Vec<Thing>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match things {
            Some(vec) => {
                let strings: Vec<String> = vec.iter().map(|t| t.to_string()).collect();
                strings.serialize(serializer)
            }
            None => serializer.serialize_none(),
        }
    }
}
