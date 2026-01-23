//! Serde helpers for SurrealDB Thing type
//!
//! 序列化/反序列化 Thing，支持两种输入格式：
//! 1. 字符串格式 "table:id" (来自 API 请求)
//! 2. Thing 对象格式 { tb: "table", id: "xxx" } (来自 SurrealDB 查询结果)

use serde::de::VariantAccess;
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

/// Thing 的 Visitor，支持字符串和对象两种格式
struct ThingVisitor;

impl<'de> de::Visitor<'de> for ThingVisitor {
    type Value = Thing;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string like 'table:id' or a Thing object { tb, id }")
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

    /// 处理 SurrealDB 返回的 Thing 对象格式 { tb: "table", id: "xxx" }
    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let mut tb: Option<String> = None;
        let mut id: Option<String> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "tb" => tb = Some(map.next_value()?),
                "id" => {
                    // id 可能是字符串或 { String: "xxx" } 格式，使用 IdVisitor 处理
                    id = Some(map.next_value_seed(IdVisitorSeed)?);
                }
                _ => {
                    // 忽略未知字段
                    let _: de::IgnoredAny = map.next_value()?;
                }
            }
        }

        let tb = tb.ok_or_else(|| de::Error::missing_field("tb"))?;
        let id = id.ok_or_else(|| de::Error::missing_field("id"))?;

        Ok(Thing::from((tb, id)))
    }
}

/// DeserializeSeed 用于反序列化 Thing.id 字段
struct IdVisitorSeed;

impl<'de> de::DeserializeSeed<'de> for IdVisitorSeed {
    type Value = String;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(IdVisitor)
    }
}

/// Visitor 用于处理各种 id 格式
struct IdVisitor;

impl<'de> de::Visitor<'de> for IdVisitor {
    type Value = String;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string or an object like { String: \"xxx\" }")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v.to_string())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(v)
    }

    /// 处理 { String: "xxx" } 格式
    fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
    where
        M: de::MapAccess<'de>,
    {
        let mut value: Option<String> = None;

        while let Some(key) = map.next_key::<String>()? {
            match key.as_str() {
                "String" => value = Some(map.next_value()?),
                _ => {
                    let _: de::IgnoredAny = map.next_value()?;
                }
            }
        }

        value.ok_or_else(|| de::Error::missing_field("String"))
    }

    /// 处理 SurrealDB 的 enum 变体（如 Id::String）
    fn visit_enum<A>(self, data: A) -> Result<Self::Value, A::Error>
    where
        A: de::EnumAccess<'de>,
    {
        let (variant, accessor) = data.variant::<String>()?;
        match variant.as_str() {
            "String" => accessor.newtype_variant(),
            other => Err(de::Error::unknown_variant(other, &["String"])),
        }
    }
}

/// 反序列化 Thing，支持字符串和对象两种格式
pub fn deserialize<'de, D>(deserializer: D) -> Result<Thing, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_any(ThingVisitor)
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
            formatter.write_str("null, a string like 'table:id', or a Thing object { tb, id }")
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
            // 使用 deserialize_any 支持字符串和对象两种格式
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

        /// 处理 SurrealDB 返回的 Thing 对象格式
        fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
        where
            M: de::MapAccess<'de>,
        {
            ThingVisitor.visit_map(map).map(Some)
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Thing>, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_any(OptionThingVisitor)
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

    /// 包装类型用于反序列化单个 Thing 元素，支持字符串和对象两种格式
    pub(super) struct ThingWrapper(pub(super) Thing);

    impl<'de> Deserialize<'de> for ThingWrapper {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            // 使用 deserialize_any 支持字符串和对象两种格式
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
            let s = format!("{}:{}", thing.tb, thing.id.to_raw());
            seq.serialize_element(&s)?;
        }
        seq.end()
    }
}

/// Option<Vec<Thing>> 的序列化/反序列化
pub mod option_vec {
    use super::vec::ThingWrapper;
    use super::*;

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<Thing>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // 使用 ThingWrapper 支持字符串和对象两种格式
        Option::<Vec<ThingWrapper>>::deserialize(deserializer)?
            .map(|wrappers| Ok(wrappers.into_iter().map(|w| w.0).collect()))
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
