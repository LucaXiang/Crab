import { request } from './client';
import type { StoreDetail } from '@/core/types/store';

export function getStores(token: string): Promise<StoreDetail[]> {
  return request('GET', '/api/tenant/stores', undefined, token);
}

export function updateStore(
  token: string,
  storeId: number,
  data: {
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
