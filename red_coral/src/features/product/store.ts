import { createCrudResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { ProductFull, ProductCreate, ProductUpdate } from '@/core/domain/types/api';

const api = createTauriClient();

// ProductFull with guaranteed id (contains specs, attributes, tags)
type ProductEntity = ProductFull & { id: string };

export const useProductStore = createCrudResourceStore<ProductEntity, ProductCreate, ProductUpdate>(
  'product',
  () => api.listProducts() as Promise<ProductEntity[]>,
  {
    create: (data) => api.createProduct(data) as Promise<ProductEntity>,
    update: (id, data) => api.updateProduct(id, data) as Promise<ProductEntity>,
    remove: (id) => api.deleteProduct(id),
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
