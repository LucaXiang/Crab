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

    /// 获取系统默认打印机
    ///
    /// Returns (kitchen_printer_id, label_printer_id)
    pub async fn get_defaults(&self) -> (Option<String>, Option<String>) {
        let inner = self.inner.read().await;
        (
            inner.default_kitchen_printer.clone(),
            inner.default_label_printer.clone(),
        )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn create_product_config(
        product_id: &str,
        category_id: &str,
        kitchen_enabled: i32,
        label_enabled: i32,
        kitchen_dests: Vec<&str>,
        label_dests: Vec<&str>,
    ) -> ProductPrintConfig {
        ProductPrintConfig {
            product_id: product_id.to_string(),
            product_name: format!("Product {}", product_id),
            kitchen_name: format!("Kitchen {}", product_id),
            kitchen_print_destinations: kitchen_dests.into_iter().map(String::from).collect(),
            label_print_destinations: label_dests.into_iter().map(String::from).collect(),
            is_kitchen_print_enabled: kitchen_enabled,
            is_label_print_enabled: label_enabled,
            root_spec_external_id: None,
            category_id: category_id.to_string(),
        }
    }

    fn create_category_config(
        category_id: &str,
        kitchen_enabled: bool,
        label_enabled: bool,
        kitchen_dests: Vec<&str>,
        label_dests: Vec<&str>,
    ) -> CategoryPrintConfig {
        CategoryPrintConfig {
            category_id: category_id.to_string(),
            category_name: format!("Category {}", category_id),
            kitchen_print_destinations: kitchen_dests.into_iter().map(String::from).collect(),
            label_print_destinations: label_dests.into_iter().map(String::from).collect(),
            is_kitchen_print_enabled: kitchen_enabled,
            is_label_print_enabled: label_enabled,
        }
    }

    #[tokio::test]
    async fn test_defaults() {
        let cache = PrintConfigCache::new();

        // Initially no defaults
        assert!(!cache.is_kitchen_print_enabled().await);
        assert!(!cache.is_label_print_enabled().await);
        let (k, l) = cache.get_defaults().await;
        assert!(k.is_none());
        assert!(l.is_none());

        // Set defaults
        cache.set_defaults(Some("printer-k".to_string()), Some("printer-l".to_string())).await;

        assert!(cache.is_kitchen_print_enabled().await);
        assert!(cache.is_label_print_enabled().await);
        let (k, l) = cache.get_defaults().await;
        assert_eq!(k, Some("printer-k".to_string()));
        assert_eq!(l, Some("printer-l".to_string()));

        // Clear one default
        cache.set_defaults(None, Some("printer-l".to_string())).await;
        assert!(!cache.is_kitchen_print_enabled().await);
        assert!(cache.is_label_print_enabled().await);
    }

    #[tokio::test]
    async fn test_product_crud() {
        let cache = PrintConfigCache::new();

        let config = create_product_config("p1", "c1", 1, 0, vec!["dest1"], vec![]);
        cache.update_product(config).await;

        let p = cache.get_product("p1").await;
        assert!(p.is_some());
        assert_eq!(p.unwrap().product_id, "p1");

        cache.remove_product("p1").await;
        assert!(cache.get_product("p1").await.is_none());
    }

    #[tokio::test]
    async fn test_category_crud() {
        let cache = PrintConfigCache::new();

        let config = create_category_config("c1", true, false, vec!["dest1"], vec![]);
        cache.update_category(config).await;

        let c = cache.get_category("c1").await;
        assert!(c.is_some());
        assert_eq!(c.unwrap().category_id, "c1");

        cache.remove_category("c1").await;
        assert!(cache.get_category("c1").await.is_none());
    }

    #[tokio::test]
    async fn test_tristate_kitchen_enabled() {
        let cache = PrintConfigCache::new();

        // Setup category with kitchen enabled
        cache.update_category(create_category_config("c1", true, false, vec![], vec![])).await;

        // Product explicitly enabled
        cache.update_product(create_product_config("p1", "c1", 1, 0, vec![], vec![])).await;
        assert!(cache.is_product_kitchen_enabled("p1").await);

        // Product explicitly disabled
        cache.update_product(create_product_config("p2", "c1", 0, 0, vec![], vec![])).await;
        assert!(!cache.is_product_kitchen_enabled("p2").await);

        // Product inherits from category (-1)
        cache.update_product(create_product_config("p3", "c1", -1, 0, vec![], vec![])).await;
        assert!(cache.is_product_kitchen_enabled("p3").await);

        // Product inherits but category disabled
        cache.update_category(create_category_config("c2", false, false, vec![], vec![])).await;
        cache.update_product(create_product_config("p4", "c2", -1, 0, vec![], vec![])).await;
        assert!(!cache.is_product_kitchen_enabled("p4").await);

        // Unknown product
        assert!(!cache.is_product_kitchen_enabled("unknown").await);
    }

    #[tokio::test]
    async fn test_tristate_label_enabled() {
        let cache = PrintConfigCache::new();

        // Setup category with label enabled
        cache.update_category(create_category_config("c1", false, true, vec![], vec![])).await;

        // Product explicitly enabled
        cache.update_product(create_product_config("p1", "c1", 0, 1, vec![], vec![])).await;
        assert!(cache.is_product_label_enabled("p1").await);

        // Product explicitly disabled
        cache.update_product(create_product_config("p2", "c1", 0, 0, vec![], vec![])).await;
        assert!(!cache.is_product_label_enabled("p2").await);

        // Product inherits from category (-1)
        cache.update_product(create_product_config("p3", "c1", 0, -1, vec![], vec![])).await;
        assert!(cache.is_product_label_enabled("p3").await);
    }

    #[tokio::test]
    async fn test_kitchen_destinations_fallback() {
        let cache = PrintConfigCache::new();

        // Setup system default
        cache.set_defaults(Some("default-k".to_string()), None).await;

        // Setup category with destinations
        cache.update_category(create_category_config("c1", true, false, vec!["cat-k"], vec![])).await;

        // Product with own destinations - should use product's
        cache.update_product(create_product_config("p1", "c1", 1, 0, vec!["prod-k"], vec![])).await;
        let dests = cache.get_kitchen_destinations("p1").await;
        assert_eq!(dests, vec!["prod-k"]);

        // Product without destinations - should fallback to category
        cache.update_product(create_product_config("p2", "c1", 1, 0, vec![], vec![])).await;
        let dests = cache.get_kitchen_destinations("p2").await;
        assert_eq!(dests, vec!["cat-k"]);

        // Product in category without destinations - should fallback to system default
        cache.update_category(create_category_config("c2", true, false, vec![], vec![])).await;
        cache.update_product(create_product_config("p3", "c2", 1, 0, vec![], vec![])).await;
        let dests = cache.get_kitchen_destinations("p3").await;
        assert_eq!(dests, vec!["default-k"]);

        // Product with kitchen disabled - should return empty
        cache.update_product(create_product_config("p4", "c1", 0, 0, vec![], vec![])).await;
        let dests = cache.get_kitchen_destinations("p4").await;
        assert!(dests.is_empty());

        // Unknown product - should return system default
        let dests = cache.get_kitchen_destinations("unknown").await;
        assert_eq!(dests, vec!["default-k"]);
    }

    #[tokio::test]
    async fn test_label_destinations_fallback() {
        let cache = PrintConfigCache::new();

        // Setup system default
        cache.set_defaults(None, Some("default-l".to_string())).await;

        // Setup category with destinations
        cache.update_category(create_category_config("c1", false, true, vec![], vec!["cat-l"])).await;

        // Product with own destinations
        cache.update_product(create_product_config("p1", "c1", 0, 1, vec![], vec!["prod-l"])).await;
        let dests = cache.get_label_destinations("p1").await;
        assert_eq!(dests, vec!["prod-l"]);

        // Product without destinations - fallback to category
        cache.update_product(create_product_config("p2", "c1", 0, 1, vec![], vec![])).await;
        let dests = cache.get_label_destinations("p2").await;
        assert_eq!(dests, vec!["cat-l"]);

        // Product with label disabled
        cache.update_product(create_product_config("p3", "c1", 0, 0, vec![], vec![])).await;
        let dests = cache.get_label_destinations("p3").await;
        assert!(dests.is_empty());
    }

    #[tokio::test]
    async fn test_clear() {
        let cache = PrintConfigCache::new();

        cache.update_product(create_product_config("p1", "c1", 1, 1, vec![], vec![])).await;
        cache.update_category(create_category_config("c1", true, true, vec![], vec![])).await;

        assert!(cache.get_product("p1").await.is_some());
        assert!(cache.get_category("c1").await.is_some());

        cache.clear().await;

        assert!(cache.get_product("p1").await.is_none());
        assert!(cache.get_category("c1").await.is_none());
    }
}
