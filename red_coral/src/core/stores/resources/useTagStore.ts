import { createResourceStore } from '../factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Tag } from '@/infrastructure/api/types';

const api = createTauriClient();

async function fetchTags(): Promise<Tag[]> {
  const response = await api.listTags();
  if (response.data?.tags) {
    return response.data.tags;
  }
  throw new Error(response.message || 'Failed to fetch tags');
}

export const useTagStore = createResourceStore<Tag & { id: string }>(
  'tag',
  fetchTags as () => Promise<(Tag & { id: string })[]>
);

// Convenience hooks
export const useTags = () => useTagStore((state) => state.items);
export const useTagsLoading = () => useTagStore((state) => state.isLoading);
export const useTagById = (id: string) =>
  useTagStore((state) => state.items.find((t) => t.id === id));
