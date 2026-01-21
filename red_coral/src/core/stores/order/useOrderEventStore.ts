/**
 * 基于事件溯源的订单管理 Store
 *
 * 所有订单操作通过发射事件实现，订单状态通过事件重放计算
 */

import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { HeldOrder, CartItem, PaymentRecord } from '@/core/domain/types';
import { OrderEvent, OrderEventType, createEvent, ItemChanges } from '@/core/domain/events';
import { reduceOrderEvents, createEmptyOrder } from '@/core/services/order/eventReducer';
import { logger } from '@/utils/logger';
import { reportError } from '@/utils/reportError';
import { useUIStore } from '@/core/stores/ui/useUIStore';
// 按新存储策略：活跃订单仅使用 localStorage；完成/作废订单发送到 Tauri 端关系型库

interface OrderEventStore {
  // 事件存储（按订单key分组）
  eventsByOrder: Record<string, OrderEvent[]>;

  // 计算后的订单状态缓存
  ordersCache: Record<string, HeldOrder>;

  // ============ 订单生命周期操作 ============

  /**
   * 开台
   */
  openTable: (params: {
    table_id: string;
    table_name: string;
    guest_count: number;
    zone_id?: string;
    zone_name?: string;
    surcharge?: { type: 'percentage' | 'fixed'; amount: number; name?: string };
    receipt_number?: string;
  }) => void;

  /**
   * 结单
   */
  completeOrder: (orderKey: string, receiptNumber: string) => void;

  /**
   * 作废订单
   */
  voidOrder: (orderKey: string, reason?: string) => void;

  /**
   * 恢复订单
   */
  restoreOrder: (orderKey: string, reason?: string) => void;

  // ============ 商品操作 ============

  /**
   * 加菜
   */
  addItems: (orderKey: string, items: CartItem[]) => void;

  /**
   * 修改菜品
   */
  modifyItem: (
    orderKey: string,
    instanceId: string,
    changes: ItemChanges,
    options?: { userId?: string }
  ) => void;

  /**
   * 移除菜品
   */
  removeItem: (orderKey: string, instanceId: string, reason?: string, options?: { userId?: string, quantity?: number }) => void;

  /**
   * 恢复菜品
   */
  restoreItem: (orderKey: string, instanceId: string) => void;

  // ============ Table Management ============
  mergeOrder: (targetOrder: HeldOrder, sourceOrder: HeldOrder) => void;
  moveOrder: (sourceOrderKey: string, targetTable: { id: string, name: string, zoneId?: string, zone_name?: string }) => void;


  // ============ 支付操作 ============

  /**
   * 添加支付记录
   */
  addPayment: (
    orderKey: string,
    payment: PaymentRecord
  ) => void;

  /**
   * 取消支付记录
   */
  cancelPayment: (orderKey: string, paymentId: string, reason?: string) => void;

  updateOrderInfo: (
    orderKey: string,
    info: {
      receiptNumber?: string;
      guestCount?: number;
      tableName?: string;
      is_pre_payment?: boolean;
    }
  ) => void;

  addSplitEvent: (
    orderKey: string,
    data: {
      split_amount: number;
      items: {
        instance_id: string;
        name: string;
        quantity: number;
        price: number;
        selected_options?: import('@/core/domain/types').ItemAttributeSelection[];
      }[];
      payment_method: string;
      tendered?: number;
      change?: number;
    }
  ) => void;

  // ============ 查询方法 ============

  /**
   * 获取订单当前状态
   */
  getOrder: (orderKey: string) => HeldOrder | undefined;

  /**
   * 获取所有活跃订单
   */
  getActiveOrders: () => HeldOrder[];

  /**
   * 获取订单的所有事件
   */
  getOrderEvents: (orderKey: string) => OrderEvent[];

  /**
   * 重新计算订单状态（从事件重放）
   */
  recomputeOrder: (orderKey: string) => void;

  /**
   * 从 localStorage 恢复活跃订单
   */
  hydrateActiveFromLocalStorage: () => void;

  // ============ 内部方法 ============

  /**
   * 添加事件并更新缓存
   */
  _addEvent: (orderKey: string, event: OrderEvent) => void;

