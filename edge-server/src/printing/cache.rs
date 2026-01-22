//! Print configuration cache with fallback routing

use super::types::{CategoryPrintConfig, ProductPrintConfig};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 打印配置缓存
#[derive(Debug, Clone)]
pub struct PrintConfigCache {
    inner: Arc<RwLock<PrintConfigCacheInner>>,
}

#[derive(Debug, Default)]
struct PrintConfigCacheInner {
    products: HashMap<String, ProductPrintConfig>,
    categories: HashMap<String, CategoryPrintConfig>,
    /// 系统默认厨房打印机（最终回退）
    default_kitchen_printer: Option<String>,
    /// 系统默认标签打印机（最终回退）
    default_label_printer: Option<String>,
}

impl PrintConfigCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(PrintConfigCacheInner::default())),
        }
    }

    /// 厨房打印功能是否启用（系统级）
    pub async fn is_kitchen_print_enabled(&self) -> bool {
        let inner = self.inner.read().await;
        inner.default_kitchen_printer.is_some()
    }

    /// 标签打印功能是否启用（系统级）
    pub async fn is_label_print_enabled(&self) -> bool {
        let inner = self.inner.read().await;
        inner.default_label_printer.is_some()
    }

    /// 设置系统默认打印机
    pub async fn set_defaults(&self, kitchen: Option<String>, label: Option<String>) {
        let mut inner = self.inner.write().await;
        inner.default_kitchen_printer = kitchen;
        inner.default_label_printer = label;
    }

    /// 更新商品配置
    pub async fn update_product(&self, config: ProductPrintConfig) {
        let mut inner = self.inner.write().await;
        inner.products.insert(config.product_id.clone(), config);
    }

    /// 更新分类配置
    pub async fn update_category(&self, config: CategoryPrintConfig) {
        let mut inner = self.inner.write().await;
        inner.categories.insert(config.category_id.clone(), config);
    }

    /// 移除商品配置
    pub async fn remove_product(&self, product_id: &str) {
        let mut inner = self.inner.write().await;
        inner.products.remove(product_id);
    }

    /// 移除分类配置
    pub async fn remove_category(&self, category_id: &str) {
        let mut inner = self.inner.write().await;
        inner.categories.remove(category_id);
    }

    /// 获取商品配置
    pub async fn get_product(&self, product_id: &str) -> Option<ProductPrintConfig> {
        let inner = self.inner.read().await;
        inner.products.get(product_id).cloned()
    }

    /// 获取分类配置
    pub async fn get_category(&self, category_id: &str) -> Option<CategoryPrintConfig> {
        let inner = self.inner.read().await;
        inner.categories.get(category_id).cloned()
    }

    /// 判断商品是否启用厨房打印 (tri-state: -1=继承, 0=禁用, 1=启用)
    pub async fn is_product_kitchen_enabled(&self, product_id: &str) -> bool {
        let inner = self.inner.read().await;

        if let Some(product) = inner.products.get(product_id) {
            match product.is_kitchen_print_enabled {
                1 => return true,                      // 明确启用
                0 => return false,                     // 明确禁用
                _ => {                                 // -1: 继承分类
                    if let Some(category) = inner.categories.get(&product.category_id) {
                        return category.is_kitchen_print_enabled;
                    }
                }
            }
        }
        false
    }

    /// 判断商品是否启用标签打印 (tri-state: -1=继承, 0=禁用, 1=启用)
    pub async fn is_product_label_enabled(&self, product_id: &str) -> bool {
        let inner = self.inner.read().await;

        if let Some(product) = inner.products.get(product_id) {
            match product.is_label_print_enabled {
                1 => return true,                      // 明确启用
                0 => return false,                     // 明确禁用
                _ => {                                 // -1: 继承分类
                    if let Some(category) = inner.categories.get(&product.category_id) {
                        return category.is_label_print_enabled;
                    }
                }
            }
        }
        false
    }

    /// 获取厨房打印目的地（商品 > 分类 > 系统默认）
    pub async fn get_kitchen_destinations(&self, product_id: &str) -> Vec<String> {
        let inner = self.inner.read().await;

        if let Some(product) = inner.products.get(product_id) {
            // 先检查是否启用厨房打印
            let enabled = match product.is_kitchen_print_enabled {
                1 => true,
                0 => false,
                _ => inner
                    .categories
                    .get(&product.category_id)
                    .map(|c| c.is_kitchen_print_enabled)
                    .unwrap_or(false),
            };

            if !enabled {
                return vec![];
            }

            // 商品有配置
            if !product.kitchen_print_destinations.is_empty() {
                return product.kitchen_print_destinations.clone();
            }
            // 回退到分类
            if let Some(category) = inner.categories.get(&product.category_id) {
                if !category.kitchen_print_destinations.is_empty() {
                    return category.kitchen_print_destinations.clone();
                }
            }
        }

        // 最终回退到系统默认
        inner.default_kitchen_printer.iter().cloned().collect()
    }

    /// 获取标签打印目的地（商品 > 分类 > 系统默认）
    pub async fn get_label_destinations(&self, product_id: &str) -> Vec<String> {
        let inner = self.inner.read().await;

        if let Some(product) = inner.products.get(product_id) {
            // 先检查是否启用标签打印
            let enabled = match product.is_label_print_enabled {
                1 => true,
                0 => false,
                _ => inner
                    .categories
                    .get(&product.category_id)
                    .map(|c| c.is_label_print_enabled)
                    .unwrap_or(false),
            };

            if !enabled {
                return vec![];
            }

            // 商品有配置
            if !product.label_print_destinations.is_empty() {
                return product.label_print_destinations.clone();
            }
            // 回退到分类
            if let Some(category) = inner.categories.get(&product.category_id) {
                if !category.label_print_destinations.is_empty() {
                    return category.label_print_destinations.clone();
                }
            }
        }

        // 最终回退到系统默认
        inner.default_label_printer.iter().cloned().collect()
    }

    /// 清空缓存
    pub async fn clear(&self) {
        let mut inner = self.inner.write().await;
        inner.products.clear();
        inner.categories.clear();
    }
}

impl Default for PrintConfigCache {
    fn default() -> Self {
        Self::new()
    }
}
