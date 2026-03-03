# Chain Sync Integrity: 统一链序同步 + 断链重起

**日期**: 2026-03-02
**状态**: Draft
**问题级别**: Critical — Hash 链完整性被破坏
**范围**: 订单层（chain_entry），不含发票层（Invoice huella 链）

## 问题

### 现象
订单 001 因 `ao.order_id` SQL bug 导致 `build_order_detail_sync()` 失败，被标记为 `cloud_synced=1`（从未到达云端）。退款凭证 002 独立同步成功到达云端。云端 hash 链断裂：002 的 `prev_hash` 指向不存在的 001。

### 根因

两个架构缺陷叠加：

1. **独立同步而非链序同步** — `sync_archives_http()` 将 ORDER、CREDIT_NOTE、ANULACION 作为独立流程分别同步，不按 `chain_entry.id` 全局顺序。
2. **失败即标记已同步** — 构建失败的条目被 `mark_cloud_synced()` 标记为"已同步"以"防阻塞"（worker.rs:574-584），但数据从未到达云端。

## 设计方案：统一链序同步 + 断链重起

### 核心思路

1. **统一按 chain_entry.id 顺序同步** — 废弃独立的 order/credit_note/anulacion sync 函数，改为以 chain_entry 为驱动
2. **构建失败 = 插入 BREAK + 重起链** — 不阻塞、不假装已同步，显式记录断裂

### 范围

chain_entry 上的 4 种 entry_type 全部纳入统一同步：
- `ORDER` — 归档订单
- `CREDIT_NOTE` — 退款凭证
- `ANULACION` — 发票作废
- `UPGRADE` — 发票升级

**不含 Invoice** — Invoice 有独立的 huella 链（AEAT Verifactu 规范），属于发票层，保持独立同步。

### 1. chain_entry 表变更

```sql
ALTER TABLE chain_entry ADD COLUMN cloud_synced INTEGER NOT NULL DEFAULT 0;
CREATE INDEX idx_chain_entry_cloud_synced ON chain_entry(cloud_synced);
```

entry_type 新增 `'BREAK'` 值，BREAK 记录的字段含义：
- `entry_pk`: 失败的原始 chain_entry.id
- `prev_hash`: 断裂点的最后一个 hash
- `curr_hash`: `"CHAIN_BREAK"`
- `created_at`: 断链发生时间

### 2. 同步主循环重写

废弃:
- `sync_archived_orders_http()`
- `sync_credit_notes_http()`
- `sync_anulaciones_http()`

新建 `sync_chain_entries_http()`：

```rust
async fn sync_chain_entries_http(&mut self) -> Result<(), AppError> {
    let binding = self.get_binding().await?;

    loop {
        // 按 chain_entry.id 严格顺序取未同步条目
        let entries = chain_entry::list_unsynced(&self.state.pool, BATCH_SIZE).await?;
        if entries.is_empty() { break; }

        let mut items: Vec<CloudSyncItem> = Vec::with_capacity(entries.len());
        let mut synced_entry_ids: Vec<i64> = Vec::new();

        for entry in &entries {
            match entry.entry_type.as_str() {
                "ORDER" => {
                    match self.build_order_sync_item(entry).await {
                        Ok(item) => {
                            items.push(item);
                            synced_entry_ids.push(entry.id);
                        }
                        Err(e) => {
                            self.handle_chain_break(entry, &e.to_string()).await?;
                            synced_entry_ids.push(entry.id);
                        }
                    }
                }
                "CREDIT_NOTE" => {
                    match self.build_credit_note_sync_item(entry).await {
                        Ok(item) => {
                            items.push(item);
                            synced_entry_ids.push(entry.id);
                        }
                        Err(e) => {
                            self.handle_chain_break(entry, &e.to_string()).await?;
                            synced_entry_ids.push(entry.id);
                        }
                    }
                }
                "ANULACION" => {
                    match self.build_anulacion_sync_item(entry).await {
                        Ok(item) => {
                            items.push(item);
                            synced_entry_ids.push(entry.id);
                        }
                        Err(e) => {
                            self.handle_chain_break(entry, &e.to_string()).await?;
                            synced_entry_ids.push(entry.id);
                        }
                    }
                }
                "UPGRADE" => {
                    match self.build_upgrade_sync_item(entry).await {
                        Ok(item) => {
                            items.push(item);
                            synced_entry_ids.push(entry.id);
                        }
                        Err(e) => {
                            self.handle_chain_break(entry, &e.to_string()).await?;
                            synced_entry_ids.push(entry.id);
                        }
                    }
                }
                "BREAK" => {
                    items.push(self.build_break_sync_item(entry));
                    synced_entry_ids.push(entry.id);
                }
                _ => {
                    tracing::warn!(entry_type = %entry.entry_type, "Unknown chain entry type");
                }
            }
        }

        if !items.is_empty() {
            let batch = CloudSyncBatch { edge_id, items, sent_at };
            let response = self.cloud_service.push_batch(batch, &binding).await?;
            // 处理 response（duplicate key → 标记已同步，real error → break）
        }

        chain_entry::mark_synced(&self.state.pool, &synced_entry_ids).await?;

        if entries.len() < BATCH_SIZE as usize { break; }
    }
    Ok(())
}
```

