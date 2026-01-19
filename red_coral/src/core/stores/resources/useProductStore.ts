import { createCrudResourceStore } from '../factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { Product } from '@/infrastructure/api/types';

const api = createTauriClient();

// Product with guaranteed id
type ProductEntity = Product & { id: string };

// Create product input type
interface CreateProductInput {
  name: string;
  receipt_name?: string;
  price: number;
  category_id: string;
  image?: string;
  external_id?: number;
  tax_rate?: number;
  sort_order?: number;
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
  kitchen_printer_id?: string | null;
}

// Update product input type
interface UpdateProductInput {
  name?: string;
  receipt_name?: string;
  price?: number;
  category_id?: string;
  image?: string;
  external_id?: number;
  tax_rate?: number;
  sort_order?: number;
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
  kitchen_printer_id?: string | null;
}

async function fetchProducts(): Promise<ProductEntity[]> {
  const response = await api.listProducts();
  if (response.data?.products) {
    return response.data.products as ProductEntity[];
  }
  throw new Error(response.message || 'Failed to fetch products');
}

async function createProduct(data: CreateProductInput): Promise<ProductEntity> {
  const response = await api.createProduct(data as any);
  if (response.data?.product) {
    return response.data.product as ProductEntity;
  }
  throw new Error(response.message || 'Failed to create product');
}

async function updateProduct(id: string, data: UpdateProductInput): Promise<ProductEntity> {
  const response = await api.updateProduct(id, data as any);
  if (response.data?.product) {
    return response.data.product as ProductEntity;
  }
  // Some APIs return the updated product directly
  if (response.data && 'id' in response.data) {
    return response.data as ProductEntity;
  }
  throw new Error(response.message || 'Failed to update product');
}

async function deleteProduct(id: string): Promise<void> {
  const response = await api.deleteProduct(id);
  if (!response.data?.deleted && response.code !== 'OK') {
    throw new Error(response.message || 'Failed to delete product');
  }
}

export const useProductStore = createCrudResourceStore<ProductEntity, CreateProductInput, UpdateProductInput>(
  'product',
  fetchProducts,
  {
    create: createProduct,
    update: updateProduct,
    remove: deleteProduct,
  }
);

// Convenience hooks
export const useProducts = () => useProductStore((state) => state.items);
export const useProductsLoading = () => useProductStore((state) => state.isLoading);
export const useProductById = (id: string) =>
  useProductStore((state) => state.items.find((p) => p.id === id));

// CRUD action hooks
export const useProductActions = () => ({
  create: useProductStore.getState().create,
  update: useProductStore.getState().update,
  remove: useProductStore.getState().remove,
  fetchAll: useProductStore.getState().fetchAll,
});
