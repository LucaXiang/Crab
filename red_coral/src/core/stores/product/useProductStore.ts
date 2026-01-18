import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { createApiClient, type ProductQuery } from '@/infrastructure/api';
import type { Product } from '@/infrastructure/api/types';
import { logger } from '@/utils/logger';

// API Client
const api = createApiClient();

// 类型转换：从 API Product 转换到本地 Product 类型
function transformApiProduct(apiProduct: any): any {
  return {
    id: apiProduct.id,
    uuid: apiProduct.uuid,
    name: apiProduct.name,
    receiptName: apiProduct.receipt_name,
    price: apiProduct.price,
    image: apiProduct.image,
    category: apiProduct.category_id,
    externalId: apiProduct.external_id,
    taxRate: apiProduct.tax_rate,
    sortOrder: apiProduct.sort_order,
    kitchenPrinterId: apiProduct.kitchen_printer_id,
    isKitchenPrintEnabled: apiProduct.is_kitchen_print_enabled,
    isLabelPrintEnabled: apiProduct.is_label_print_enabled,
  };
}

/**
 * Product cache entry with timestamp for LRU eviction
 */
interface CacheEntry {
  products: Product[];
  timestamp: number;
}

/**
 * Generic cache entry with timestamp
 */
interface GenericCacheEntry<T> {
  data: T;
  timestamp: number;
}

/**
 * Product cache configuration
 */
const CACHE_CONFIG = {
  MAX_ENTRIES: 10, // Maximum number of cached categories
  TTL_MS: 5 * 60 * 1000, // 5 minutes cache lifetime for products
  CATEGORIES_TTL_MS: 30 * 60 * 1000, // 30 minutes for categories (rarely change)
  ENABLE_CACHE: true, // Master switch
};

interface ProductStore {
  // State
  products: Product[]; // Products for current category
  categories: string[]; // Base categories from data source
  selectedCategory: string; // 'all' means show all categories
  searchQuery: string;
  _searchDebounceId: number | null;
  extraCategories: string[]; // Custom categories added via Debug Menu
  isLoading: boolean;
  error: string | null;
  dataVersion: number; // Global version to trigger reloads

  // Cache state (private)
  _productCache: Map<string, CacheEntry>;
  _categoriesCache: GenericCacheEntry<string[]> | null; // Cache for categories
  _loadingPromises: Map<string, Promise<any>>; // Prevent duplicate simultaneous requests

  // Computed
  availableCategories: string[];

  // Actions
  setSelectedCategory: (category: string) => void;
  setSearchQuery: (query: string) => void;
  addProduct: (product: Product) => void;
  addCategory: (name: string) => void;
  clearCache: () => void;
  clearCategoryCache: () => void;
  clearProductCache: () => void;
  refreshData: () => void; // Force reload all data

  // Data Source Actions
  loadCategories: () => Promise<void>;
  loadProductsByCategory: (category: string) => Promise<void>;
  loadProducts: () => Promise<void>;
  createProduct: (params: {
    name: string;
    receiptName?: string;
    price: number;
    category: string;
    image?: string;
    externalId: number;
    taxRate: number;
    sortOrder?: number;
    isKitchenPrintEnabled?: number;
    isLabelPrintEnabled?: number;
    kitchenPrinterId?: number | null;
  }) => Promise<void>;
}

/**
 * Generate cache key for products query
 */
function getCacheKey(category: string, search: string): string {
  const cat = category || 'all';
  const searchKey = search.trim() ? `-search:${search.trim()}` : '';
  return `${cat}${searchKey}`;
}

/**
 * Check if cache entry is still valid
 */
function isCacheValid(entry: CacheEntry): boolean {
  return Date.now() - entry.timestamp < CACHE_CONFIG.TTL_MS;
}

/**
 * Evict oldest cache entries if exceeding max size
 */
function evictOldestEntries(cache: Map<string, CacheEntry>): void {
  if (cache.size <= CACHE_CONFIG.MAX_ENTRIES) return;

  const entries = Array.from(cache.entries());
  entries.sort((a, b) => a[1].timestamp - b[1].timestamp);

  const toRemove = entries.slice(0, cache.size - CACHE_CONFIG.MAX_ENTRIES);
  toRemove.forEach(([key]) => cache.delete(key));

  logger.debug(`Evicted ${toRemove.length} old cache entries`, { component: 'ProductStore', action: 'evictCache' });
}

