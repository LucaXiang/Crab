//! Serde helpers for SurrealDB Thing type
//!
//! 序列化/反序列化 Thing 为字符串格式 "table:id"

use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use std::fmt;
use surrealdb::sql::Thing;

/// 从字符串 "table:id" 解析为 Thing
fn parse_thing_from_string(s: &str) -> Thing {
    if let Some((tb, id)) = s.split_once(':') {
        Thing::from((tb.to_string(), id.to_string()))
    } else {
        // 没有冒号时，整个字符串作为 id，table 为空
        Thing::from(("".to_string(), s.to_string()))
    }
}

/// Thing 字符串格式的 Visitor
struct ThingVisitor;

impl<'de> de::Visitor<'de> for ThingVisitor {
    type Value = Thing;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string like 'table:id'")
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
}

/// 反序列化 Thing，从字符串格式 "table:id"
pub fn deserialize<'de, D>(deserializer: D) -> Result<Thing, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(ThingVisitor)
}

/// 序列化 Thing 为字符串格式 "table:id"
pub fn serialize<S>(thing: &Thing, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // 使用 tb:id.to_raw() 格式避免 SurrealDB 的特殊括号
    let s = format!("{}:{}", thing.tb, thing.id.to_raw());
    serializer.serialize_str(&s)
}

/// Option<Thing> 的序列化/反序列化
pub mod option {
    use super::*;

    struct OptionThingVisitor;

    impl<'de> de::Visitor<'de> for OptionThingVisitor {
        type Value = Option<Thing>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("null or a string like 'table:id'")
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
            deserializer.deserialize_str(ThingVisitor).map(Some)
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
            Some(t) => {
                let s = format!("{}:{}", t.tb, t.id.to_raw());
                serializer.serialize_some(&s)
            }
            None => serializer.serialize_none(),
        }
    }
}

/// Vec<Thing> 的序列化/反序列化
pub mod vec {
    use super::*;

    /// 包装类型用于反序列化单个 Thing 元素
    struct ThingWrapper(Thing);

    impl<'de> Deserialize<'de> for ThingWrapper {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            deserializer.deserialize_str(ThingVisitor).map(ThingWrapper)
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
            let s = format!("{}:{}", thing.tb, thing.id.to_raw());
            seq.serialize_element(&s)?;
        }
        seq.end()
    }
}

/// Option<Vec<Thing>> 的序列化/反序列化
pub mod option_vec {
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<Thing>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        Option::<Vec<String>>::deserialize(deserializer)?
            .map(|values| {
                Ok(values
                    .into_iter()
                    .map(|s| parse_thing_from_string(&s))
                    .collect())
            })
            .transpose()
    }

    pub fn serialize<S>(things: &Option<Vec<Thing>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match things {
            Some(vec) => {
                let strings: Vec<String> = vec
                    .iter()
                    .map(|t| format!("{}:{}", t.tb, t.id.to_raw()))
                    .collect();
                strings.serialize(serializer)
            }
            None => serializer.serialize_none(),
        }
    }
}