  _persistOrder: (orderKey: string, order: HeldOrder, events: OrderEvent[]) => void;
}

interface OrderEventStoreState {
  eventsByOrder: Record<string, OrderEvent[]>;
  ordersCache: Record<string, HeldOrder>;
  isInitialized: boolean;
}

export const useOrderEventStore = create<OrderEventStore & OrderEventStoreState>((set, get) => ({
  eventsByOrder: {},
  ordersCache: {},
  isInitialized: false,

  // ============ 订单生命周期操作 ============

  openTable: (params) => {
    const orderKey = params.table_id;
    
    // Check if we have a finished order in memory that needs clearing
    // If the previous order on this table was completed/voided, we must clear the event history
    // before starting a new one, otherwise events will accumulate indefinitely.
    const existingOrder = get().getOrder(orderKey);
    if (existingOrder && existingOrder.status !== 'ACTIVE') {
        set((state) => ({
            eventsByOrder: {
                ...state.eventsByOrder,
                [orderKey]: []
            }
        }));
    }

    const event = createEvent(OrderEventType.TABLE_OPENED, params);

    // 如果订单已存在，这是重新开台
    get()._addEvent(orderKey, event);
  },

  completeOrder: (orderKey, receiptNumber) => {
    const order = get().getOrder(orderKey);
    if (!order) return;

    const event = createEvent(OrderEventType.ORDER_COMPLETED, {
      receipt_number: receiptNumber,
      final_total: order.total,
    });

    get()._addEvent(orderKey, event);

    // Retail Label Print (Print on Complete)
    const is_retail = orderKey.startsWith('RETAIL-') || order.is_retail === true;
    if (is_retail) {
      import('@/infrastructure/label/LabelPrintService').then(async ({ LabelPrintService }) => {
        const { isLabelPrintEnabled } = useUIStore.getState();
        if (isLabelPrintEnabled) {
          try {
            // Use the order state with the final receipt number
            const orderToPrint = { ...order, receiptNumber };
            await LabelPrintService.printOrderLabels(orderToPrint);
          } catch (e) {
            console.error('Retail label print failed', e);
            reportError('Retail label print failed', e as any, 'useOrderEventStore:completeOrder', {
              extras: {
                order_key: orderKey,
                receipt_number: receiptNumber
              }
            });
          }
        }
      });
    }
  },

  voidOrder: (orderKey, reason) => {
    const event = createEvent(OrderEventType.ORDER_VOIDED, { reason });
    get()._addEvent(orderKey, event);
  },

  restoreOrder: (orderKey, reason) => {
    const event = createEvent(OrderEventType.ORDER_RESTORED, { reason });
    get()._addEvent(orderKey, event);
  },

  // ============ 商品操作 ============

  addItems: (orderKey, items) => {
    // Ideally openTable should be called first
    // Even if items merge, we now emit ITEMS_ADDED to preserve the user intent for auditing.
    // The Event Reducer will handle the merging logic.

    // Check if current order has is_pre_payment set to true
    const currentOrder = get().getOrder(orderKey);
    const is_pre_payment = currentOrder?.is_pre_payment === true;

    const event = createEvent(OrderEventType.ITEMS_ADDED, { 
      items,
      ...(is_pre_payment ? { prePaymentReset: true } : {})
    });
    get()._addEvent(orderKey, event);

    try {
      const orderAfterAdd = get().getOrder(orderKey);
      const is_retail = orderKey.startsWith('RETAIL-') || orderAfterAdd?.is_retail === true;
      // Kitchen print is now handled server-side via order events
      if (orderAfterAdd && !is_retail) {
        // Label Print (client-side for now)
        import('@/infrastructure/label/LabelPrintService').then(async ({ LabelPrintService }) => {
          const { isLabelPrintEnabled } = useUIStore.getState();
          if (isLabelPrintEnabled) {
            try {
              await LabelPrintService.printItemsLabels(orderAfterAdd, items);
            } catch (e) {
              console.error('Auto label print failed', e);
              // Report error to user/logging system
              reportError('Auto label print failed', e as any, 'useOrderEventStore:addItems', {
                extras: {
                  order_key: orderKey,
                  item_count: items.length
                }
              });
            }
          }
        });
      }
    } catch {}
  },

  modifyItem: (orderKey, instanceId, changes, options) => {
    const order = get().getOrder(orderKey);
    let item = order?.items.find(i => i.instance_id === instanceId);
    if (!item && instanceId && instanceId.startsWith('item-')) {
      const idxStr = instanceId.replace('item-', '');
      const idx = parseInt(idxStr);
      if (!isNaN(idx)) {
        item = order?.items[idx];
      }
    }
    
    const realChanges: Partial<ItemChanges> = {};
    const previousValues: Partial<ItemChanges> = {};

    if (item) {
        type ChangeKey = keyof ItemChanges;
        Object.keys(changes).forEach(k => {
          const key = k as ChangeKey;
          const newValue = changes[key];
          const oldValue = (item as Partial<ItemChanges>)[key];
          
          // Normalize for comparison
          let isDifferent = false;
          
          if (typeof newValue === 'number') {
            const nOld = (oldValue === undefined || oldValue === null) ? 0 : Number(oldValue);
            const nNew = (newValue === undefined || newValue === null) ? 0 : Number(newValue);
            if (Math.abs(nOld - nNew) > 0.0001) {
              isDifferent = true;
            }
          } else {
            if (newValue !== oldValue) {
              isDifferent = true;
            }
          }

          if (isDifferent) {
            // Special handling for price/originalPrice
            // If this is the 'price' field, only include it if it's different from originalPrice
            // OR if originalPrice itself is changing.
            // The issue is ItemEditModal sends both price and originalPrice as the new base price.
            // If originalPrice hasn't changed, then changes.price (base) might differ from item.price (final),
            // but we should ignore this unless originalPrice is also changing.
            
            if (key === 'price') {
              const originalPriceChanged = changes.original_price !== undefined && 
                  changes.original_price !== item?.original_price;
              
              if (!originalPriceChanged) {
                // If originalPrice didn't change, ignore 'price' update (it's just a base price reset)
                return;
              }
            }

            // Ignore original_price normalization unless price truly changed
            if (key === 'original_price') {
              const priceChanged = changes.price !== undefined && changes.price !== item?.price;
              if (!priceChanged) {
                return;
              }
            }

            switch (key) {
              case 'price':
                if (typeof newValue === 'number') {
                  realChanges.price = newValue;
                  if (typeof oldValue === 'number') previousValues.price = oldValue;
                }
                break;
              case 'original_price':
                if (typeof newValue === 'number') {
                  realChanges.original_price = newValue;
                  if (typeof oldValue === 'number') previousValues.original_price = oldValue;
                }
                break;
              case 'quantity':
                if (typeof newValue === 'number') {
                  realChanges.quantity = newValue;
                  if (typeof oldValue === 'number') previousValues.quantity = oldValue;
                }
                break;
              case 'discount_percent':
                if (typeof newValue === 'number') {
                  realChanges.discount_percent = newValue;
                  if (typeof oldValue === 'number') previousValues.discount_percent = oldValue;
                }
                break;
              case 'surcharge':
                if (typeof newValue === 'number') {
                  realChanges.surcharge = newValue;
                  if (typeof oldValue === 'number') previousValues.surcharge = oldValue;
                }
                break;
              case 'note':
                if (typeof newValue === 'string') {
                  realChanges.note = newValue;
                  if (typeof oldValue === 'string') previousValues.note = oldValue;
                }
                break;
              case 'selected_options':
                if (Array.isArray(newValue)) {
                  realChanges.selected_options = newValue;
                  if (Array.isArray(oldValue)) previousValues.selected_options = oldValue;
                }
                break;
            }
          }
        });
    }

    if (Object.keys(realChanges).length === 0) return;

    const event = createEvent(OrderEventType.ITEM_MODIFIED, {
      instance_id: instanceId,
      item_name: item?.name,
      external_id: item?.external_id ? String(item.external_id) : undefined,
      changes: realChanges,
      previousValues: Object.keys(previousValues).length > 0 ? previousValues : undefined
    }, { userId: options?.userId });
    get()._addEvent(orderKey, event);
  },

  removeItem: (orderKey, instanceId, reason, options) => {
    const order = get().getOrder(orderKey);
    const item = order?.items.find(i => i.instance_id === instanceId);
    const event = createEvent(OrderEventType.ITEM_REMOVED, {
      instance_id: instanceId,
      item_name: item?.name,
      external_id: item?.external_id ? String(item.external_id) : undefined,
      quantity: options?.quantity,
      reason,
    }, { userId: options?.userId });
    get()._addEvent(orderKey, event);
  },

  restoreItem: (orderKey, instanceId) => {
    const event = createEvent(OrderEventType.ITEM_RESTORED, { instance_id: instanceId });
    get()._addEvent(orderKey, event);
  },

  // ============ 支付操作 ============

  addPayment: (orderKey, payment) => {
    const event = createEvent(OrderEventType.PAYMENT_ADDED, {
      payment,
    });
    get()._addEvent(orderKey, event);
  },

  cancelPayment: (orderKey, paymentId, reason) => {
    const event = createEvent(OrderEventType.PAYMENT_CANCELLED, {
      payment_id: paymentId,
      reason,
    });
    get()._addEvent(orderKey, event);
  },

  updateOrderInfo: (orderKey, info) => {
    const event = createEvent(OrderEventType.ORDER_INFO_UPDATED, info);
    get()._addEvent(orderKey, event);
  },

  addSplitEvent: (orderKey, data) => {
    const summary = `Split Bill: ${data.items.map(i => `${i.name} x${i.quantity}`).join(', ')}, Paid: ${data.split_amount.toFixed(2)}`;
    const event = createEvent(OrderEventType.ORDER_SPLIT, data, {
        title: 'Split Payment',
        summary
    });
    get()._addEvent(orderKey, event);
  },

  // ============ Table Management ============

  mergeOrder: (targetOrder, sourceOrder) => {
    if (!sourceOrder) return;

    const targetKey = targetOrder.key || String(targetOrder.table_id || '');
    const sourceKey = sourceOrder.key || String(sourceOrder.table_id || '');

    // 1. Add merged items to target
    const mergeEvent = createEvent(OrderEventType.ORDER_MERGED, {
      source_table_id: sourceKey,
      source_table_name: sourceOrder.receipt_number || sourceKey,
      items: sourceOrder.items.filter(i => !i._removed)
    });
    get()._addEvent(targetKey, mergeEvent);

    // 2. Mark source order as merged out (new status for audit)
    const mergedOutEvent = createEvent(OrderEventType.ORDER_MERGED_OUT, {
      target_table_id: targetKey,
      target_table_name: targetOrder.table_name || targetOrder.receipt_number || targetKey,
      reason: `Merged to ${targetOrder.receipt_number || targetKey}`,
    });
    get()._addEvent(sourceKey, mergedOutEvent);
  },

  moveOrder: (sourceOrderKey, targetTable) => {
    const sourceOrder = get().getOrder(sourceOrderKey);
    if (!sourceOrder) return;

    // Get source events for item history
    const sourceEvents = get().getOrderEvents(sourceOrderKey);

    // Find the original TABLE_OPENED event to get the original start time
    const tableOpenedEvent = sourceEvents.find(e => e.type === OrderEventType.TABLE_OPENED);

    // Create a new order on target table with TABLE_REASSIGNED event
    // This preserves event immutability - we don't modify the original TABLE_OPENED event
    const targetOrderKey = targetTable.id;

    // 1. First, open the target table (creates base order state)
    const openEvent = createEvent(OrderEventType.TABLE_OPENED, {
      table_id: targetTable.id,
      table_name: targetTable.name,
      zone_id: targetTable.zoneId,
      zone_name: targetTable.zone_name,
      guest_count: sourceOrder.guest_count,
    });
    get()._addEvent(targetOrderKey, openEvent);

    // 2. Add TABLE_REASSIGNED event to update table info and transfer items
    // This event handles the table change without mutating original events
    const reassignEvent = createEvent(OrderEventType.TABLE_REASSIGNED, {
      source_table_id: sourceOrderKey,
      source_table_name: sourceOrder.table_name,
      source_zone_id: sourceOrder.zone_name,
      source_zone_name: sourceOrder.zone_name,
      target_table_id: targetTable.id,
      target_table_name: targetTable.name,
      target_zone_id: targetTable.zoneId,
      target_zone_name: targetTable.zone_name,
      original_start_time: tableOpenedEvent?.timestamp || sourceOrder.start_time,
      items: sourceOrder.items.filter(i => !i._removed),
    });
    get()._addEvent(targetOrderKey, reassignEvent);

    // 3. Close source order as MOVED
    const moveOutEvent = createEvent(OrderEventType.ORDER_MOVED_OUT, {
      target_table_id: targetTable.id,
      target_table_name: targetTable.name,
    });
    get()._addEvent(sourceOrderKey, moveOutEvent);
  },



  // ============ 查询方法 ============

  getOrder: (orderKey) => {
    return get().ordersCache[orderKey];
  },

  getActiveOrders: () => {
    const cache = get().ordersCache;
    return Object.values(cache).filter((order) => order.status === 'ACTIVE');
  },

  getOrderEvents: (orderKey) => {
    return get().eventsByOrder[orderKey] || [];
  },

  recomputeOrder: (orderKey) => {
    const events = get().getOrderEvents(orderKey);
    if (events.length === 0) return;

    const order = reduceOrderEvents(events, createEmptyOrder(orderKey));

    set((state) => ({
      ordersCache: {
        ...state.ordersCache,
        [orderKey]: order,
      },
    }));
  },

  hydrateActiveFromLocalStorage: () => {
    try {
      const listRaw = localStorage.getItem('pos-active-orders');
      if (!listRaw) return;
      let keys: string[] = [];
      try { keys = JSON.parse(listRaw) as string[]; } catch {}
      const orders: Record<string, HeldOrder> = {};
      const events: Record<string, OrderEvent[]> = {};
      

      keys.forEach((k) => {
        // Auto-cleanup: Remove ALL RETAIL orders from storage
        if (k.startsWith('RETAIL-')) {
            localStorage.removeItem(`pos-active-order:${k}`);
            localStorage.removeItem(`pos-active-events:${k}`);
            return; // Skip loading
        }

        const raw = localStorage.getItem(`pos-active-order:${k}`);
        let valid = false;
        if (raw) {
          try {
            const o = JSON.parse(raw) as HeldOrder;
            // Validate: Only load ACTIVE orders. If an order was completed but cleanup failed,
            // we should not load it as active.
            if (o.status === 'ACTIVE') {
                orders[k] = o;
                valid = true;
            }
          } catch {}
        }
        
        if (valid) {
            const evRaw = localStorage.getItem(`pos-active-events:${k}`);
            if (evRaw) {
            try {
                const evs = JSON.parse(evRaw) as OrderEvent[];
                events[k] = evs;
                if (evs && evs.length > 0) {
                  const rebuilt = reduceOrderEvents(evs, createEmptyOrder(k));
                  orders[k] = rebuilt;
                  const lightOrder = { ...rebuilt, timeline: [] } as HeldOrder;
                  localStorage.setItem(`pos-active-order:${k}`, JSON.stringify(lightOrder));
                }
            } catch {}
            }
        } else {
            // Cleanup invalid/orphan keys from localStorage
            localStorage.removeItem(`pos-active-order:${k}`);
            localStorage.removeItem(`pos-active-events:${k}`);
        }
      });
      
      // Self-Healing: Detect and remove ghost orders resulting from interrupted moves
      // If Order B has a MOVED event coming from Order A, and Order A is still ACTIVE,
      // it means the move transaction was interrupted. Order A should be considered closed (moved).
      const activeOrders = Object.values(orders);
      const moveEvents = activeOrders
        .flatMap(o => {
          const orderKey = o.key || String(o.table_id || '');
          return orderKey ? (events[orderKey] || []) : [];
        })
        .filter(e => e.type === OrderEventType.ORDER_MOVED);

      moveEvents.forEach(e => {
          // e.data has sourceTableId
          const sourceId = (e.data as any).sourceTableId;
          if (sourceId && orders[sourceId] && orders[sourceId].status === 'ACTIVE') {
              logger.warn(`Detected ghost order ${sourceId} (already moved to ${(e.data as any).targetTableId}), auto-closing during hydration.`, { component: 'OrderEventStore' });
              
              // 1. Remove from memory
              delete orders[sourceId];
              delete events[sourceId];
              
              // 2. Remove from storage
              localStorage.removeItem(`pos-active-order:${sourceId}`);
              localStorage.removeItem(`pos-active-events:${sourceId}`);
          }
      });

      // Update the index list to only contain valid keys
      const validKeys = Object.keys(orders);
      if (validKeys.length !== keys.length) {
          localStorage.setItem('pos-active-orders', JSON.stringify(validKeys));
      }

      set((state) => ({
        ordersCache: { ...state.ordersCache, ...orders },
        eventsByOrder: { ...state.eventsByOrder, ...events },
        isInitialized: true,
      }));
    } catch (e) {
      logger.error('Failed to hydrate active orders from localStorage', e, { component: 'OrderEventStore', action: 'hydrateActiveFromLocalStorage' });
    }
  },

  // ============ 内部方法 ============

  _persistOrder: (orderKey: string, order: HeldOrder, events: OrderEvent[]) => {
    if (orderKey.startsWith('RETAIL-')) return;
    try {
      const key = `pos-active-order:${orderKey}`;
      // Remove timeline to save space
      const lightOrder = { ...order, timeline: [] };
      localStorage.setItem(key, JSON.stringify(lightOrder));

      const listKey = 'pos-active-orders';
      const listRaw = localStorage.getItem(listKey);
      let keys: string[] = [];
      if (listRaw) {
        try { keys = JSON.parse(listRaw) as string[]; } catch {}
      }
      if (!keys.includes(orderKey)) {
        keys.push(orderKey);
        localStorage.setItem(listKey, JSON.stringify(keys));
      }

      localStorage.setItem(`pos-active-events:${orderKey}`, JSON.stringify(events));
    } catch (e) {
      logger.error('Failed to persist active order', e, { component: 'OrderEventStore', action: '_persistOrder', orderKey });
    }
  },

  _addEvent: (orderKey, event) => {
    set((state) => {
      // 添加事件到事件列表
      const currentEvents = state.eventsByOrder[orderKey] || [];
      const newEvents = [...currentEvents, event];

      // 重新计算订单状态
      const order = reduceOrderEvents(newEvents, createEmptyOrder(orderKey));

      // 按新策略持久化：未完成订单仅写入 localStorage；完成/作废写入关系型数据库
      if (order.status === 'ACTIVE') {
        get()._persistOrder(orderKey, order, newEvents);
      } else {
        try {
          localStorage.removeItem(`pos-active-order:${orderKey}`);
          localStorage.removeItem(`pos-active-events:${orderKey}`);
          const listKey = 'pos-active-orders';
          const listRaw = localStorage.getItem(listKey);
          let keys: string[] = [];
          if (listRaw) {
            try { keys = JSON.parse(listRaw) as string[]; } catch {}
            keys = keys.filter(k => k !== orderKey);
            localStorage.setItem(listKey, JSON.stringify(keys));
          }
        } catch {}
        import('@/core/services/order/eventPersistence').then(async ({ saveCompletedOrder }) => {
          try {
            // 保存完成/作废订单（含全部事件与支付）
            await saveCompletedOrder(order, newEvents);
          } catch (error) {
            logger.error('Failed to persist completed/void order data', error, { component: 'OrderEventStore', action: '_addEvent', orderKey });
          }
        });
      }

      return {
        eventsByOrder: {
          ...state.eventsByOrder,
          [orderKey]: newEvents,
        },
        ordersCache: {
          ...state.ordersCache,
          [orderKey]: order,
        },
      };
    });
  },
}));

// ============ Selectors ============

 

/**
 * 获取订单操作方法
 */
export const useOrderActions = () => {
  return useOrderEventStore(useShallow((state) => ({
    openTable: state.openTable,
    completeOrder: state.completeOrder,
    voidOrder: state.voidOrder,
    restoreOrder: state.restoreOrder,
    addItems: state.addItems,
    modifyItem: state.modifyItem,
    removeItem: state.removeItem,
    restoreItem: state.restoreItem,
    addPayment: state.addPayment,
    cancelPayment: state.cancelPayment,
    updateOrderInfo: state.updateOrderInfo,
  })));
};
