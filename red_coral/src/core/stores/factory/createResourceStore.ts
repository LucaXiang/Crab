import { create } from 'zustand';

/**
 * Sync payload for version-based incremental sync
 *
 * 版本同步策略:
 * - lastVersion = 0: 初始状态，接受任何版本的同步消息
 * - lastVersion > 0: 正常同步，检查版本连续性
 * - 检测到版本间隙: 触发全量刷新
 */
export interface SyncPayload<T = unknown> {
  id: number;
  version: number;
  action: 'created' | 'updated' | 'deleted';
  data: T | null;
}

/**
 * 资源 Store 基础接口
 *
 * 所有资源 Store 统一使用此接口，确保一致的数据管理模式。
 */
export interface ResourceStore<T extends { id: number }> {
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
  getById: (id: number) => T | undefined;
  clear: () => void;
}

/**
 * 带 CRUD 操作的资源 Store 接口
 */
export interface CrudResourceStore<T extends { id: number }, TCreate, TUpdate>
  extends ResourceStore<T> {
  create: (data: TCreate) => Promise<T>;
  update: (id: number, data: TUpdate) => Promise<T>;
  remove: (id: number) => Promise<void>;
  // 乐观更新辅助
  optimisticUpdate: (id: number, updater: (item: T) => T, version?: number) => void;
  optimisticRemove: (id: number) => void;
  optimisticAdd: (item: T) => void;
}

/**
 * CRUD 操作配置
 */
export interface CrudOperations<T, TCreate, TUpdate> {
  create: (data: TCreate) => Promise<T>;
  update: (id: number, data: TUpdate) => Promise<T>;
  remove: (id: number) => Promise<void>;
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
export function createResourceStore<T extends { id: number }>(
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
      const state = get();
      if (state.isLoading) return;
      if (state.isLoaded && !force) return;

      set({ isLoading: true, error: null });
      try {
        const items = await fetchFn();
        set({ items, isLoading: false, isLoaded: true });
      } catch (e: unknown) {
        const errorMsg = e instanceof Error ? e.message : 'Failed to fetch';
        set({ error: errorMsg, isLoading: false });
      }
    },

    applySync: (payload: SyncPayload<T>) => {
      const state = get();
      const { id, version, action, data } = payload;

      // Skip if duplicate (but allow if lastVersion is 0, meaning never synced)
      if (state.lastVersion > 0 && version <= state.lastVersion) {
        return;
      }

      // Gap detected: need full refresh
      // Only trigger fetchAll if we have a previous version to compare against
      if (state.lastVersion > 0 && version > state.lastVersion + 1) {
        if (state.isLoaded) {
          get().fetchAll(true);
        }
        return;
      }

      switch (action) {
        case 'created':
          if (data) {
            // Check if item already exists (from optimistic add)
            const exists = state.items.some((item) => item.id === id);
            if (exists) {
              // Update existing item instead of adding duplicate
              set((s) => ({
                items: s.items.map((item) => (item.id === id ? data : item)),
                lastVersion: version,
              }));
            } else {
              set((s) => ({
                items: [...s.items, data],
                lastVersion: version,
              }));
            }
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
  T extends { id: number },
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
      const state = get();
      if (state.isLoading) return;
      if (state.isLoaded && !force) return;

      set({ isLoading: true, error: null });
      try {
        const items = await fetchFn();
        set({ items, isLoading: false, isLoaded: true });
      } catch (e: unknown) {
        const errorMsg = e instanceof Error ? e.message : 'Failed to fetch';
        set({ error: errorMsg, isLoading: false });
      }
    },

    applySync: (payload: SyncPayload<T>) => {
      const state = get();
      const { id, version, action, data } = payload;

      // Skip if duplicate (but allow if lastVersion is 0, meaning never synced)
      if (state.lastVersion > 0 && version <= state.lastVersion) {
        return;
      }

      // Gap detected: need full refresh
      // Only trigger fetchAll if we have a previous version to compare against
      if (state.lastVersion > 0 && version > state.lastVersion + 1) {
        if (state.isLoaded) {
          get().fetchAll(true);
        }
        return;
      }

      switch (action) {
        case 'created':
          if (data) {
            // Check if item already exists (from optimistic add)
            const exists = state.items.some((item) => item.id === id);
            if (exists) {
              // Update existing item instead of adding duplicate
              set((s) => ({
                items: s.items.map((item) => (item.id === id ? data : item)),
                lastVersion: version,
              }));
            } else {
              set((s) => ({
                items: [...s.items, data],
                lastVersion: version,
              }));
            }
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
    // version 参数可选，传入时会同时更新 lastVersion，避免后续 sync 触发不必要的 fetchAll
    optimisticUpdate: (id, updater, version) => {
      set((state) => ({
        items: state.items.map((item) =>
          item.id === id ? updater(item) : item
        ),
        ...(version !== undefined && { lastVersion: version }),
      }));
    },

    optimisticRemove: (id) => {
      set((state) => ({
        items: state.items.filter((item) => item.id !== id),
      }));
    },

    optimisticAdd: (item) => {
      set((state) => {
        const exists = state.items.some((i) => i.id === item.id);
        if (exists) return state;
        return { items: [...state.items, item] };
      });
    },
  }));
}
