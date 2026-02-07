import { createTauriClient } from '@/infrastructure/api';
import { useTableStore } from './store';

const getApi = () => createTauriClient();

export interface CreateTableInput {
  name: string;
  zone_id: number;
  capacity: number;
}

export interface UpdateTableInput {
  name?: string;
  zone_id?: number;
  capacity?: number;
  is_active?: boolean;
}

/**
 * Create a new table
 */
export async function createTable(input: CreateTableInput): Promise<void> {
  await getApi().createTable({
    name: input.name,
    zone_id: input.zone_id,
    capacity: input.capacity,
  });
  // Refresh tables from server
  await useTableStore.getState().fetchAll(true);
}

/**
 * Update an existing table
 */
export async function updateTable(id: number, input: UpdateTableInput): Promise<void> {
  await getApi().updateTable(id, {
    name: input.name,
    zone_id: input.zone_id,
    capacity: input.capacity,
    is_active: input.is_active,
  });
  // Refresh tables from server
  await useTableStore.getState().fetchAll(true);
}

/**
 * Delete a table
 */
export async function deleteTable(id: number): Promise<void> {
  await getApi().deleteTable(id);
  // Refresh tables from server
  await useTableStore.getState().fetchAll(true);
}