export const useProductStore = create<ProductStore>((set, get) => ({
  // Initial State
  products: [],
  categories: [],
  selectedCategory: 'all',
  searchQuery: '',
  _searchDebounceId: null,
  extraCategories: [],
  availableCategories: ['all'],
  isLoading: false,
  error: null,
  dataVersion: 0,
  _productCache: new Map(),
  _categoriesCache: null,
  _loadingPromises: new Map(),

  // Actions
  setSelectedCategory: (category: string) => {
    const { selectedCategory } = get();
    if (category === selectedCategory) return;

    set({ selectedCategory: category });
    // Load products for the selected category + current search
    get().loadProducts();
  },

  setSearchQuery: (query: string) => {
    const { _searchDebounceId } = get();
    if (_searchDebounceId) {
      window.clearTimeout(_searchDebounceId);
    }
    set({ searchQuery: query });
    const id = window.setTimeout(() => {
      get().loadProducts();
    }, 250);
    set({ _searchDebounceId: id });
  },

  addProduct: (product: Product) => {
    const { selectedCategory, _productCache } = get();

    // Invalidate cache for this category and 'all'
    const keysToInvalidate = [
      getCacheKey(selectedCategory, ''),
      getCacheKey('all', ''),
    ];
    keysToInvalidate.forEach((key) => _productCache.delete(key));

    // Only add to current list if product belongs to current category
    const categoryId = product.category;
    if ((categoryId !== null && String(categoryId) === selectedCategory) || selectedCategory === 'all') {
      set((state) => ({
        products: [product, ...state.products]
      }));
    }
  },

  addCategory: (name: string) => {
    const { categories, extraCategories } = get();
    // Don't add if already exists
    if (extraCategories.includes(name) || categories.includes(name)) {
      return;
    }
    set((state) => ({
      extraCategories: [...state.extraCategories, name],
      availableCategories: [...state.availableCategories, name]
    }));
    // Auto-switch to new category
    get().setSelectedCategory(name);
  },

  clearCache: () => {
    const { _productCache } = get();
    _productCache.clear();
    set({ _categoriesCache: null });
    logger.debug('All caches cleared (products + categories)', { component: 'ProductStore', action: 'clearCache' });
  },

  clearCategoryCache: () => {
    set({ _categoriesCache: null });
    logger.debug('Category cache cleared', { component: 'ProductStore', action: 'clearCategoryCache' });
  },

  clearProductCache: () => {
    const { _productCache } = get();
    _productCache.clear();
    logger.debug('Product cache cleared', { component: 'ProductStore', action: 'clearProductCache' });
  },

  refreshData: () => {
    set((state) => ({ dataVersion: state.dataVersion + 1 }));
    logger.debug('ProductStore dataVersion incremented', { component: 'ProductStore', action: 'refreshData' });
    const { products } = get();
    logger.debug(`ProductStore has ${products.length} products before refresh`, { component: 'ProductStore', action: 'refreshData' });
  },

  // Data Source Actions
  loadCategories: async () => {
    const { _categoriesCache, _loadingPromises } = get();

    // Check if already loading (prevent duplicate simultaneous requests)
    const existingPromise = _loadingPromises.get('categories');
    if (existingPromise) {
      logger.debug('Category request already in flight, reusing promise', { component: 'ProductStore', action: 'loadCategories' });
      return existingPromise;
    }

    // Check cache first (categories rarely change, use longer TTL)
    if (CACHE_CONFIG.ENABLE_CACHE && _categoriesCache) {
      const isCacheValid = Date.now() - _categoriesCache.timestamp < CACHE_CONFIG.CATEGORIES_TTL_MS;
      if (isCacheValid) {
        logger.debug('Category cache HIT', { component: 'ProductStore', action: 'loadCategories' });
        const { extraCategories } = get();
        set({
          categories: _categoriesCache.data,
          availableCategories: ['all', ..._categoriesCache.data, ...extraCategories],
          isLoading: false,
        });
        return Promise.resolve();
      }
    }

    logger.debug('Category cache MISS', { component: 'ProductStore', action: 'loadCategories' });
    set({ isLoading: true, error: null });

    // Create loading promise and store it
    const loadingPromise = (async () => {
      try {
        const response = await api.listCategories();
        const categories = response.data?.categories || [];
        const { extraCategories } = get();

        // Store in cache
        if (CACHE_CONFIG.ENABLE_CACHE) {
          set({
            _categoriesCache: {
              data: categories.map(c => c.name),
              timestamp: Date.now(),
            },
          });
          logger.debug(`Cached ${categories.length} categories`, { component: 'ProductStore', action: 'loadCategories' });
        }

        set({
          categories: categories.map(c => c.name),
          availableCategories: ['all', ...categories.map(c => c.name), ...extraCategories],
          isLoading: false
        });

        // Load products for first category if no category selected
        const { selectedCategory } = get();
        if (!selectedCategory) {
          await get().setSelectedCategory('all');
        }
      } catch (error: any) {
        set({
          isLoading: false,
          error: error.message || 'Failed to load categories'
        });
        logger.error('Failed to load categories', error, { component: 'ProductStore', action: 'loadCategories' });
        throw error;
      } finally {
        // Remove promise from map when done
        const { _loadingPromises } = get();
        _loadingPromises.delete('categories');
      }
    })();

    // Store the promise
    _loadingPromises.set('categories', loadingPromise);

    return loadingPromise;
  },

  loadProductsByCategory: async (category: string) => {
    set({ isLoading: true, error: null });

    try {
      const params: ProductQuery = { page_size: 1000 };
      if (category && category !== 'all') params.category_id = parseInt(category);
      const response = await api.listProducts(params);
      const products = response.data?.products || [];
      const specs = response.data?.specs || [];

      // Helper to find root spec for a product (root spec stores price and external_id)
      const findRootSpec = (productId: number) => {
          return specs.find((s: any) => s.product_id === productId && s.is_root);
      };

      // Transform products with price and external_id from root spec
      const transformedProducts = products.map((p: any) => {
          const rootSpec = findRootSpec(p.id);
          return {
              ...transformApiProduct(p),
              price: rootSpec?.price ?? 0,
              externalId: rootSpec?.external_id ?? null,
          };
      });

      // Sort based on category
      const isAll = !category || category === 'all';
      transformedProducts.sort((a, b) => {
        if (isAll) {
          return a.externalId - b.externalId;
        } else {
          // Sort by sortOrder
          const orderA = a.sortOrder ?? Number.MAX_SAFE_INTEGER;
          const orderB = b.sortOrder ?? Number.MAX_SAFE_INTEGER;
          if (orderA !== orderB) return orderA - orderB;
          if (a.externalId !== b.externalId) return a.externalId - b.externalId;
          // Fallback to name
          return a.name.localeCompare(b.name);
        }
      });

      set({
        products: transformedProducts,
        isLoading: false
      });
    } catch (error: any) {
      set({
        isLoading: false,
        error: error.message || 'Failed to load products'
      });
      logger.error('Failed to load products by category', error, { component: 'ProductStore', action: 'loadProductsByCategory' });
    }
  },

  loadProducts: async () => {
    const { selectedCategory, searchQuery, _productCache, _loadingPromises } = get();

    // Generate cache key
    const cacheKey = getCacheKey(selectedCategory, searchQuery);

    const existingPromise = _loadingPromises.get(cacheKey);
    if (existingPromise) return existingPromise;

    // Check cache first (if enabled and no search query)
    if (CACHE_CONFIG.ENABLE_CACHE && !searchQuery.trim()) {
      const cached = _productCache.get(cacheKey);
      if (cached && isCacheValid(cached)) {
        logger.debug(`Product cache HIT for key: ${cacheKey}`, { component: 'ProductStore', action: 'loadProducts' });
        set({ products: cached.products, isLoading: false });
        return;
      }
    }

    logger.debug(`Product cache MISS for key: ${cacheKey}`, { component: 'ProductStore', action: 'loadProducts' });
    set({ isLoading: true, error: null });

    const loadingPromise = (async () => {
      try {
        const params: ProductQuery = { page_size: 1000 };
        if (selectedCategory && selectedCategory !== 'all') params.category_id = parseInt(selectedCategory);
        if (searchQuery && searchQuery.trim()) params.search = searchQuery.trim();
        const response = await api.listProducts(params);
        const products = response.data?.products || [];
        const specs = response.data?.specs || [];

        // Helper to find root spec for a product (root spec stores price and external_id)
        const findRootSpec = (productId: number) => {
            return specs.find((s: any) => s.product_id === productId && s.is_root);
        };

        // Transform products with price and external_id from root spec
        // Backend already converts price from cents to euros
        const transformedProducts = products.map((p: any) => {
            const rootSpec = findRootSpec(p.id);
            return {
                ...transformApiProduct(p),
                price: rootSpec?.price ?? 0,  // Price is already in euros from backend
                externalId: rootSpec?.external_id ?? null,
            };
        });

        // Sort based on category
        const isAll = !selectedCategory || selectedCategory === 'all';
        transformedProducts.sort((a, b) => {
          if (isAll) {
            return a.externalId - b.externalId;
          } else {
            // Sort by sortOrder
            const orderA = a.sortOrder ?? Number.MAX_SAFE_INTEGER;
            const orderB = b.sortOrder ?? Number.MAX_SAFE_INTEGER;
            if (orderA !== orderB) return orderA - orderB;
            // Fallback to externalId
            if (a.externalId !== b.externalId) return a.externalId - b.externalId;
            // Fallback to name
            return a.name.localeCompare(b.name);
          }
        });

        // Store in cache (only if no search query, to avoid cache bloat)
        if (CACHE_CONFIG.ENABLE_CACHE && !searchQuery.trim()) {
          _productCache.set(cacheKey, {
            products: transformedProducts,
            timestamp: Date.now(),
          });
          evictOldestEntries(_productCache);
          logger.debug(`Cached ${transformedProducts.length} products for key: ${cacheKey}`, { component: 'ProductStore', action: 'loadProducts' });
        }

        logger.debug(`Setting ${transformedProducts.length} products to state`, { component: 'ProductStore', action: 'loadProducts' });
        set({ products: transformedProducts, isLoading: false });
      } catch (error: any) {
        set({
          isLoading: false,
          error: error.message || 'Failed to load products'
        });
        logger.error('Failed to load products', error, { component: 'ProductStore', action: 'loadProducts' });
        throw error;
      } finally {
        const { _loadingPromises } = get();
        _loadingPromises.delete(cacheKey);
      }
    })();

    _loadingPromises.set(cacheKey, loadingPromise);
    return loadingPromise;
  },

  createProduct: async (params: any) => {
    set({ isLoading: true, error: null });

    try {
      const response = await api.createProduct(params);
      const product = transformApiProduct(response.data);
      get().addProduct(product);
      set({ isLoading: false });
    } catch (error: any) {
      set({
        isLoading: false,
        error: error.message || 'Failed to create product'
      });
      logger.error('Failed to create product', error, { component: 'ProductStore', action: 'createProduct' });
      throw error;
    }
  }
}));

