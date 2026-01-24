import { createTauriClient } from '@/infrastructure/api';
import { useTableStore } from './store';

const api = createTauriClient();

export interface CreateTableInput {
  name: string;
  zone: string;
  capacity: number;
}

export interface UpdateTableInput {
  name?: string;
  zone?: string;
  capacity?: number;
  is_active?: boolean;
}

/**
 * Create a new table
 */
export async function createTable(input: CreateTableInput): Promise<void> {
  await api.createTable({
    name: input.name,
    zone: String(input.zone),
    capacity: Number(input.capacity),
  });
  // Refresh tables from server
  await useTableStore.getState().fetchAll(true);
}

/**
 * Update an existing table
 */
export async function updateTable(id: string, input: UpdateTableInput): Promise<void> {
  await api.updateTable(String(id), {
    name: input.name,
    zone: input.zone ? String(input.zone) : undefined,
    capacity: input.capacity ? Number(input.capacity) : undefined,
    is_active: input.is_active,
  });
  // Refresh tables from server
  await useTableStore.getState().fetchAll(true);
}

/**
 * Delete a table
 */
export async function deleteTable(id: string): Promise<void> {
  await api.deleteTable(String(id));
  // Refresh tables from server
  await useTableStore.getState().fetchAll(true);
}
