import { request } from './client';
import type { OrderSummary, OrderDetailResponse } from '@/core/types/order';

export function getOrders(
  token: string,
  storeId: number,
  page = 1,
  perPage = 20,
  status?: string,
): Promise<OrderSummary[]> {
  let path = `/api/tenant/stores/${storeId}/orders?page=${page}&per_page=${perPage}`;
  if (status) path += `&status=${status}`;
  return request('GET', path, undefined, token);
}

export function getOrderDetail(
  token: string,
  storeId: number,
  orderKey: string,
): Promise<OrderDetailResponse> {
  return request('GET', `/api/tenant/stores/${storeId}/orders/${orderKey}/detail`, undefined, token);
}
