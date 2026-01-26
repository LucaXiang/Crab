//! Image Reference Repository
//!
//! 管理图片引用计数，支持同步引用和查找孤儿图片

use super::{BaseRepository, RepoResult};
use crate::db::models::{ImageRef, ImageRefEntityType};
use surrealdb::engine::local::Db;
use surrealdb::Surreal;
use std::collections::HashSet;

#[allow(dead_code)]
const TABLE: &str = "image_ref";

#[derive(Clone)]
pub struct ImageRefRepository {
    base: BaseRepository,
}

impl ImageRefRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// 同步实体的图片引用
    ///
    /// 1. 获取实体现有的引用
    /// 2. 计算差异（新增/移除）
    /// 3. 批量创建新引用
    /// 4. 批量删除旧引用
    /// 5. 返回被移除引用的 hash 列表（供调用方检查是否成为孤儿）
    pub async fn sync_refs(
        &self,
        entity_type: ImageRefEntityType,
        entity_id: &str,
        current_hashes: HashSet<String>,
    ) -> RepoResult<Vec<String>> {
        let entity_type_str = entity_type.as_str().to_string();
        let entity_id_owned = entity_id.to_string();

        // 1. 获取现有引用
        let existing: Vec<ImageRef> = self
            .base
            .db()
            .query("SELECT * FROM image_ref WHERE entity_type = $entity_type AND entity_id = $entity_id")
            .bind(("entity_type", entity_type_str.clone()))
            .bind(("entity_id", entity_id_owned.clone()))
            .await?
            .take(0)?;

        let existing_hashes: HashSet<String> = existing.iter().map(|r| r.hash.clone()).collect();

        // 2. 计算差异
        let to_add: Vec<String> = current_hashes.difference(&existing_hashes).cloned().collect();
        let to_remove: Vec<String> = existing_hashes.difference(&current_hashes).cloned().collect();

        // 3. 批量创建新引用
        for hash in &to_add {
            self.base
                .db()
                .query(
                    "CREATE image_ref CONTENT {
                        hash: $hash,
                        entity_type: $entity_type,
                        entity_id: $entity_id,
                        created_at: time::now()
                    }",
                )
                .bind(("hash", hash.clone()))
                .bind(("entity_type", entity_type_str.clone()))
                .bind(("entity_id", entity_id_owned.clone()))
                .await?;
        }

        // 4. 批量删除旧引用
        if !to_remove.is_empty() {
            self.base
                .db()
                .query(
                    "DELETE image_ref WHERE entity_type = $entity_type AND entity_id = $entity_id AND hash IN $hashes",
                )
                .bind(("entity_type", entity_type_str))
                .bind(("entity_id", entity_id_owned))
                .bind(("hashes", to_remove.clone()))
                .await?;

            // 返回被移除的 hash
            return Ok(to_remove);
        }

        Ok(vec![])
    }

    /// 删除实体的所有图片引用
    ///
    /// 返回被删除的 hash 列表（供调用方检查是否成为孤儿）
    pub async fn delete_entity_refs(
        &self,
        entity_type: ImageRefEntityType,
        entity_id: &str,
    ) -> RepoResult<Vec<String>> {
        let entity_type_str = entity_type.as_str().to_string();
        let entity_id_owned = entity_id.to_string();

        // 先获取所有引用的 hash
        let refs: Vec<ImageRef> = self
            .base
            .db()
            .query("SELECT * FROM image_ref WHERE entity_type = $entity_type AND entity_id = $entity_id")
            .bind(("entity_type", entity_type_str.clone()))
            .bind(("entity_id", entity_id_owned.clone()))
            .await?
            .take(0)?;

        let hashes: Vec<String> = refs.into_iter().map(|r| r.hash).collect();

        // 删除所有引用
        self.base
            .db()
            .query("DELETE image_ref WHERE entity_type = $entity_type AND entity_id = $entity_id")
            .bind(("entity_type", entity_type_str))
            .bind(("entity_id", entity_id_owned))
            .await?;

        Ok(hashes)
    }

    /// 统计图片的引用数
    pub async fn count_refs(&self, hash: &str) -> RepoResult<i64> {
        let hash_owned = hash.to_string();
        let mut result = self
            .base
            .db()
            .query("SELECT count() FROM image_ref WHERE hash = $hash GROUP ALL")
            .bind(("hash", hash_owned))
            .await?;

        let count: Option<i64> = result.take((0, "count"))?;
        Ok(count.unwrap_or(0))
    }

    /// 找出没有引用的图片 hash（孤儿图片）
    ///
    /// 输入一组 hash，返回其中引用数为 0 的 hash
    pub async fn find_orphan_hashes(&self, hashes: &[String]) -> RepoResult<Vec<String>> {
        if hashes.is_empty() {
            return Ok(vec![]);
        }

        let mut orphans = Vec::new();

        for hash in hashes {
            let count = self.count_refs(hash).await?;
            if count == 0 {
                orphans.push(hash.clone());
            }
        }

        Ok(orphans)
    }

    /// 获取实体的所有图片引用
    pub async fn get_entity_refs(
        &self,
        entity_type: ImageRefEntityType,
        entity_id: &str,
    ) -> RepoResult<Vec<ImageRef>> {
        let entity_type_str = entity_type.as_str().to_string();
        let entity_id_owned = entity_id.to_string();

        let refs: Vec<ImageRef> = self
            .base
            .db()
            .query("SELECT * FROM image_ref WHERE entity_type = $entity_type AND entity_id = $entity_id")
            .bind(("entity_type", entity_type_str))
            .bind(("entity_id", entity_id_owned))
            .await?
            .take(0)?;

        Ok(refs)
    }
}
