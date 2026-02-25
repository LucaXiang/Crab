# TODO - 待办事项

## 双向同步 LWW 修复

**状态**: 部分完成（Phase 1 回弹防护已完成，Phase 2-4 未开始）

**计划文件**: `.claude/plans/proud-beaming-creek.md`

- [ ] Phase 2: Cloud 侧 LWW Guard — upsert SQL 加 `WHERE updated_at <= EXCLUDED.updated_at`
- [ ] Phase 3: Pending Ops 队列 — edge 离线时 Console 编辑入队，重连时回放
- [ ] Phase 4: Edge 侧 LWW Guard — RPC 消息携带 `changed_at`，edge 对比本地时间戳
