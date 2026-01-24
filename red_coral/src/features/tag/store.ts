import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Tag } from '@/core/domain/types/api';

const api = createTauriClient();

export const useTagStore = createResourceStore<Tag & { id: string }>(
  'tag',
  () => api.listTags() as Promise<(Tag & { id: string })[]>
);

// Convenience hooks
export const useTags = () => useTagStore((state) => state.items);
export const useTagsLoading = () => useTagStore((state) => state.isLoading);
export const useTagById = (id: string) =>
  useTagStore((state) => state.items.find((t) => t.id === id));
