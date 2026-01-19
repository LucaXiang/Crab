/**
 * DataSource Types
 */

import type { Product, ProductAttribute, CategoryAttribute } from '@/core/domain/types/api';

/**
 * Type of data source backend
 */
export type DataSourceType = 'tauri' | 'http';

/**
 * DataSource configuration
 */
export interface DataSourceConfig {
  type: DataSourceType;
  baseUrl?: string;
}

/**
 * Base error class for data source errors
 */
export class DataSourceError extends Error {
  constructor(message: string, public code?: string) {
    super(message);
    this.name = 'DataSourceError';
  }
}

/**
 * Network-related error
 */
export class NetworkError extends DataSourceError {
  constructor(message: string = 'Network error occurred') {
    super(message, 'NETWORK_ERROR');
    this.name = 'NetworkError';
  }
}

/**
 * Resource not found error
 */
export class NotFoundError extends DataSourceError {
  constructor(resource: string) {
    super(`${resource} not found`, 'NOT_FOUND');
    this.name = 'NotFoundError';
  }
}

/**
 * Validation error
 */
export class ValidationError extends DataSourceError {
  constructor(message: string) {
    super(message, 'VALIDATION_ERROR');
    this.name = 'ValidationError';
  }
}

// Product types
export interface FetchProductsParams {
  categoryId?: string;
  category?: string;
  search?: string;
  page?: number;
  pageSize?: number;
  limit?: number;
}

export interface FetchProductsResponse {
  items: Array<{
    id: string;
    name: string;
    price: number;
    category: string;
    image: string;
    externalId: number;
    sortOrder?: number;
    receiptName?: string;
    taxRate: number;
    kitchenPrinterId?: number | null;
    kitchenPrintName?: string;
    isKitchenPrintEnabled?: number | null;
    isLabelPrintEnabled?: number | null;
  }>;
  products: Array<{
    id: string;
    name: string;
    price: number;
    category: string;
    image: string;
    externalId: number;
    sortOrder?: number;
    receiptName?: string;
    taxRate: number;
    kitchenPrinterId?: number | null;
    kitchenPrintName?: string;
    isKitchenPrintEnabled?: number | null;
    isLabelPrintEnabled?: number | null;
  }>;
  total: number;
  page?: number;
}

export interface CreateProductParams {
  name: string;
  price: number;
  category: string;
  image?: string;
  receiptName?: string;
  taxRate: number;
  kitchenPrinterId?: number | null;
  kitchenPrintName?: string;
  isKitchenPrintEnabled?: number;
  isLabelPrintEnabled?: number;
  sortOrder?: number;
  externalId: number;
}

export interface UpdateProductParams extends Partial<CreateProductParams> {
  id: string;
}

// Attribute types
export interface CreateAttributeTemplateParams {
  name: string;
  type?: string;
  displayOrder?: number;
  showOnReceipt?: boolean;
  receiptName?: string;
  kitchenPrinterId?: number | null;
  isGlobal?: boolean;
}

export interface UpdateAttributeTemplateParams extends Partial<CreateAttributeTemplateParams> {
  id: string;
  isActive?: boolean;
}

export interface CreateAttributeOptionParams {
  attributeId: string;
  name: string;
  displayOrder?: number;
  priceModifier?: number;
  receiptName?: string;
  isDefault?: boolean;
  valueCode?: string;
}

export interface UpdateAttributeOptionParams extends Partial<CreateAttributeOptionParams> {
  id: string;
  isActive?: boolean;
}

export interface BindProductAttributeParams {
  productId: string;
  attributeId: string;
  isRequired?: boolean;
  displayOrder?: number;
  defaultOptionIds?: string[];
}

export interface BindCategoryAttributeParams {
  categoryId: string;
  attributeId: string;
  isRequired?: boolean;
  displayOrder?: number;
  defaultOptionIds?: string[];
}

export interface ProductAttributeBinding {
  productId: string;
  attributeId: string;
  defaultOptionIds: string[];
}

export interface CategoryAttributeBinding {
  categoryId: string;
  attributeId: string;
  defaultOptionIds: string[];
}

/**
 * Product with attributes response
 * Uses API types for consistency
 */
export interface ProductWithAttributesResp {
  product: Product;
  attributes: ProductAttribute[];
}

export interface CategoryAttributesResp {
  categoryId: string;
  attributes: CategoryAttribute[];
}

// Table types
export interface FetchTablesParams {
  zoneId?: string;
  status?: string;
  search?: string;
  page?: number;
  limit?: number;
}

export interface FetchTablesResponse {
  tables: Array<{
    id: string;
    name: string;
    zoneId: string;
    capacity: number;
    status: string;
  }>;
  total: number;
  page?: number;
}

// Kitchen Printer types
export interface FetchKitchenPrintersResponse {
  printers: Array<{
    id: number;
    name: string;
    connectionType: string;
    connectionInfo: string;
    isDefault: number;
  }>;
}

export interface CreateKitchenPrinterParams {
  name: string;
  connectionType: string;
  connectionInfo: string;
  isDefault?: number;
}

export interface UpdateKitchenPrinterParams extends Partial<CreateKitchenPrinterParams> {
  id: number;
}
