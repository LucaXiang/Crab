import { request, ApiError, API_BASE } from './client';
import type {
  StoreProduct, ProductCreate, ProductUpdate,
  StoreCategory, CategoryCreate, CategoryUpdate,
  StoreTag, TagCreate, TagUpdate,
  StoreAttribute, AttributeCreate, AttributeUpdate,
  PriceRule, PriceRuleCreate, PriceRuleUpdate,
  LabelTemplate, LabelTemplateCreate, LabelTemplateUpdate,
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

// ── Categories ──
export const listCategories = (token: string, storeId: number) =>
  request<StoreCategory[]>('GET', storePath(storeId, 'categories'), undefined, token);

export const createCategory = (token: string, storeId: number, data: CategoryCreate) =>
  request<StoreOpResult>('POST', storePath(storeId, 'categories'), data, token);

export const updateCategory = (token: string, storeId: number, id: number, data: CategoryUpdate) =>
  request<StoreOpResult>('PUT', `${storePath(storeId, 'categories')}/${id}`, data, token);

export const deleteCategory = (token: string, storeId: number, id: number) =>
  request<StoreOpResult>('DELETE', `${storePath(storeId, 'categories')}/${id}`, undefined, token);

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
  // Fetch from presigned S3 URL and create blob URL
  const imgRes = await fetch(data.url);
  if (!imgRes.ok) throw new ApiError(imgRes.status, 'Failed to load image', null);
  const blob = await imgRes.blob();
  return URL.createObjectURL(blob);
}
