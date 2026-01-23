//! Thing 转换辅助函数

/// 将 Thing 转换为 "table:id" 格式字符串
pub fn thing_to_string(thing: &surrealdb::sql::Thing) -> String {
    thing.to_string()
}

pub fn option_thing_to_string(thing: &Option<surrealdb::sql::Thing>) -> Option<String> {
    thing.as_ref().map(thing_to_string)
}

pub fn things_to_strings(things: &[surrealdb::sql::Thing]) -> Vec<String> {
    things.iter().map(thing_to_string).collect()
}
