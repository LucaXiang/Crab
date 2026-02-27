import { request } from './client';
import type { StoreDetail, DeviceRecord } from '@/core/types/store';

export function getStores(token: string): Promise<StoreDetail[]> {
  return request('GET', '/api/tenant/stores', undefined, token);
}

export function updateStore(
  token: string,
  storeId: number,
  data: {
    alias?: string;
    name?: string;
    address?: string;
    phone?: string;
    nif?: string;
    email?: string;
    website?: string;
    business_day_cutoff?: string;
  },
): Promise<void> {
  return request('PATCH', `/api/tenant/stores/${storeId}`, data, token);
}

export function deleteStore(token: string, storeId: number): Promise<void> {
  return request('DELETE', `/api/tenant/stores/${storeId}`, undefined, token);
}

export function getStoreDevices(token: string, storeId: number): Promise<DeviceRecord[]> {
  return request('GET', `/api/tenant/stores/${storeId}/devices`, undefined, token);
}
