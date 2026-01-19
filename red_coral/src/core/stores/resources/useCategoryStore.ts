import { createCrudResourceStore } from '../factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Category } from '@/core/domain/types/api';

const api = createTauriClient();

// Category with guaranteed id
type CategoryEntity = Category & { id: string };

// Create category input type
interface CreateCategoryInput {
  name: string;
  sort_order?: number;
  is_active?: boolean;
}

// Update category input type
interface UpdateCategoryInput {
  name?: string;
  sort_order?: number;
  is_active?: boolean;
}

async function fetchCategories(): Promise<CategoryEntity[]> {
  const response = await api.listCategories();
  // Handle both formats: direct array or { data: { categories: [...] } }
  if (Array.isArray(response)) {
    return response as CategoryEntity[];
  }
  if (response.data?.categories) {
    return response.data.categories as CategoryEntity[];
  }
  throw new Error(response.message || 'Failed to fetch categories');
}

async function createCategory(data: CreateCategoryInput): Promise<CategoryEntity> {
  const response = await api.createCategory(data as any);
  if (response.data?.category) {
    return response.data.category as CategoryEntity;
  }
  throw new Error(response.message || 'Failed to create category');
}

async function updateCategory(id: string, data: UpdateCategoryInput): Promise<CategoryEntity> {
  const response = await api.updateCategory(id, data);
  if (response.data?.category) {
    return response.data.category as CategoryEntity;
  }
  throw new Error(response.message || 'Failed to update category');
}

async function deleteCategory(id: string): Promise<void> {
  const response = await api.deleteCategory(id);
  if (!response.data?.deleted && response.error_code) {
    throw new Error(response.message || 'Failed to delete category');
  }
}

export const useCategoryStore = createCrudResourceStore<CategoryEntity, CreateCategoryInput, UpdateCategoryInput>(
  'category',
  fetchCategories,
  {
    create: createCategory,
    update: updateCategory,
    remove: deleteCategory,
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
