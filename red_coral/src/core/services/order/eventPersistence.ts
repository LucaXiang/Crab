/**
 * 事件持久化服务
 *
 * 负责将事件保存到数据库和从数据库加载事件
 */

import { invoke } from '@tauri-apps/api/core';
import { OrderEvent, OrderEventType, PaymentAddedEvent, OrderSplitEvent } from '@/core/domain/events/types';
import { HeldOrder } from '@/core/domain/types';
import { PaymentRecord } from '@/core/domain/types';
import { logger } from '@/utils/logger';


/**
 * 保存已完成/已作废订单到关系型表
 */
export const saveCompletedOrder = async (
  order: HeldOrder,
  events: OrderEvent[],
  payments?: PaymentRecord[],
): Promise<void> => {
  if (!('__TAURI__' in window)) {
    return;
  }

  const persistedOrder: HeldOrder = {
    ...order,
    endTime:
      order.endTime && order.endTime > 0
        ? order.endTime
        : (order.status && order.status !== 'ACTIVE' ? Date.now() : order.endTime),
  } as HeldOrder;

  // 若未显式传入 payments，则从事件中提取
  // 同时也需要提取分账事件 (ORDER_SPLIT) 作为支付记录
  const derivedPayments: PaymentRecord[] = payments && payments.length > 0
    ? payments
    : events
        .filter(ev => ev.type === OrderEventType.PAYMENT_ADDED || ev.type === OrderEventType.ORDER_SPLIT)
        .map((ev) => {
          if (ev.type === OrderEventType.PAYMENT_ADDED) {
            const p = (ev as PaymentAddedEvent).data.payment;
            return {
              id: p.id || `pay-${order.key}-${ev.timestamp}`,
              method: p.method,
              amount: Number(p.amount || 0),
              timestamp: Number(ev.timestamp || order.endTime || Date.now()),
              note: p.note,
              tendered: p.tendered,
              change: p.change,
            };
          } else {
            // ORDER_SPLIT
            const data = (ev as OrderSplitEvent).data;
            return {
              id: `split-${order.key}-${ev.timestamp}`,
              method: `Split ${data.paymentMethod}`,
              amount: Number(data.splitAmount || 0),
              timestamp: Number(ev.timestamp || order.endTime || Date.now()),
              tendered: data.tendered,
              change: data.change,
            };
          }
        });

  // Retry logic: try 3 times with exponential backoff
  let attempts = 0;
  const maxAttempts = 3;
  
  while (attempts < maxAttempts) {
    try {
      await invoke('save_order', {
        params: {
          order: persistedOrder,
          payments: derivedPayments,
          events: events || [],
        },
      });
      // Success
      return;
    } catch (error) {
      attempts++;
      logger.error(`Failed to save completed order (Attempt ${attempts}/${maxAttempts})`, error, { component: 'eventPersistence', action: 'saveCompletedOrder', orderKey: persistedOrder.key, attempt: attempts });

      if (attempts >= maxAttempts) {
        // Final failure
        // In a real app, we might want to save to a "failed_sync_queue" in localStorage here
        logger.error(`Giving up on saving order after ${maxAttempts} attempts`, error, { component: 'eventPersistence', action: 'saveCompletedOrder', orderKey: persistedOrder.key });
      } else {
        // Wait before retry (500ms, 1000ms, etc.)
        await new Promise(resolve => setTimeout(resolve, 500 * Math.pow(2, attempts - 1)));
      }
    }
  }
};

// 事件读取与批量持久化均移除：活跃订单仅在本地管理，历史订单回看使用后端的历史接口
