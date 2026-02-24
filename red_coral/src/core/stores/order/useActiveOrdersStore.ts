/**
 * Active Orders Store (Server Authority Model)
 *
 * This store maintains a read-only mirror of server-side order state.
 * All state changes come from server - NO local computation allowed.
 *
 * Architecture:
 * - State is updated only through server-computed snapshots (_applyOrderSync, _fullSync)
 * - Client NEVER computes snapshots locally
 * - UI components read state through selectors
 * - Commands are sent via commands/ module (fire & forget)
 *
 * Event Flow:
 * 1. Server broadcasts OrderEvent with computed snapshot
 * 2. Event listener receives (event, snapshot) pair
 * 3. Listener calls _applyOrderSync with server-computed snapshot
 * 4. Store updates, React re-renders
 */

import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import type {
  OrderSnapshot,
  OrderEvent,
  OrderConnectionState,
} from '@/core/domain/types/orderEvent';
import { logger } from '@/utils/logger';

// ============================================================================
// Constants
// ============================================================================

/** Maximum sequence gap before triggering timeline sync for an order */
const MAX_SEQUENCE_GAP = 5;

// ============================================================================
// Store Interface
// ============================================================================

interface ActiveOrdersState {
  /** Map of order_id -> OrderSnapshot */
  orders: Map<string, OrderSnapshot>;
  /** Map of order_id -> OrderEvent[] (timeline for UI display) */
  timelines: Map<string, OrderEvent[]>;
  /** Last processed event sequence number */
  lastSequence: number;
  /** Connection state for order sync */
  connectionState: OrderConnectionState;
  /** Whether the store has been initialized with server data */
  isInitialized: boolean;
  /**
   * Server instance epoch (UUID)
   * Used to detect server restarts - if epoch changes, full sync is required
   */
  serverEpoch: string | null;
  /**
   * Orders that need timeline sync due to sequence gap
   * External listeners can watch this and trigger sync
   */
  ordersNeedingTimelineSync: Set<string>;
}

interface ActiveOrdersActions {
  // ==================== Read-Only Queries ====================

  /**
   * Get order by ID
   */
  getOrder: (orderId: string) => OrderSnapshot | undefined;

  /**
   * Get all active orders (status === 'ACTIVE')
   */
  getActiveOrders: () => OrderSnapshot[];

  /**
   * Get order by table ID
   */
  getOrderByTable: (tableId: number) => OrderSnapshot | undefined;

  /**
   * Get all orders (including completed/voided)
   */
  getAllOrders: () => OrderSnapshot[];

  /**
   * Check if a table has an active order
   */
  hasActiveOrderOnTable: (tableId: number) => boolean;

  // ==================== Internal Methods (Server Authority) ====================
  // These methods are prefixed with _ to indicate they should only be called
  // by event listeners, not by UI components directly.
  // IMPORTANT: Client NEVER computes snapshots locally - always use server-computed snapshots.

  /**
   * Apply order sync (Server Authority Model)
   * Called when receiving events with server-computed snapshots
   * - event: 追加到时间线 (用于 UI 渲染操作记录)
   * - snapshot: 替换状态 (服务端权威，无本地计算)
   */
  _applyOrderSync: (event: OrderEvent, snapshot: OrderSnapshot) => void;

  /**
   * Full sync: replace all orders with server state
   * Called when gap is too large, on initial load, or when server epoch changes
   *
   * @param orders - Active order snapshots from server
   * @param serverSequence - Server's current sequence number
   * @param serverEpoch - Server instance epoch (optional, updates if provided)
   * @param events - All events to populate timelines (optional)
   */
  _fullSync: (orders: OrderSnapshot[], serverSequence: number, serverEpoch?: string, events?: OrderEvent[]) => void;

  /**
   * Update connection state
   * Called by connection status listener
   */
  _setConnectionState: (state: OrderConnectionState) => void;

  /**
   * Mark store as initialized
   */
  _setInitialized: (initialized: boolean) => void;

