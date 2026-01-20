import { createResourceStore } from '../factory/createResourceStore';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import type { Order, ApiResponse } from '@/core/domain/types/api';

/**
 * Order Store - 订单数据
 *
 * 订单数据量可能较大，按需加载。
 * 通常只加载 OPEN 状态的订单。
 */
async function fetchOrders(): Promise<Order[]> {
  // OrderListData wrapper
  const data = await invokeApi<{ orders: Order[] }>('list_orders');
  return data.orders;
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
