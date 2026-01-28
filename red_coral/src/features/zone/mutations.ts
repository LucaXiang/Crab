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
  is_active?: boolean;
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
export async function updateZone(id: string, input: UpdateZoneInput): Promise<void> {
  await getApi().updateZone(String(id), {
    name: input.name,
    description: input.description,
    is_active: input.is_active,
  });
  // Refresh zones from server
  await useZoneStore.getState().fetchAll(true);
}

/**
 * Delete a zone
 */
export async function deleteZone(id: string): Promise<void> {
  await getApi().deleteZone(String(id));
  // Refresh zones from server
  await useZoneStore.getState().fetchAll(true);
}
