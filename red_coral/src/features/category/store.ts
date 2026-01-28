import { createCrudResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Category, CategoryCreate, CategoryUpdate } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

// Category with guaranteed id
type CategoryEntity = Category & { id: string };

export const useCategoryStore = createCrudResourceStore<CategoryEntity, CategoryCreate, CategoryUpdate>(
  'category',
  () => getApi().listCategories() as Promise<CategoryEntity[]>,
  {
    create: (data) => getApi().createCategory(data) as Promise<CategoryEntity>,
    update: (id, data) => getApi().updateCategory(id, data) as Promise<CategoryEntity>,
    remove: (id) => getApi().deleteCategory(id),
  }
);

// Convenience hooks
export const useCategories = () => useCategoryStore((state) => state.items);
export const useCategoriesLoading = () => useCategoryStore((state) => state.isLoading);
export const useCategoryById = (id: string) =>
  useCategoryStore((state) => state.items.find((c) => c.id === id));

// CRUD action hooks
export const useCategoryActions = () => ({
  create: useCategoryStore.getState().create,
  update: useCategoryStore.getState().update,
  remove: useCategoryStore.getState().remove,
  fetchAll: useCategoryStore.getState().fetchAll,
});

// Virtual/Regular category selectors
export const useVirtualCategories = () =>
  useCategoryStore((state) =>
    state.items
      .filter((c) => c.is_virtual && c.is_active)
      .sort((a, b) => a.sort_order - b.sort_order)
  );

export const useRegularCategories = () =>
  useCategoryStore((state) =>
    state.items
      .filter((c) => !c.is_virtual)
      .sort((a, b) => a.sort_order - b.sort_order)
  );

export const useCategoryByName = (name: string) =>
  useCategoryStore((state) => state.items.find((c) => c.name === name));

// Static getters for non-hook usage
export const getVirtualCategories = () =>
  useCategoryStore.getState().items
    .filter((c) => c.is_virtual && c.is_active)
    .sort((a, b) => a.sort_order - b.sort_order);

export const getRegularCategories = () =>
  useCategoryStore.getState().items
    .filter((c) => !c.is_virtual)
    .sort((a, b) => a.sort_order - b.sort_order);

export const getCategoryByName = (name: string) =>
  useCategoryStore.getState().items.find((c) => c.name === name);
