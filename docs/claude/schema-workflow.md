# Schema 变更工作流

修改数据库 schema 时按以下顺序执行:

1. `sqlx migrate add -r -s <desc> --source edge-server/migrations` — 创建迁移
2. 编写 up/down SQL
3. `sqlx db reset -y --source edge-server/migrations` — 重置并应用
4. 更新 Rust 模型 (`edge-server/src/db/models/`) + 共享类型 (`shared/`)
5. `cargo sqlx prepare --workspace` — 更新离线元数据
6. 更新 TypeScript 类型 (`red_coral/src/core/domain/types/`)
7. 验证: `cargo check --workspace && cd red_coral && npx tsc --noEmit`
