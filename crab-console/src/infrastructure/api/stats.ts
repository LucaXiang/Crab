import { request } from './client';
import type { StoreOverview, DailyReportEntry, RedFlagsResponse } from '@/core/types/stats';

export function getTenantOverview(
  token: string,
  from: number,
  to: number,
): Promise<StoreOverview> {
  return request('GET', `/api/tenant/overview?from=${from}&to=${to}`, undefined, token);
}

export function getStoreOverview(
  token: string,
  storeId: number,
  from: number,
  to: number,
): Promise<StoreOverview> {
  return request('GET', `/api/tenant/stores/${storeId}/overview?from=${from}&to=${to}`, undefined, token);
}

export function getStats(
  token: string,
  storeId: number,
  from?: string,
  to?: string,
): Promise<DailyReportEntry[]> {
  let path = `/api/tenant/stores/${storeId}/stats?`;
  if (from) path += `from=${from}&`;
  if (to) path += `to=${to}&`;
  return request('GET', path, undefined, token);
}

export function getStoreRedFlags(
  token: string,
  storeId: number,
  from: number,
  to: number,
): Promise<RedFlagsResponse> {
  return request('GET', `/api/tenant/stores/${storeId}/red-flags?from=${from}&to=${to}`, undefined, token);
}
