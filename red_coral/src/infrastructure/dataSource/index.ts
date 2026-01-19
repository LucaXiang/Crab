/**
 * DataSource Types
 * Kept for type compatibility, but data access now uses API client directly
 */

// Types from domain
export type {
  Zone,
  KitchenPrinter,
  AttributeTemplate,
  AttributeOption,
  ProductSpecification,
} from '@/core/domain/types';

// Types
export type { DataSourceType, DataSourceConfig, DataSourceError, NetworkError, NotFoundError, ValidationError } from './types';
export type {
  FetchProductsParams,
  FetchProductsResponse,
  CreateProductParams,
  UpdateProductParams,
  CreateAttributeTemplateParams,
  UpdateAttributeTemplateParams,
  CreateAttributeOptionParams,
  UpdateAttributeOptionParams,
  BindProductAttributeParams,
  BindCategoryAttributeParams,
  ProductAttributeBinding,
  CategoryAttributeBinding,
  ProductWithAttributesResp,
  CategoryAttributesResp,
  FetchTablesParams,
  FetchTablesResponse,
  FetchKitchenPrintersResponse,
  CreateKitchenPrinterParams,
  UpdateKitchenPrinterParams,
} from './types';
