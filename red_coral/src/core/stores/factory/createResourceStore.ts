import { create } from 'zustand';

/**
 * 资源 Store 基础接口
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
 * 带 CRUD 操作的资源 Store 接口
 */
export interface CrudResourceStore<T extends { id: string }, TCreate, TUpdate>
  extends ResourceStore<T> {
  create: (data: TCreate) => Promise<T>;
  update: (id: string, data: TUpdate) => Promise<T>;
  remove: (id: string) => Promise<void>;
  // 乐观更新辅助
  optimisticUpdate: (id: string, updater: (item: T) => T) => void;
  optimisticRemove: (id: string) => void;
  optimisticAdd: (item: T) => void;
}

/**
 * CRUD 操作配置
 */
export interface CrudOperations<T, TCreate, TUpdate> {
  create: (data: TCreate) => Promise<T>;
  update: (id: string, data: TUpdate) => Promise<T>;
  remove: (id: string) => Promise<void>;
}

/**
 * 创建只读资源 Store 的工厂函数
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

/**
 * 创建带 CRUD 操作的资源 Store
 *
 * 在只读 Store 基础上增加：
 * - create/update/remove 操作
 * - 乐观更新支持
 *
 * @param resourceName - 资源名称
 * @param fetchFn - 获取数据的函数
 * @param crudOps - CRUD 操作函数
 */
export function createCrudResourceStore<
  T extends { id: string },
  TCreate = Partial<Omit<T, 'id'>>,
  TUpdate = Partial<Omit<T, 'id'>>
>(
  resourceName: string,
  fetchFn: () => Promise<T[]>,
  crudOps: CrudOperations<T, TCreate, TUpdate>
) {
  return create<CrudResourceStore<T, TCreate, TUpdate>>((set, get) => ({
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

    applySync: () => {
      if (get().isLoaded) {
        console.log(`[Store] ${resourceName}: sync triggered, refreshing...`);
        get().fetchAll();
      }
    },

    getById: (id) => get().items.find((item) => item.id === id),

    clear: () => set({ items: [], isLoaded: false, error: null }),

    // CRUD 操作
    create: async (data) => {
      try {
        const newItem = await crudOps.create(data);
        // 服务器权威：创建成功后等待 Sync 信号刷新
        // 但为了 UI 响应，可以先乐观添加
        set((state) => ({ items: [...state.items, newItem] }));
        console.log(`[Store] ${resourceName}: created item ${newItem.id}`);
        return newItem;
      } catch (e: any) {
        console.error(`[Store] ${resourceName}: create failed -`, e.message);
        throw e;
      }
    },

    update: async (id, data) => {
      try {
        const updatedItem = await crudOps.update(id, data);
        set((state) => ({
          items: state.items.map((item) =>
            item.id === id ? updatedItem : item
          ),
        }));
        console.log(`[Store] ${resourceName}: updated item ${id}`);
        return updatedItem;
      } catch (e: any) {
        console.error(`[Store] ${resourceName}: update failed -`, e.message);
        throw e;
      }
    },

    remove: async (id) => {
      try {
        await crudOps.remove(id);
        set((state) => ({
          items: state.items.filter((item) => item.id !== id),
        }));
        console.log(`[Store] ${resourceName}: removed item ${id}`);
      } catch (e: any) {
        console.error(`[Store] ${resourceName}: remove failed -`, e.message);
        throw e;
      }
    },

    // 乐观更新辅助方法
    optimisticUpdate: (id, updater) => {
      set((state) => ({
        items: state.items.map((item) =>
          item.id === id ? updater(item) : item
        ),
      }));
    },

    optimisticRemove: (id) => {
      set((state) => ({
        items: state.items.filter((item) => item.id !== id),
      }));
    },

    optimisticAdd: (item) => {
      set((state) => ({ items: [...state.items, item] }));
    },
  }));
}
