export interface StoreDetail {
  id: number;
  entity_id: string;
  name?: string;
  address?: string;
  phone?: string;
  nif?: string;
  email?: string;
  website?: string;
  business_day_cutoff?: string;
  device_id: string;
  is_online: boolean;
  last_sync_at: number | null;
  registered_at: number;
  store_info: Record<string, unknown> | null;
}
