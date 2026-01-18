import { create } from 'zustand';

/**
 * 资源 Store 接口
 *
 * 所有资源 Store 统一使用此接口，确保一致的数据管理模式。
 */
export interface ResourceStore<T extends { id: string }> {
  // 状态
  items: T[];
  isLoading: boolean;
  isLoaded: boolean;
  error: string | null;

  // 方法
  fetchAll: () => Promise<void>;
  applySync: () => void;
  getById: (id: string) => T | undefined;
  clear: () => void;
}

/**
 * 创建资源 Store 的工厂函数
 *
 * 服务器权威模型：
 * - 收到 Sync 信号时直接全量刷新，不做增量更新
 * - 组件必须直接订阅 Store，数据变化自动触发重新渲染
 *
 * @param resourceName - 资源名称（用于日志）
 * @param fetchFn - 获取数据的函数
 */
export function createResourceStore<T extends { id: string }>(
  resourceName: string,
  fetchFn: () => Promise<T[]>
) {
  return create<ResourceStore<T>>((set, get) => ({
    items: [],
    isLoading: false,
    isLoaded: false,
    error: null,

    fetchAll: async () => {
      set({ isLoading: true, error: null });
      try {
        const items = await fetchFn();
        set({ items, isLoading: false, isLoaded: true });
        console.log(`[Store] ${resourceName}: loaded ${items.length} items`);
      } catch (e: any) {
        const errorMsg = e.message || 'Failed to fetch';
        set({ error: errorMsg, isLoading: false });
        console.error(`[Store] ${resourceName}: fetch failed -`, errorMsg);
      }
    },

    // 服务器权威：收到 Sync 直接全量刷新
    applySync: () => {
      if (get().isLoaded) {
        console.log(`[Store] ${resourceName}: sync triggered, refreshing...`);
        get().fetchAll();
      }
    },

    getById: (id) => get().items.find((item) => item.id === id),

    clear: () => set({ items: [], isLoaded: false, error: null }),
  }));
}
