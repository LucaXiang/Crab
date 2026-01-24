import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Zone } from '@/core/domain/types/api';

const api = createTauriClient();

export const useZoneStore = createResourceStore<Zone & { id: string }>(
  'zone',
  () => api.listZones() as Promise<(Zone & { id: string })[]>
);

// Convenience hooks
export const useZones = () => useZoneStore((state) => state.items);
export const useZonesLoading = () => useZoneStore((state) => state.isLoading);
export const useZoneById = (id: string) =>
  useZoneStore((state) => state.items.find((z) => z.id === id));
