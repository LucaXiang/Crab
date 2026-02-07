import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Zone } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

export const useZoneStore = createResourceStore<Zone>(
  'zone',
  () => getApi().listZones()
);

// Convenience hooks
export const useZones = () => useZoneStore((state) => state.items);
export const useZonesLoading = () => useZoneStore((state) => state.isLoading);
export const useZoneById = (id: number) =>
  useZoneStore((state) => state.items.find((z) => z.id === id));
