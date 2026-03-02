import { request } from './client';
import type {
  OrderSummary,
  OrderDetailResponse,
  CreditNoteSummary,
  ChainEntryItem,
  CreditNoteDetailResponse,
  AnulacionDetailResponse,
  UpgradeDetailResponse,
} from '@/core/types/order';

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
  orderId: number,
): Promise<OrderDetailResponse> {
  return request('GET', `/api/tenant/stores/${storeId}/orders/${orderId}/detail`, undefined, token);
}

export function getCreditNotes(
  token: string,
  storeId: number,
  orderId: number,
): Promise<CreditNoteSummary[]> {
  return request('GET', `/api/tenant/stores/${storeId}/orders/${orderId}/credit-notes`, undefined, token);
}

export function getChainEntries(
  token: string,
  storeId: number,
  page = 1,
  perPage = 20,
): Promise<ChainEntryItem[]> {
  return request('GET', `/api/tenant/stores/${storeId}/chain-entries?page=${page}&per_page=${perPage}`, undefined, token);
}

export function getCreditNoteDetail(
  token: string,
  storeId: number,
  sourceId: number,
): Promise<CreditNoteDetailResponse> {
  return request('GET', `/api/tenant/stores/${storeId}/credit-notes/${sourceId}`, undefined, token);
}

export function getAnulacionDetail(
  token: string,
  storeId: number,
  orderId: number,
): Promise<AnulacionDetailResponse> {
  return request('GET', `/api/tenant/stores/${storeId}/anulaciones/${orderId}`, undefined, token);
}

export function getUpgradeDetail(
  token: string,
  storeId: number,
  orderId: number,
): Promise<UpgradeDetailResponse> {
  return request('GET', `/api/tenant/stores/${storeId}/upgrades/${orderId}`, undefined, token);
}
