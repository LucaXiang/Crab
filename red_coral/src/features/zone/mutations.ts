import { createTauriClient } from '@/infrastructure/api';

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
}

/**
 * Update an existing zone
 */
export async function updateZone(id: number, input: UpdateZoneInput): Promise<void> {
  await getApi().updateZone(id, {
    name: input.name,
    description: input.description,
  });
}

/**
 * Delete a zone
 */
export async function deleteZone(id: number): Promise<void> {
  await getApi().deleteZone(id);
}
