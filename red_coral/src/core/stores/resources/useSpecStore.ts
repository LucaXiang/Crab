import { createResourceStore } from '../factory/createResourceStore';
import { invoke } from '@tauri-apps/api/core';
import type { ProductSpecification, ApiResponse } from '@/core/domain/types/api';

/**
 * Spec Store - 规格数据
 *
 * 注意：规格数据与产品关联，通常按产品查询。
 * 这里提供全量加载，用于需要全局规格列表的场景。
 */
async function fetchSpecs(): Promise<ProductSpecification[]> {
  // 使用直接的 invoke 调用，因为 TauriApiClient.listSpecs 需要 product_id
  const response = await invoke<ProductSpecification[] | ApiResponse<{ specs: ProductSpecification[] }>>('list_all_specs');
  if (Array.isArray(response)) {
    return response;
  }
  if (response.data?.specs) {
    return response.data.specs;
  }
  throw new Error(response.message || 'Failed to fetch specs');
}

export const useSpecStore = createResourceStore<ProductSpecification & { id: string }>(
  'spec',
  fetchSpecs as () => Promise<(ProductSpecification & { id: string })[]>
);

// Convenience hooks
export const useSpecs = () => useSpecStore((state) => state.items);
export const useSpecsLoading = () => useSpecStore((state) => state.isLoading);
export const useSpecById = (id: string) =>
  useSpecStore((state) => state.items.find((s) => s.id === id));
export const useSpecsByProduct = (productId: string) =>
  useSpecStore((state) => state.items.filter((s) => s.product === productId));
