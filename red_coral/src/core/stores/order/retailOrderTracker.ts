/**
 * Retail Order Tracker
 *
 * 本地追踪当前客户端创建的零售订单，用于：
 * 1. 防止意外关闭程序导致零售订单丢失
 * 2. 启动时恢复未完成的零售订单
 */

const STORAGE_KEY = 'pos-pending-retail-order';

export interface PendingRetailOrder {
  orderId: string;
  createdAt: number;
}

/**
 * 保存待处理的零售订单ID
 */
export function savePendingRetailOrder(orderId: string): void {
  const data: PendingRetailOrder = {
    orderId,
    createdAt: Date.now(),
  };
  localStorage.setItem(STORAGE_KEY, JSON.stringify(data));
}

/**
 * 获取待处理的零售订单
 */
export function getPendingRetailOrder(): PendingRetailOrder | null {
  const data = localStorage.getItem(STORAGE_KEY);
  if (!data) return null;

  try {
    return JSON.parse(data) as PendingRetailOrder;
  } catch {
    return null;
  }
}

/**
 * 清除待处理的零售订单（完成或作废后调用）
 */
export function clearPendingRetailOrder(): void {
  localStorage.removeItem(STORAGE_KEY);
}