### 3. 断链处理

```rust
async fn handle_chain_break(&self, failed_entry: &ChainEntry, reason: &str) -> Result<(), AppError> {
    let break_id = snowflake_id();
    sqlx::query(
        "INSERT INTO chain_entry (id, entry_type, entry_pk, prev_hash, curr_hash, created_at, cloud_synced) \
         VALUES (?1, 'BREAK', ?2, ?3, 'CHAIN_BREAK', ?4, 0)"
    )
    .bind(break_id)
    .bind(failed_entry.id)
    .bind(&failed_entry.curr_hash)
    .bind(now_millis())
    .execute(&self.state.pool)
    .await?;

    tracing::error!(
        chain_entry_id = failed_entry.id,
        entry_type = %failed_entry.entry_type,
        reason = reason,
        "CHAIN BREAK: inserting break marker"
    );

    Ok(())
}
```

### 4. 云端处理

**新增 SyncResource variant**: `ChainBreak`

云端收到 `ChainBreak` 时：
- 记录断链（哪个 entry 丢失）
- BREAK 后的第一个 entry 视为新链创世（跳过 prev_hash 连续性校验）

### 5. sync_archives_http 调整

```rust
async fn sync_archives_http(&mut self, trigger: &str) {
    // 订单层：chain_entry 统一链序同步（ORDER + CREDIT_NOTE + ANULACION + UPGRADE + BREAK）
    if let Err(e) = self.sync_chain_entries_http().await {
        tracing::warn!("{trigger}: chain entry sync failed: {e}");
    }
    // 发票层：Invoice 保持独立同步（huella 链）
    if let Err(e) = self.sync_invoices_http().await {
        tracing::warn!("{trigger}: invoice sync failed: {e}");
    }
}
```

### 6. 资源表 cloud_synced

同步驱动改为 chain_entry.cloud_synced。各资源表的 cloud_synced 同步更新（保持一致性）。

## 文件变更清单

| 文件 | 变更 |
|------|------|
| `edge-server/migrations/0001_initial.sql` | chain_entry 添加 cloud_synced 列 |
| `shared/src/cloud/sync.rs` | SyncResource 新增 ChainBreak variant |
| `edge-server/src/cloud/worker.rs` | 重写为 sync_chain_entries_http，废弃 3 个独立 sync 函数 |
| `edge-server/src/db/repository/chain_entry.rs` | 新文件：list_unsynced, mark_synced |
| `edge-server/src/db/repository/mod.rs` | 注册 chain_entry module |
| `crab-cloud/src/db/sync_store.rs` | 处理 ChainBreak + ANULACION + UPGRADE，调整 hash 验证 |

## 迁移策略

chain_entry.cloud_synced 初始化：
```sql
UPDATE chain_entry SET cloud_synced = 1
WHERE (entry_type = 'ORDER' AND entry_pk IN (SELECT id FROM archived_order WHERE cloud_synced = 1))
   OR (entry_type = 'CREDIT_NOTE' AND entry_pk IN (SELECT id FROM credit_note WHERE cloud_synced = 1))
   OR (entry_type = 'ANULACION' AND entry_pk IN (SELECT id FROM invoice_anulacion WHERE cloud_synced = 1));
```

对于"标记已同步但云端没有"的数据（如 001）：重新标记为未同步，新逻辑会触发 BREAK + 重起链。
