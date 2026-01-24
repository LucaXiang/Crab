/**
 * Category Feature Module
 *
 * This module contains all category-related functionality:
 * - Store: Zustand store for category state management
 * - Mutations: CRUD operations for categories
 * - Components: CategoryManagement, CategoryForm, CategoryModal
 */

// Store exports
export {
  useCategoryStore,
  useCategories,
  useCategoriesLoading,
  useCategoryById,
  useCategoryActions,
  useVirtualCategories,
  useRegularCategories,
  useCategoryByName,
  getVirtualCategories,
  getRegularCategories,
  getCategoryByName,
} from './store';

// Mutation exports
export {
  createCategory,
  updateCategory,
  deleteCategory,
  loadCategoryAttributes,
  type CategoryFormData,
} from './mutations';

// Component exports
export { CategoryManagement } from './CategoryManagement';
export { CategoryForm } from './CategoryForm';
export { CategoryModal } from './CategoryModal';
