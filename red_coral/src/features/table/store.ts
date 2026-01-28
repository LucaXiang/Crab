import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Table } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

export const useTableStore = createResourceStore<Table & { id: string }>(
  'table',
  () => getApi().listTables() as Promise<(Table & { id: string })[]>
);

// Convenience hooks
export const useTables = () => useTableStore((state) => state.items);
export const useTablesLoading = () => useTableStore((state) => state.isLoading);
export const useTableById = (id: string) =>
  useTableStore((state) => state.items.find((t) => t.id === id));
export const useTablesByZone = (zoneId: string) =>
  useTableStore((state) => state.items.filter((t) => t.zone === zoneId));
