import { createResourceStore } from '../factory/createResourceStore';
import { invoke } from '@tauri-apps/api/core';
import type { Order, ApiResponse } from '@/core/domain/types/api';

/**
 * Order Store - 订单数据
 *
 * 订单数据量可能较大，按需加载。
 * 通常只加载 OPEN 状态的订单。
 */
async function fetchOrders(): Promise<Order[]> {
  // 使用直接的 invoke 调用
  const response = await invoke<Order[] | ApiResponse<{ orders: Order[] }>>('list_orders');

  // 处理两种可能的响应格式
  if (Array.isArray(response)) {
    return response;
  }
  if (response.data?.orders) {
    return response.data.orders;
  }
  throw new Error('Failed to fetch orders');
}

export const useOrderStore = createResourceStore<Order & { id: string }>(
  'order',
  fetchOrders as () => Promise<(Order & { id: string })[]>
);

// Convenience hooks
export const useOrders = () => useOrderStore((state) => state.items);
export const useOrdersLoading = () => useOrderStore((state) => state.isLoading);
export const useOrderById = (id: string) =>
  useOrderStore((state) => state.items.find((o) => o.id === id));
export const useOpenOrders = () =>
  useOrderStore((state) => state.items.filter((o) => o.status === 'OPEN'));
