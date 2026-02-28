import { request } from './client';
import type {
  OrderSummary,
  OrderDetailResponse,
  CreditNoteSummary,
  ChainEntryItem,
  CreditNoteDetailResponse,
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
  orderKey: string,
): Promise<OrderDetailResponse> {
  return request('GET', `/api/tenant/stores/${storeId}/orders/${orderKey}/detail`, undefined, token);
}

export function getCreditNotes(
  token: string,
  storeId: number,
  orderKey: string,
): Promise<CreditNoteSummary[]> {
  return request('GET', `/api/tenant/stores/${storeId}/orders/${orderKey}/credit-notes`, undefined, token);
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
  sourceId: string,
): Promise<CreditNoteDetailResponse> {
  return request('GET', `/api/tenant/stores/${storeId}/credit-notes/${sourceId}`, undefined, token);
}
