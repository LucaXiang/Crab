import { createCrudResourceStore } from '@/core/stores/factory/createResourceStore';
import { createTauriClient } from '@/infrastructure/api';
import type { ProductFull, ProductCreate, ProductUpdate } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

export const useProductStore = createCrudResourceStore<ProductFull, ProductCreate, ProductUpdate>(
  'product',
  () => getApi().listProducts(),
  {
    create: (data) => getApi().createProduct(data),
    update: (id, data) => getApi().updateProduct(id, data),
    remove: (id) => getApi().deleteProduct(id),
  }
);

// Convenience hooks
export const useProducts = () => useProductStore((state) => state.items);
export const useProductsLoading = () => useProductStore((state) => state.isLoading);
export const useProductById = (id: number) =>
  useProductStore((state) => state.items.find((p) => p.id === id));

// CRUD action hooks
export const useProductActions = () => ({
  create: useProductStore.getState().create,
  update: useProductStore.getState().update,
  remove: useProductStore.getState().remove,
  fetchAll: useProductStore.getState().fetchAll,
});
