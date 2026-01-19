import { createResourceStore } from '../factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Table } from '@/infrastructure/api/types';

const api = createTauriClient();

async function fetchTables(): Promise<Table[]> {
  const response = await api.listTables();
  if (response.data?.tables) {
    return response.data.tables;
  }
  throw new Error(response.message || 'Failed to fetch tables');
}

export const useTableStore = createResourceStore<Table & { id: string }>(
  'table',
  fetchTables as () => Promise<(Table & { id: string })[]>
);

// Convenience hooks
export const useTables = () => useTableStore((state) => state.items);
export const useTablesLoading = () => useTableStore((state) => state.isLoading);
export const useTableById = (id: string) =>
  useTableStore((state) => state.items.find((t) => t.id === id));
export const useTablesByZone = (zoneId: string) =>
  useTableStore((state) => state.items.filter((t) => t.zone === zoneId));
