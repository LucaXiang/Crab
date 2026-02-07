import { createTauriClient } from '@/infrastructure/api';
import type { Tag } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

export interface CreateTagInput {
  name: string;
  color: string;
  display_order?: number;
}

export interface UpdateTagInput {
  name?: string;
  color?: string;
  display_order?: number;
}

/**
 * Create a new tag
 */
export async function createTag(data: CreateTagInput): Promise<Tag> {
  return await getApi().createTag({
    name: data.name,
    color: data.color,
    display_order: data.display_order,
  });
}

/**
 * Update an existing tag
 */
export async function updateTag(id: number, data: UpdateTagInput): Promise<Tag> {
  return await getApi().updateTag(id, data);
}

/**
 * Delete a tag
 */
export async function deleteTag(id: number): Promise<void> {
  await getApi().deleteTag(id);
}
