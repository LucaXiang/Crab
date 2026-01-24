import { createTauriClient } from '@/infrastructure/api';
import type { Tag } from '@/core/domain/types/api';

const api = createTauriClient();

export interface CreateTagInput {
  name: string;
  color: string;
  display_order?: number;
}

export interface UpdateTagInput {
  name?: string;
  color?: string;
  display_order?: number;
  is_active?: boolean;
}

/**
 * Create a new tag
 */
export async function createTag(data: CreateTagInput): Promise<Tag> {
  return await api.createTag({
    name: data.name,
    color: data.color,
    display_order: data.display_order,
  });
}

/**
 * Update an existing tag
 */
export async function updateTag(id: string, data: UpdateTagInput): Promise<Tag> {
  return await api.updateTag(id, data);
}

/**
 * Delete a tag
 */
export async function deleteTag(id: string): Promise<void> {
  await api.deleteTag(id);
}
