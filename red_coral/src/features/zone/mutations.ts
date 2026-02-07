import { createTauriClient } from '@/infrastructure/api';
import { useZoneStore } from './store';

const getApi = () => createTauriClient();

export interface CreateZoneInput {
  name: string;
  description?: string;
}

export interface UpdateZoneInput {
  name?: string;
  description?: string;
}

/**
 * Create a new zone
 */
export async function createZone(input: CreateZoneInput): Promise<void> {
  await getApi().createZone({
    name: input.name,
    description: input.description,
  });
  // Refresh zones from server
  await useZoneStore.getState().fetchAll(true);
}

/**
 * Update an existing zone
 */
export async function updateZone(id: number, input: UpdateZoneInput): Promise<void> {
  await getApi().updateZone(id, {
    name: input.name,
    description: input.description,
  });
  // Refresh zones from server
  await useZoneStore.getState().fetchAll(true);
}

/**
 * Delete a zone
 */
export async function deleteZone(id: number): Promise<void> {
  await getApi().deleteZone(id);
  // Refresh zones from server
  await useZoneStore.getState().fetchAll(true);
}
