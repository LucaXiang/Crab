// shared/src/models/sync.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 同步状态响应
///
/// 用于客户端重连时检查资源版本
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStatus {
    /// 服务器实例 epoch (启动时生成的 UUID)
    /// 用于检测服务器重启
    pub epoch: String,
    /// 各资源类型的当前版本
    pub versions: HashMap<String, u64>,
}
