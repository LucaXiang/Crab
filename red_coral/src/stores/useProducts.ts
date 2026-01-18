/**
 * Products Store
 * Manages product data for the application
 */

import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export interface Product {
  id: string;
  name: string;
  categoryId: string;
  price: number;
  cost?: number;
  description?: string;
  image?: string;
  isAvailable: boolean;
  isKitchenPrintEnabled: boolean;
  sortOrder?: number;
  createdAt: string;
  updatedAt: string;
}

export interface ProductState {
  products: Product[];
  selectedProductId: string | null;
  searchQuery: string;
  isLoading: boolean;

  // Actions
  setProducts: (products: Product[]) => void;
  addProduct: (product: Product) => void;
  updateProduct: (id: string, updates: Partial<Product>) => void;
  deleteProduct: (id: string) => void;
  setSelectedProduct: (id: string | null) => void;
  setSearchQuery: (query: string) => void;
  setLoading: (loading: boolean) => void;
}

export const useProducts = create<ProductState>()(
  persist(
    (set) => ({
      products: [],
      selectedProductId: null,
      searchQuery: '',
      isLoading: false,

      setProducts: (products) => set({ products }),
      addProduct: (product) => set((state) => ({
        products: [...state.products, product],
      })),
      updateProduct: (id, updates) => set((state) => ({
        products: state.products.map((p) =>
          p.id === id ? { ...p, ...updates } : p
        ),
      })),
      deleteProduct: (id) => set((state) => ({
        products: state.products.filter((p) => p.id !== id),
      })),
      setSelectedProduct: (id) => set({ selectedProductId: id }),
      setSearchQuery: (query) => set({ searchQuery: query }),
      setLoading: (loading) => set({ isLoading: loading }),
    }),
    {
      name: 'products-storage',
    }
  )
);

export default useProducts;
