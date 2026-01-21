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
  fetchAll: (force?: boolean) => Promise<void>;
  applySync: (id: string) => void;
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

    fetchAll: async (force = false) => {
      // Guard: skip if already loading, or already loaded (unless forced)
      const state = get();
      console.log(`[${resourceName}] fetchAll called, force=${force}, isLoading=${state.isLoading}, isLoaded=${state.isLoaded}`);
      if (state.isLoading) return;
      if (state.isLoaded && !force) return;

      set({ isLoading: true, error: null });
      try {
        const items = await fetchFn();
        console.log(`[${resourceName}] fetchAll success, got ${items.length} items`);
        set({ items, isLoading: false, isLoaded: true });
      } catch (e: any) {
        const errorMsg = e.message || 'Failed to fetch';
        set({ error: errorMsg, isLoading: false });
        console.error(`[${resourceName}] fetch failed:`, errorMsg);
      }
    },

    // 服务器权威：收到 Sync 直接全量刷新
    // 只读 Store 没有本地 CRUD，无需去重
    applySync: (_id: string) => {
      if (get().isLoaded) {
        get().fetchAll(true);  // Force refresh on sync
      }
    },

    getById: (id) => get().items.find((item) => item.id === id),

    clear: () => set({ items: [], isLoaded: false, error: null }),
  }));
}

/** Sync 消息去重：本地操作后忽略自己触发的 Sync 的时间窗口 (ms) */
const PENDING_EXPIRY_MS = 1000;

/**
 * 创建带 CRUD 操作的资源 Store
 *
 * 在只读 Store 基础上增加：
 * - create/update/remove 操作
 * - 乐观更新支持
 * - Sync 消息去重（避免本地操作后重复刷新）
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
  // 用于 Sync 去重：记录最近本地操作过的 ID
  const pendingIds = new Set<string>();

  /** 添加 ID 到 pending 集合，延迟后自动清除 */
  const addPendingId = (id: string) => {
    pendingIds.add(id);
    setTimeout(() => pendingIds.delete(id), PENDING_EXPIRY_MS);
  };

  return create<CrudResourceStore<T, TCreate, TUpdate>>((set, get) => ({
    items: [],
    isLoading: false,
    isLoaded: false,
    error: null,

    fetchAll: async (force = false) => {
      // Guard: skip if already loading, or already loaded (unless forced)
      const state = get();
      console.log(`[${resourceName}] fetchAll called, force=${force}, isLoading=${state.isLoading}, isLoaded=${state.isLoaded}`);
      if (state.isLoading) return;
      if (state.isLoaded && !force) return;

      set({ isLoading: true, error: null });
      try {
        const items = await fetchFn();
        console.log(`[${resourceName}] fetchAll success, got ${items.length} items`);
        set({ items, isLoading: false, isLoaded: true });
      } catch (e: any) {
        const errorMsg = e.message || 'Failed to fetch';
        set({ error: errorMsg, isLoading: false });
        console.error(`[${resourceName}] fetch failed:`, errorMsg);
      }
    },

    // Sync 去重：如果是自己刚操作过的 ID，跳过刷新
    applySync: (id: string) => {
      console.log(`[${resourceName}] applySync called, id=${id}, isLoaded=${get().isLoaded}`);
      if (pendingIds.has(id)) {
        console.log(`[${resourceName}] applySync skipped for id=${id} (local pending)`);
        return;
      }
      if (get().isLoaded) {
        console.log(`[${resourceName}] applySync triggering fetchAll(true)`);
        get().fetchAll(true);  // Force refresh on sync
      }
    },

    getById: (id) => get().items.find((item) => item.id === id),

    clear: () => set({ items: [], isLoaded: false, error: null }),

    // CRUD 操作（操作完成后添加到 pendingIds）
    create: async (data) => {
      const newItem = await crudOps.create(data);
      addPendingId(newItem.id); // 服务端生成 ID，响应后添加
      set((state) => ({ items: [...state.items, newItem] }));
      return newItem;
    },

    update: async (id, data) => {
      const updatedItem = await crudOps.update(id, data);
      addPendingId(id);
      set((state) => ({
        items: state.items.map((item) =>
          item.id === id ? updatedItem : item
        ),
      }));
      return updatedItem;
    },

    remove: async (id) => {
      await crudOps.remove(id);
      addPendingId(id);
      set((state) => ({
        items: state.items.filter((item) => item.id !== id),
      }));
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
