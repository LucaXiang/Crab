import { request, ApiError, API_BASE } from './client';
import type {
  StoreProduct, ProductCreate, ProductUpdate,
  StoreCategory, CategoryCreate, CategoryUpdate,
  StoreTag, TagCreate, TagUpdate,
  StoreAttribute, AttributeCreate, AttributeUpdate,
  AttributeOptionCreate, AttributeOptionUpdate,
  BindAttributeRequest, UnbindAttributeRequest,
  PriceRule, PriceRuleCreate, PriceRuleUpdate,
  LabelTemplate, LabelTemplateCreate, LabelTemplateUpdate,
  SortOrderItem, BulkDeleteRequest,
  StoreInfo, StoreInfoUpdate,
  StoreOpResult,
} from '@/core/types/store';

const storePath = (storeId: number, resource: string) =>
  `/api/tenant/stores/${storeId}/${resource}`;

// ── Products ──
export const listProducts = (token: string, storeId: number) =>
  request<StoreProduct[]>('GET', storePath(storeId, 'products'), undefined, token);

export const createProduct = (token: string, storeId: number, data: ProductCreate) =>
  request<StoreOpResult>('POST', storePath(storeId, 'products'), data, token);

export const updateProduct = (token: string, storeId: number, id: number, data: ProductUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, 'products')}/${id}`, data, token);

export const deleteProduct = (token: string, storeId: number, id: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, 'products')}/${id}`, undefined, token);

export const batchUpdateProductSortOrder = (token: string, storeId: number, items: SortOrderItem[]) =>
  request<StoreOpResult>('PATCH', `${storePath(storeId, 'products/sort-order')}`, { items }, token);

export const bulkDeleteProducts = (token: string, storeId: number, ids: number[]) =>
  request<StoreOpResult>('POST', `${storePath(storeId, 'products/bulk-delete')}`, { ids } as BulkDeleteRequest, token);

// ── Categories ──
export const listCategories = (token: string, storeId: number) =>
  request<StoreCategory[]>('GET', storePath(storeId, 'categories'), undefined, token);

export const createCategory = (token: string, storeId: number, data: CategoryCreate) =>
  request<StoreOpResult>('POST', storePath(storeId, 'categories'), data, token);

export const updateCategory = (token: string, storeId: number, id: number, data: CategoryUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, 'categories')}/${id}`, data, token);

export const deleteCategory = (token: string, storeId: number, id: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, 'categories')}/${id}`, undefined, token);

export const batchUpdateCategorySortOrder = (token: string, storeId: number, items: SortOrderItem[]) =>
  request<StoreOpResult>('PATCH', `${storePath(storeId, 'categories/sort-order')}`, { items }, token);

// ── Tags ──
export const listTags = (token: string, storeId: number) =>
  request<StoreTag[]>('GET', storePath(storeId, 'tags'), undefined, token);

export const createTag = (token: string, storeId: number, data: TagCreate) =>
  request<StoreOpResult>('POST', storePath(storeId, 'tags'), data, token);

export const updateTag = (token: string, storeId: number, id: number, data: TagUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, 'tags')}/${id}`, data, token);

export const deleteTag = (token: string, storeId: number, id: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, 'tags')}/${id}`, undefined, token);

// ── Attributes ──
export const listAttributes = (token: string, storeId: number) =>
  request<StoreAttribute[]>('GET', storePath(storeId, 'attributes'), undefined, token);

export const createAttribute = (token: string, storeId: number, data: AttributeCreate) =>
  request<StoreOpResult>('POST', storePath(storeId, 'attributes'), data, token);

export const updateAttribute = (token: string, storeId: number, id: number, data: AttributeUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, 'attributes')}/${id}`, data, token);

export const deleteAttribute = (token: string, storeId: number, id: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, 'attributes')}/${id}`, undefined, token);

// ── Attribute Options (independent CRUD) ──
export const createAttributeOption = (token: string, storeId: number, attributeId: number, data: AttributeOptionCreate) =>
  request<StoreOpResult>('POST', `${storePath(storeId, `attributes/${attributeId}/options`)}`, data, token);