// NOTE: loadCategories() is intentionally NOT called here automatically.
// It should be called explicitly when needed (e.g., after login, in POS page)
// to avoid unnecessary API calls and potential errors when not logged in.

// ============ Granular Selectors (Performance Optimization) ============

export const useProducts = () => useProductStore((state) => state.products);
export const useSelectedCategory = () => useProductStore((state) => state.selectedCategory);
export const useAvailableCategories = () => useProductStore((state) => state.availableCategories);
export const useProductLoading = () => useProductStore((state) => state.isLoading);
export const useProductSearchQuery = () => useProductStore((state) => state.searchQuery);

export const useCategoryData = () => useProductStore(
  useShallow((state) => ({
    selected: state.selectedCategory,
    categories: state.availableCategories
  }))
);

export const useProductActions = () => useProductStore(
  useShallow((state) => ({
    setSelectedCategory: state.setSelectedCategory,
    setSearchQuery: state.setSearchQuery,
    addProduct: state.addProduct,
    addCategory: state.addCategory,
    loadCategories: state.loadCategories,
    loadProductsByCategory: state.loadProductsByCategory,
    loadProducts: state.loadProducts,
    clearCache: state.clearCache,
    clearCategoryCache: state.clearCategoryCache,
    clearProductCache: state.clearProductCache,
    refreshData: state.refreshData,
  }))
);
