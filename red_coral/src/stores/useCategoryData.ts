/**
 * Category Data Store
 * Manages category data for the application
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface Category {
  id: string;
  name: string;
  parentId?: string;
  sortOrder?: number;
  isActive: boolean;
  icon?: string;
  color?: string;
}

export interface CategoryState {
  categories: Category[];
  activeCategoryId: string | null;
  isLoading: boolean;

  // Actions
  setCategories: (categories: Category[]) => void;
  addCategory: (category: Category) => void;
  updateCategory: (id: string, updates: Partial<Category>) => void;
  deleteCategory: (id: string) => void;
  setActiveCategory: (id: string | null) => void;
  setLoading: (loading: boolean) => void;
  reorderCategories: (activeId: string, overId: string) => void;
}

export const useCategoryData = create<CategoryState>()(
  persist(
    (set, get) => ({
      categories: [],
      activeCategoryId: null,
      isLoading: false,

      setCategories: (categories) => set({ categories }),
      addCategory: (category) => set((state) => ({
        categories: [...state.categories, category],
      })),
      updateCategory: (id, updates) => set((state) => ({
        categories: state.categories.map((c) =>
          c.id === id ? { ...c, ...updates } : c
        ),
      })),
      deleteCategory: (id) => set((state) => ({
        categories: state.categories.filter((c) => c.id !== id),
      })),
      setActiveCategory: (id) => set({ activeCategoryId: id }),
      setLoading: (loading) => set({ isLoading: loading }),
      reorderCategories: (activeId, overId) => set((state) => {
        const categories = [...state.categories];
        const activeIndex = categories.findIndex(c => c.id === activeId);
        const overIndex = categories.findIndex(c => c.id === overId);
        if (activeIndex !== -1 && overIndex !== -1) {
          const [removed] = categories.splice(activeIndex, 1);
          categories.splice(overIndex, 0, removed);
        }
        return { categories };
      }),
    }),
    {
      name: 'category-data-storage',
    }
  )
);

export default useCategoryData;
