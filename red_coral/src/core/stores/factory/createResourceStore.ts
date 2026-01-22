import { create } from 'zustand';

/**
 * Sync payload for version-based incremental sync
 */
export interface SyncPayload<T = unknown> {
  id: string;
  version: number;
  action: 'created' | 'updated' | 'deleted';
  data: T | null;
}

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
  lastVersion: number;

  // 方法
  fetchAll: (force?: boolean) => Promise<void>;
  applySync: (payload: SyncPayload<T>) => void;
  checkVersion: (serverVersion: number) => boolean;
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
 * 版本同步模型：
 * - 收到 Sync 信号时根据版本号决定是否需要更新
 * - 版本号连续时做增量更新
 * - 版本号有缺口时触发全量刷新
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
    lastVersion: 0,

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

    // 版本同步：根据版本号决定增量更新或全量刷新
    applySync: (payload: SyncPayload<T>) => {
      const state = get();
      const { id, version, action, data } = payload;

      console.log(`[${resourceName}] applySync called, id=${id}, version=${version}, action=${action}, lastVersion=${state.lastVersion}`);

      // Skip if duplicate (version already seen)
      if (version <= state.lastVersion) {
        console.log(`[${resourceName}] applySync skipped for id=${id} (duplicate, version=${version} <= lastVersion=${state.lastVersion})`);
        return;
      }

      // Gap detected: need full refresh
      if (version > state.lastVersion + 1) {
        console.log(`[${resourceName}] applySync detected gap (version=${version}, lastVersion=${state.lastVersion}), triggering fetchAll`);
        if (state.isLoaded) {
          get().fetchAll(true);
        }
        return;
      }

      // Incremental update: version === lastVersion + 1
      console.log(`[${resourceName}] applySync performing incremental update for action=${action}`);

      switch (action) {
        case 'created':
          if (data) {
            set((s) => ({
              items: [...s.items, data],
              lastVersion: version,
            }));
          }
          break;
        case 'updated':
          if (data) {
            set((s) => ({
              items: s.items.map((item) => (item.id === id ? data : item)),
              lastVersion: version,
            }));
          }
          break;
        case 'deleted':
          set((s) => ({
            items: s.items.filter((item) => item.id !== id),
            lastVersion: version,
          }));
          break;
      }
    },

    checkVersion: (serverVersion: number) => {
      return serverVersion > get().lastVersion;
    },

    getById: (id) => get().items.find((item) => item.id === id),

    clear: () => set({ items: [], isLoaded: false, error: null, lastVersion: 0 }),
  }));
}

/**
 * 创建带 CRUD 操作的资源 Store
 *
 * 在只读 Store 基础上增加：
 * - create/update/remove 操作
 * - 乐观更新支持
 * - 版本同步（增量更新或全量刷新）
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
    lastVersion: 0,

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

    // 版本同步：根据版本号决定增量更新或全量刷新
    applySync: (payload: SyncPayload<T>) => {
      const state = get();
      const { id, version, action, data } = payload;

      console.log(`[${resourceName}] applySync called, id=${id}, version=${version}, action=${action}, lastVersion=${state.lastVersion}`);

      // Skip if duplicate (version already seen)
      if (version <= state.lastVersion) {
        console.log(`[${resourceName}] applySync skipped for id=${id} (duplicate, version=${version} <= lastVersion=${state.lastVersion})`);
        return;
      }

      // Gap detected: need full refresh
      if (version > state.lastVersion + 1) {
        console.log(`[${resourceName}] applySync detected gap (version=${version}, lastVersion=${state.lastVersion}), triggering fetchAll`);
        if (state.isLoaded) {
          get().fetchAll(true);
        }
        return;
      }

      // Incremental update: version === lastVersion + 1
      console.log(`[${resourceName}] applySync performing incremental update for action=${action}`);

      switch (action) {
        case 'created':
          if (data) {
            set((s) => ({
              items: [...s.items, data],
              lastVersion: version,
            }));
          }
          break;
        case 'updated':
          if (data) {
            set((s) => ({
              items: s.items.map((item) => (item.id === id ? data : item)),
              lastVersion: version,
            }));
          }
          break;
        case 'deleted':
          set((s) => ({
            items: s.items.filter((item) => item.id !== id),
            lastVersion: version,
          }));
          break;
      }
    },

    checkVersion: (serverVersion: number) => {
      return serverVersion > get().lastVersion;
    },

    getById: (id) => get().items.find((item) => item.id === id),

    clear: () => set({ items: [], isLoaded: false, error: null, lastVersion: 0 }),

    // CRUD 操作
    create: async (data) => {
      const newItem = await crudOps.create(data);
      set((state) => ({ items: [...state.items, newItem] }));
      return newItem;
    },

    update: async (id, data) => {
      const updatedItem = await crudOps.update(id, data);
      set((state) => ({
        items: state.items.map((item) =>
          item.id === id ? updatedItem : item
        ),
      }));
      return updatedItem;
    },

    remove: async (id) => {
      await crudOps.remove(id);
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
