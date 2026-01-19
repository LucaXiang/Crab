import { createResourceStore } from '../factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Zone } from '@/core/domain/types/api';

const api = createTauriClient();

async function fetchZones(): Promise<Zone[]> {
  const response = await api.listZones();
  // Handle both formats: direct array or { data: { zones: [...] } }
  if (Array.isArray(response)) {
    return response;
  }
  if (response.data?.zones) {
    return response.data.zones;
  }
  throw new Error(response.message || 'Failed to fetch zones');
}

export const useZoneStore = createResourceStore<Zone & { id: string }>(
  'zone',
  fetchZones as () => Promise<(Zone & { id: string })[]>
);

// Convenience hooks
export const useZones = () => useZoneStore((state) => state.items);
export const useZonesLoading = () => useZoneStore((state) => state.isLoading);
export const useZoneById = (id: string) =>
  useZoneStore((state) => state.items.find((z) => z.id === id));
