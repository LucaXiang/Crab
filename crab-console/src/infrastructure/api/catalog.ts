import { request } from './client';
import type {
  CatalogProduct, ProductCreate, ProductUpdate,
  CatalogCategory, CategoryCreate, CategoryUpdate,
  CatalogTag, TagCreate, TagUpdate,
  CatalogAttribute, AttributeCreate, AttributeUpdate,
  PriceRule, PriceRuleCreate, PriceRuleUpdate,
  CatalogOpResult,
} from '@/core/types/catalog';

const catalogPath = (storeId: number, resource: string) =>
  `/api/tenant/stores/${storeId}/catalog/${resource}`;

// ── Products ──
export const listProducts = (token: string, storeId: number) =>
  request<CatalogProduct[]>('GET', catalogPath(storeId, 'products'), undefined, token);

export const createProduct = (token: string, storeId: number, data: ProductCreate) =>
  request<CatalogOpResult>('POST', catalogPath(storeId, 'products'), data, token);

export const updateProduct = (token: string, storeId: number, id: number, data: ProductUpdate) =>
  request<CatalogOpResult>('PUT', `${catalogPath(storeId, 'products')}/${id}`, data, token);

export const deleteProduct = (token: string, storeId: number, id: number) =>
  request<CatalogOpResult>('DELETE', `${catalogPath(storeId, 'products')}/${id}`, undefined, token);

// ── Categories ──
export const listCategories = (token: string, storeId: number) =>
  request<CatalogCategory[]>('GET', catalogPath(storeId, 'categories'), undefined, token);

export const createCategory = (token: string, storeId: number, data: CategoryCreate) =>
  request<CatalogOpResult>('POST', catalogPath(storeId, 'categories'), data, token);

export const updateCategory = (token: string, storeId: number, id: number, data: CategoryUpdate) =>
  request<CatalogOpResult>('PUT', `${catalogPath(storeId, 'categories')}/${id}`, data, token);

export const deleteCategory = (token: string, storeId: number, id: number) =>
  request<CatalogOpResult>('DELETE', `${catalogPath(storeId, 'categories')}/${id}`, undefined, token);

// ── Tags ──
export const listTags = (token: string, storeId: number) =>
  request<CatalogTag[]>('GET', catalogPath(storeId, 'tags'), undefined, token);

export const createTag = (token: string, storeId: number, data: TagCreate) =>
  request<CatalogOpResult>('POST', catalogPath(storeId, 'tags'), data, token);

export const updateTag = (token: string, storeId: number, id: number, data: TagUpdate) =>
  request<CatalogOpResult>('PUT', `${catalogPath(storeId, 'tags')}/${id}`, data, token);

export const deleteTag = (token: string, storeId: number, id: number) =>
  request<CatalogOpResult>('DELETE', `${catalogPath(storeId, 'tags')}/${id}`, undefined, token);

// ── Attributes ──
export const listAttributes = (token: string, storeId: number) =>
  request<CatalogAttribute[]>('GET', catalogPath(storeId, 'attributes'), undefined, token);

export const createAttribute = (token: string, storeId: number, data: AttributeCreate) =>
  request<CatalogOpResult>('POST', catalogPath(storeId, 'attributes'), data, token);

export const updateAttribute = (token: string, storeId: number, id: number, data: AttributeUpdate) =>
  request<CatalogOpResult>('PUT', `${catalogPath(storeId, 'attributes')}/${id}`, data, token);

export const deleteAttribute = (token: string, storeId: number, id: number) =>
  request<CatalogOpResult>('DELETE', `${catalogPath(storeId, 'attributes')}/${id}`, undefined, token);

// ── Price Rules ──
export const listPriceRules = (token: string, storeId: number) =>
  request<PriceRule[]>('GET', catalogPath(storeId, 'price-rules'), undefined, token);

export const createPriceRule = (token: string, storeId: number, data: PriceRuleCreate) =>
  request<CatalogOpResult>('POST', catalogPath(storeId, 'price-rules'), data, token);

export const updatePriceRule = (token: string, storeId: number, id: number, data: PriceRuleUpdate) =>
  request<CatalogOpResult>('PUT', `${catalogPath(storeId, 'price-rules')}/${id}`, data, token);

export const deletePriceRule = (token: string, storeId: number, id: number) =>
  request<CatalogOpResult>('DELETE', `${catalogPath(storeId, 'price-rules')}/${id}`, undefined, token);
