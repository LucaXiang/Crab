import { createResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Tag } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

export const useTagStore = createResourceStore<Tag>(
  'tag',
  () => getApi().listTags()
);

// Convenience hooks
export const useTags = () => useTagStore((state) => state.items);
export const useTagsLoading = () => useTagStore((state) => state.isLoading);
export const useTagById = (id: number) =>
  useTagStore((state) => state.items.find((t) => t.id === id));