export const updateAttributeOption = (token: string, storeId: number, attributeId: number, optionId: number, data: AttributeOptionUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, `attributes/${attributeId}/options/${optionId}`)}`, data, token);

export const deleteAttributeOption = (token: string, storeId: number, attributeId: number, optionId: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, `attributes/${attributeId}/options/${optionId}`)}`, undefined, token);

export const batchUpdateOptionSortOrder = (token: string, storeId: number, attributeId: number, items: SortOrderItem[]) =>
  request<StoreOpResult>('PATCH', `${storePath(storeId, `attributes/${attributeId}/options/sort-order`)}`, { items }, token);

// ── Attribute Binding ──
export const bindAttribute = (token: string, storeId: number, data: BindAttributeRequest) =>
  request<StoreOpResult>('POST', storePath(storeId, 'attributes/bind'), data, token);

export const unbindAttribute = (token: string, storeId: number, bindingId: number) =>
  request<StoreOpResult>('POST', storePath(storeId, 'attributes/unbind'), { binding_id: bindingId } as UnbindAttributeRequest, token);

// ── Price Rules ──
export const listPriceRules = (token: string, storeId: number) =>
  request<PriceRule[]>('GET', storePath(storeId, 'price-rules'), undefined, token);

export const createPriceRule = (token: string, storeId: number, data: PriceRuleCreate) =>
  request<StoreOpResult>('POST', storePath(storeId, 'price-rules'), data, token);

export const updatePriceRule = (token: string, storeId: number, id: number, data: PriceRuleUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, 'price-rules')}/${id}`, data, token);

export const deletePriceRule = (token: string, storeId: number, id: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, 'price-rules')}/${id}`, undefined, token);

// ── Label Templates ──
export const listLabelTemplates = (token: string, storeId: number) =>
  request<LabelTemplate[]>('GET', storePath(storeId, 'label-templates'), undefined, token);

export const createLabelTemplate = (token: string, storeId: number, data: LabelTemplateCreate) =>
  request<StoreOpResult>('POST', storePath(storeId, 'label-templates'), data, token);

export const updateLabelTemplate = (token: string, storeId: number, id: number, data: LabelTemplateUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, 'label-templates')}/${id}`, data, token);

export const deleteLabelTemplate = (token: string, storeId: number, id: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, 'label-templates')}/${id}`, undefined, token);

// ── Store Info ──
export const getStoreInfo = (token: string, storeId: number) =>
  request<StoreInfo>('GET', storePath(storeId, 'store-info'), undefined, token);

export const updateStoreInfo = (token: string, storeId: number, data: StoreInfoUpdate) =>
  request<StoreOpResult>('PUT', storePath(storeId, 'store-info'), data, token);

// ── Image Upload ──

export async function uploadImage(token: string, file: File): Promise<string> {
  const form = new FormData();
  form.append('file', file);
  const res = await fetch(`${API_BASE}/api/tenant/images`, {
    method: 'POST',
    headers: { Authorization: `Bearer ${token}` },
    body: form,
  });
  const data = await res.json().catch(() => null);
  if (!res.ok) throw new ApiError(res.status, data?.message ?? 'Upload failed', data?.code ?? null);
  return data.hash as string;
}

/** Get a presigned S3 URL for an image hash, then create a blob URL for use in <img> / Canvas */
export async function getImageBlobUrl(token: string, hash: string): Promise<string> {
  const res = await fetch(`${API_BASE}/api/tenant/images/${hash}`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  const data = await res.json().catch(() => null);
  if (!res.ok) throw new ApiError(res.status, data?.message ?? 'Image not found', data?.code ?? null);
  // Validate presigned URL is from expected S3 domain
  try {
    const parsed = new URL(data.url);
    if (parsed.protocol !== 'https:' || !parsed.hostname.endsWith('.amazonaws.com')) {
      throw new ApiError(400, 'Invalid image URL', null);
    }
  } catch (e) {
    if (e instanceof ApiError) throw e;
    throw new ApiError(400, 'Invalid image URL', null);
  }
  // Fetch from presigned S3 URL and create blob URL
  const imgRes = await fetch(data.url);
  if (!imgRes.ok) throw new ApiError(imgRes.status, 'Failed to load image', null);
  const blob = await imgRes.blob();
  return URL.createObjectURL(blob);
}
