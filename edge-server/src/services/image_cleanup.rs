//! Image Cleanup Service
//!
//! 负责清理孤儿图片文件

use std::path::PathBuf;
use tokio::fs;

/// 图片清理服务
#[derive(Clone)]
pub struct ImageCleanupService {
    /// 图片目录路径: {tenant}/server/images/
    images_dir: PathBuf,
}

impl ImageCleanupService {
    /// 创建新的清理服务
    ///
    /// `images_dir` 是图片存储目录的完整路径
    pub fn new(images_dir: PathBuf) -> Self {
        Self { images_dir }
    }

    /// 清理孤儿图片
    ///
    /// 输入一组 hash，删除对应的图片文件
    /// 返回成功删除的 hash 数量
    pub async fn cleanup_orphan_images(&self, orphan_hashes: &[String]) -> usize {
        let mut deleted_count = 0;

        for hash in orphan_hashes {
            let file_path = self.images_dir.join(format!("{}.jpg", hash));

            if file_path.exists() {
                match fs::remove_file(&file_path).await {
                    Ok(_) => {
                        deleted_count += 1;
                    }
                    Err(e) => {
                        tracing::warn!(hash = %hash, error = %e, "Failed to delete orphan image");
                    }
                }
            }
        }

        if deleted_count > 0 {
            tracing::info!(count = deleted_count, "Orphan images cleaned up");
        }

        deleted_count
    }

    /// 检查图片是否存在
    pub fn image_exists(&self, hash: &str) -> bool {
        let file_path = self.images_dir.join(format!("{}.jpg", hash));
        file_path.exists()
    }

    /// 获取图片文件路径
    pub fn get_image_path(&self, hash: &str) -> PathBuf {
        self.images_dir.join(format!("{}.jpg", hash))
    }
}