  /**
   * Reset store to initial state (for logout/tenant switch)
   */
  _reset: () => void;

  /**
   * Sync timeline for a specific order (called after fetching events)
   * Replaces the timeline with server-provided events
   */
  _syncOrderTimeline: (orderId: string, events: OrderEvent[]) => void;

  /**
   * Clear timeline sync request for an order (after sync completes)
   */
  _clearTimelineSyncRequest: (orderId: string) => void;
}

type ActiveOrdersStore = ActiveOrdersState & ActiveOrdersActions;

// ============================================================================
// Store Implementation
// ============================================================================

const initialState: ActiveOrdersState = {
  orders: new Map(),
  timelines: new Map(),
  ordersNeedingTimelineSync: new Set(),
  lastSequence: 0,
  connectionState: 'disconnected',
  isInitialized: false,
  serverEpoch: null,
};

export const useActiveOrdersStore = create<ActiveOrdersStore>((set, get) => ({
  // Initial state
  ...initialState,

  // ==================== Read-Only Queries ====================

  getOrder: (orderId: string) => {
    return get().orders.get(orderId);
  },

  getActiveOrders: () => {
    const orders = get().orders;
    return Array.from(orders.values()).filter(
      (order) => order.status === 'ACTIVE'
    );
  },

  getOrderByTable: (tableId: number) => {
    const orders = get().orders;
    return Array.from(orders.values()).find(
      (order) => order.table_id === tableId && order.status === 'ACTIVE'
    );
  },

  getAllOrders: () => {
    return Array.from(get().orders.values());
  },

  hasActiveOrderOnTable: (tableId: number) => {
    const orders = get().orders;
    return Array.from(orders.values()).some(
      (order) => order.table_id === tableId && order.status === 'ACTIVE'
    );
  },

  // ==================== Internal Methods (Server Authority) ====================

  _applyOrderSync: (event: OrderEvent, snapshot: OrderSnapshot) => {
    const state = get();
    const orderId = snapshot.order_id;
    const localSnapshot = state.orders.get(orderId);
    let needsTimelineSync = false;

    // 检测订单级别的 sequence gap
    if (localSnapshot) {
      const localSeq = localSnapshot.last_sequence;
      const serverSeq = snapshot.last_sequence;
      const gap = serverSeq - localSeq;

      if (gap > MAX_SEQUENCE_GAP) {
        // Gap 过大，标记需要 timeline 补全
        logger.warn(
          `Large sequence gap for order ${orderId}: local=${localSeq} -> server=${serverSeq} (missed ${gap - 1} events, triggering sync)`,
          { component: 'OrderSync' }
        );
        needsTimelineSync = true;
      } else if (gap > 1) {
        // 小 gap，记录警告但不触发补全
        logger.warn(
          `Sequence gap for order ${orderId}: local=${localSeq} -> server=${serverSeq} (missed ${gap - 1} events)`,
          { component: 'OrderSync' }
        );
      }
    } else if (event.sequence > 1) {
      // 新订单但首个事件不是 sequence=1，说明错过了前面的事件
      logger.warn(
        `New order ${orderId} first event is sequence=${event.sequence}, triggering sync`,
        { component: 'OrderSync' }
      );
      needsTimelineSync = true;
    }

    set((prevState) => {
      // 服务端权威：直接替换快照
      const orders = new Map(prevState.orders);
      orders.set(orderId, snapshot);

      // 追加 event 到 timeline（用于 UI 渲染操作记录）
      const timelines = new Map(prevState.timelines);
      const existingTimeline = timelines.get(orderId) || [];

      // 去重：检查 event_id 是否已存在
      if (!existingTimeline.some((e) => e.event_id === event.event_id)) {
        // 插入并保持 sequence 顺序
        // 优化：事件通常按顺序到达，先检查末尾
        const newTimeline = [...existingTimeline];
        if (
          existingTimeline.length === 0 ||
          event.sequence > existingTimeline[existingTimeline.length - 1].sequence
        ) {
          // 常见情况：新事件序列号最大，直接追加
          newTimeline.push(event);
        } else {
          // 少见情况：乱序到达，找到正确位置插入
          const insertIdx = newTimeline.findIndex((e) => e.sequence > event.sequence);
          newTimeline.splice(insertIdx === -1 ? newTimeline.length : insertIdx, 0, event);
        }
        timelines.set(orderId, newTimeline);
      }

      // 标记需要 timeline 补全的订单
      const ordersNeedingTimelineSync = new Set(prevState.ordersNeedingTimelineSync);
      if (needsTimelineSync) {
        ordersNeedingTimelineSync.add(orderId);
      }

      return {
        ...prevState,
        orders,
        timelines,
        ordersNeedingTimelineSync,
        lastSequence: Math.max(prevState.lastSequence, snapshot.last_sequence),
      };
    });
  },

  _fullSync: (orders: OrderSnapshot[], serverSequence: number, serverEpoch?: string, events?: OrderEvent[]) => {
    set((state) => {
      const newOrders = new Map<string, OrderSnapshot>();

      for (const order of orders) {
        newOrders.set(order.order_id, order);
      }

      // 从 events 构建 timelines
      const newTimelines = new Map<string, OrderEvent[]>();
      if (events && events.length > 0) {
        for (const event of events) {
          const orderId = event.order_id;
          // 只为活跃订单构建 timeline
          if (newOrders.has(orderId)) {
            const timeline = newTimelines.get(orderId) || [];
            timeline.push(event);
            newTimelines.set(orderId, timeline);
          }
        }
      }

      return {
        ...state,
        orders: newOrders,
        timelines: newTimelines,
        ordersNeedingTimelineSync: new Set(), // Full sync 清除所有补全请求
        lastSequence: serverSequence,
        isInitialized: true,
        connectionState: 'connected',
        // Update epoch if provided
        serverEpoch: serverEpoch ?? state.serverEpoch,
      };
    });
  },

  _setConnectionState: (connectionState: OrderConnectionState) => {
    set({ connectionState });
  },

  _setInitialized: (isInitialized: boolean) => {
    set({ isInitialized });
  },

  _reset: () => {
    set(initialState);
  },

  _syncOrderTimeline: (orderId: string, events: OrderEvent[]) => {
    set((state) => {
      const timelines = new Map(state.timelines);
      // 替换该订单的 timeline，按 sequence 排序
      const sortedEvents = [...events].sort((a, b) => a.sequence - b.sequence);
      timelines.set(orderId, sortedEvents);

      // 清除补全请求标记
      const ordersNeedingTimelineSync = new Set(state.ordersNeedingTimelineSync);
      ordersNeedingTimelineSync.delete(orderId);

      return {
        ...state,
        timelines,
        ordersNeedingTimelineSync,
      };
    });
  },

  _clearTimelineSyncRequest: (orderId: string) => {
    set((state) => {
      const ordersNeedingTimelineSync = new Set(state.ordersNeedingTimelineSync);
      ordersNeedingTimelineSync.delete(orderId);
      return { ...state, ordersNeedingTimelineSync };
    });
  },
}));

// ============================================================================
// Selectors (for optimized React re-renders)
// ============================================================================

/**
 * Select timeline for a specific order
 */
export const useOrderTimeline = (orderId: string | null | undefined) =>
  useActiveOrdersStore((state) =>
    orderId ? state.timelines.get(orderId) || [] : []
  );

// ============================================================================
// Convenience Selectors
// ============================================================================

/**
 * Returns all held (active) orders
 */
export const useHeldOrders = () => {
  return useActiveOrdersStore(useShallow((state) => state.getActiveOrders()));
};

/**
 * Count of non-retail held orders
 */
export const useHeldOrdersCount = () =>
  useActiveOrdersStore((state) =>
    state.getActiveOrders().filter((o) => o.is_retail !== true).length
  );
