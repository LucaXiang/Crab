import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Table } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

export const useTableStore = createResourceStore<Table>(
  'table',
  () => getApi().listTables()
);

// Convenience hooks
export const useTables = () => useTableStore((state) => state.items);
export const useTablesLoading = () => useTableStore((state) => state.isLoading);
export const useTableById = (id: number) =>
  useTableStore((state) => state.items.find((t) => t.id === id));
export const useTablesByZone = (zoneId: number) =>
  useTableStore((state) => state.items.filter((t) => t.zone_id === zoneId));
