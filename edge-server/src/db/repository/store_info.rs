//! Store Info Repository (Singleton)

use super::{BaseRepository, RepoError, RepoResult};
use crate::db::models::{StoreInfo, StoreInfoUpdate};
use surrealdb::engine::local::Db;
use surrealdb::{RecordId, Surreal};

const TABLE: &str = "store_info";
const SINGLETON_ID: &str = "main";

#[derive(Clone)]
pub struct StoreInfoRepository {
    base: BaseRepository,
}

impl StoreInfoRepository {
    pub fn new(db: Surreal<Db>) -> Self {
        Self {
            base: BaseRepository::new(db),
        }
    }

    /// Get or create the singleton store info
    pub async fn get_or_create(&self) -> RepoResult<StoreInfo> {
        // Try to get existing
        if let Some(info) = self.get().await? {
            return Ok(info);
        }

        // Create new singleton with defaults
        let info = StoreInfo::default();

        let created: Option<StoreInfo> = self
            .base
            .db()
            .create((TABLE, SINGLETON_ID))
            .content(info)
            .await?;
        created.ok_or_else(|| RepoError::Database("Failed to create store info".to_string()))
    }

    /// Get the singleton store info
    pub async fn get(&self) -> RepoResult<Option<StoreInfo>> {
        let info: Option<StoreInfo> = self.base.db().select((TABLE, SINGLETON_ID)).await?;
        Ok(info)
    }

    /// Update store info
    pub async fn update(&self, data: StoreInfoUpdate) -> RepoResult<StoreInfo> {
        // Ensure singleton exists
        self.get_or_create().await?;

        // Update timestamp first
        let singleton_id = RecordId::from_table_key(TABLE, SINGLETON_ID);
        let _ = self
            .base
            .db()
            .query("UPDATE $id SET updated_at = $now")
            .bind(("id", singleton_id.clone()))
            .bind(("now", shared::util::now_millis()))
            .await?;

        // Merge update data
        let updated: Option<StoreInfo> = self.base.db().update(singleton_id).merge(data).await?;
        updated.ok_or_else(|| RepoError::Database("Failed to update store info".to_string()))
    }
}
