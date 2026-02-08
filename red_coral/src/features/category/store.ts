import { createCrudResourceStore } from '@/core/stores/factory/createResourceStore';
import { useShallow } from 'zustand/react/shallow';
import { createTauriClient } from '@/infrastructure/api';
import type { Category, CategoryCreate, CategoryUpdate } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

export const useCategoryStore = createCrudResourceStore<Category, CategoryCreate, CategoryUpdate>(
  'category',
  () => getApi().listCategories(),
  {
    create: (data) => getApi().createCategory(data),
    update: (id, data) => getApi().updateCategory(id, data),
    remove: (id) => getApi().deleteCategory(id),
  }
);

// Convenience hooks
export const useCategories = () => useCategoryStore((state) => state.items);
export const useCategoriesLoading = () => useCategoryStore((state) => state.isLoading);
export const useCategoryById = (id: number) =>
  useCategoryStore((state) => state.items.find((c) => c.id === id));

// CRUD action hooks
export const useCategoryActions = () => ({
  create: useCategoryStore.getState().create,
  update: useCategoryStore.getState().update,
  remove: useCategoryStore.getState().remove,
  fetchAll: useCategoryStore.getState().fetchAll,
});

// Virtual/Regular category selectors (useShallow for stable array references)
export const useVirtualCategories = () =>
  useCategoryStore(
    useShallow((state) =>
      state.items
        .filter((c) => c.is_virtual)
        .sort((a, b) => a.sort_order - b.sort_order)
    )
  );

export const useRegularCategories = () =>
  useCategoryStore(
    useShallow((state) =>
      state.items
        .filter((c) => !c.is_virtual)
        .sort((a, b) => a.sort_order - b.sort_order)
    )
  );

export const useCategoryByName = (name: string) =>
  useCategoryStore((state) => state.items.find((c) => c.name === name));

// Static getters for non-hook usage
export const getVirtualCategories = () =>
  useCategoryStore.getState().items
    .filter((c) => c.is_virtual)
    .sort((a, b) => a.sort_order - b.sort_order);

export const getRegularCategories = () =>
  useCategoryStore.getState().items
    .filter((c) => !c.is_virtual)
    .sort((a, b) => a.sort_order - b.sort_order);

export const getCategoryByName = (name: string) =>
  useCategoryStore.getState().items.find((c) => c.name === name);
