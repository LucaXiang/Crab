export * from './error';
export * from './models';

// Import types used in this file
import type {
  Product,
  ProductSpecification,
  Tag,
  ProductAttribute,
  CategoryAttribute,
  Category,
  Zone,
  Table,
  KitchenPrinter,
  Attribute,
  AttributeTemplate,
  AttributeOption,
  Role,
  RolePermission,
  PriceRule,
} from './models';

// API Response types - aligned with Rust server
export interface ApiResponse<T> {
  error_code: string | null;
  message: string;
  data?: T;
}

export interface DeleteResponse {
  deleted: boolean;
}

// Batch operation types
export interface BulkDeleteRequest {
  ids: number[];
}

export interface BatchUpdateSortOrderRequest {
  updates: SortOrderUpdate[];
}

export interface SortOrderUpdate {
  id: number;
  sort_order: number;
}

// Data Import/Export types
export interface ImportRequest {
  data_type: 'products' | 'categories' | 'all';
  format: 'json' | 'csv';
  data: Record<string, unknown>;
  options?: ImportOptions;
}

export interface ImportOptions {
  update_existing?: boolean;
  skip_errors?: boolean;
}

export interface ImportResult {
  success: boolean;
  imported: number;
  updated: number;
  skipped: number;
  errors: string[];
}

export interface ExportRequest {
  data_type: 'products' | 'categories' | 'all';
  format: 'json' | 'csv';
  include_deleted?: boolean;
}

export interface ExportResponse {
  data_type: string;
  format: string;
  record_count: number;
  data: Record<string, unknown>;
}

// Auth types
export interface LoginRequest {
  username: string;
  password: string;
}

export interface LoginResponseData {
  access_token: string;
  expires_in: number;
  token_type: string;
  user: CurrentUser;
}

export interface CurrentUser {
  id: number;
  uuid: string;
  username: string;
  display_name: string | null;
  role_id: number;
  role_name: string;
  permissions: string[];
  avatar: string | null;
}

export interface RegisterRequest {
  username: string;
  password: string;
  display_name?: string;
}

export interface ChangePasswordRequest {
  old_password: string;
  new_password: string;
}

export interface CurrentUserData {
  user: CurrentUser;
}

export interface SuccessData {
  success: boolean;
}

// Product types
export interface ProductQuery {
  page?: number;
  page_size?: number;
  category_id?: number;
  is_active?: boolean;
  search?: string;
}

export interface ProductData {
  product: Product;
  specifications?: ProductSpecification[];
  tags?: Tag[];
  attributes?: ProductAttribute[];
}

export interface ProductResponse {
  id: number;
  uuid: string;
  name: string;
  image: string | null;
  category_id: number | null;
  sort_order: number;
  tax_rate: number;
  has_multi_spec: boolean;
  receipt_name: string | null;
  kitchen_print_name: string | null;
  kitchen_printer_id: number | null;
  is_kitchen_print_enabled: boolean;
  is_label_print_enabled: boolean;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  specifications?: ProductSpecification[];
  tags?: Tag[];
  attributes?: ProductAttribute[];
}

export interface ProductListData {
  products: Product[];
  specs?: ProductSpecListData['specs'];
  total: number;
  page?: number;
  page_size?: number;
}

export interface CreateProductRequest {
  name: string;
  price: number;
  image?: string;
  category_id?: number;
  sort_order?: number;
  tax_rate?: number;
  has_multi_spec?: boolean;
  receipt_name?: string;
  kitchen_print_name?: string;
  kitchen_printer_id?: number;
  is_kitchen_print_enabled?: number;
  is_label_print_enabled?: number;
  external_id?: number;
  specifications?: CreateSpecificationRequest[];
  specifications_to_delete?: string[];
}

export interface UpdateProductRequest {
  name?: string;
  price?: number;
  image?: string;
  category_id?: number;
  sort_order?: number;
  tax_rate?: number;
  has_multi_spec?: boolean;
  receipt_name?: string;
  kitchen_print_name?: string;
  kitchen_printer_id?: number;
  is_kitchen_print_enabled?: number;
  is_label_print_enabled?: number;
  external_id?: number;
}

export interface CreateSpecificationRequest {
  uuid?: string;
  name: string;
  price: number;
  display_order?: number;
  is_default?: boolean;
  is_active?: boolean;
  is_root?: boolean;
  external_id?: string;
}

export interface UpdateSpecificationRequest {
  name: string;
  price: number;
  receipt_name?: string | null;
  external_id?: number | null;
  display_order?: number;
  is_default?: boolean;
  is_active?: boolean;
}

export interface ProductSpecListData {
  specs: ProductSpecification[];
  total: number;
}

// Category types
export interface CategoryData extends Category {}

export interface CategoryListData {
  categories: CategoryData[];
  total: number;
}

export interface CreateCategoryRequest {
  name: string;
  sort_order?: number;
  kitchen_printer_id?: number;
  kitchen_printer?: string;
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
}

export interface UpdateCategoryRequest {
  name?: string;
  sort_order?: number;
  kitchen_printer_id?: number;
  kitchen_printer?: string;
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
}

// Price Adjustment types - aligned with Rust server
export interface PriceAdjustmentData {
  id: number;
  uuid: string;
  name: string;
  display_name: string;
  receipt_name: string;
  description?: string;
  rule_type: string;
  product_scope: string;
  target_id?: number;
  zone_scope: number;
  adjustment_type: string;
  adjustment_value: number;
  priority: number;
  is_stackable: boolean;
  time_mode: string;
  start_time?: number;
  end_time?: number;
  schedule_config_json?: string;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface PriceAdjustmentListData {
  rules: PriceAdjustmentData[];
  total: number;
}

export interface CreatePriceAdjustmentRequest {
  name: string;
  display_name: string;
  receipt_name: string;
  description?: string;
  rule_type: string;           // "SURCHARGE" | "DISCOUNT"
  product_scope: string;       // "PRODUCT" | "TAG" | "CATEGORY" | "GLOBAL"
  target_id?: number;          // i64
  zone_scope: number;          // 0=RETAIL, -1=ALL, >0=ZONE_ID
  adjustment_type: string;     // "PERCENTAGE" | "FIXED_AMOUNT"
  adjustment_value: number;    // i64 - 金额(分) 或 百分比值
  priority?: number;           // i64, default 0
  is_stackable?: boolean;      // default false
  time_mode?: string;          // "ALWAYS" | "SCHEDULE" | "ONETIME"
  start_time?: number;         // i64, Unix timestamp
  end_time?: number;           // i64, Unix timestamp
  schedule_config_json?: string;
  is_active?: boolean;         // default true
}

export interface UpdatePriceAdjustmentRequest {
  name?: string;
  display_name?: string;
  receipt_name?: string;
  description?: string;
  rule_type?: string;
  product_scope?: string;
  target_id?: number;
  zone_scope?: number;
  adjustment_type?: string;
  adjustment_value?: number;
  priority?: number;
  is_stackable?: boolean;
  time_mode?: string;
  start_time?: number;
  end_time?: number;
  schedule_config_json?: string;
  is_active?: boolean;
}

// Attribute Template types
// Note: AttributeTemplateData extends AttributeTemplate but makes options optional for API responses
export interface AttributeTemplateData extends Omit<AttributeTemplate, 'options'> {
  options?: AttributeOption[];
}

export interface AttributeTemplateListData {
  templates: AttributeTemplate[];
  total: number;
}

export interface CreateAttributeTemplateRequest {
  name: string;
  type_: string;
  display_order?: number;
  is_active?: boolean;
  show_on_receipt?: boolean;
  receipt_name?: string;
  kitchen_printer_id?: number;
  is_global?: boolean;
}

export interface UpdateAttributeTemplateRequest {
  name?: string;
  type_?: string;
  display_order?: number;
  is_active?: boolean;
  show_on_receipt?: boolean;
  receipt_name?: string;
  kitchen_printer_id?: number;
  is_global?: boolean;
}

// Attribute Option types
export interface AttributeOptionData extends AttributeOption {}

export interface AttributeOptionListData {
  options: AttributeOption[];
  total: number;
}

export interface CreateAttributeOptionRequest {
  attribute_id: number;
  name: string;
  value_code: string;
  price_modifier?: number;
  is_default?: boolean;
  display_order?: number;
  is_active?: boolean;
  receipt_name?: string;
}

export interface UpdateAttributeOptionRequest {
  name?: string;
  value_code?: string;
  price_modifier?: number;
  is_default?: boolean;
  display_order?: number;
  is_active?: boolean;
  receipt_name?: string;
}

// Tag types
export interface TagData extends Tag {}

export interface TagListData {
  tags: Tag[];
  total: number;
}

export interface CreateTagRequest {
  name: string;
  color?: string;
}

export interface UpdateTagRequest {
  name?: string;
  color?: string;
  is_active?: boolean;
}

// Role types
export interface RoleData extends Role {}

export interface RoleListData {
  roles: Role[];
  total: number;
}

export interface CreateRoleRequest {
  name: string;
  display_name: string;
  description?: string;
}

export interface UpdateRoleRequest {
  name?: string;
  display_name?: string;
  description?: string;
}

export interface CreateRolePermissionsRequest {
  role_id: number;
  permissions: string[];
}

// Printer types
export interface PrinterData extends KitchenPrinter {}

export interface PrinterListData {
  printers: KitchenPrinter[];
  total: number;
}

export interface CreatePrinterRequest {
  name: string;
  printer_name: string;
  description?: string;
}

export interface UpdatePrinterRequest {
  name: string;
  printer_name?: string;
  description?: string;
}

// Zone types
export interface ZoneData extends Zone {}

export interface ZoneListData {
  zones: Zone[];
  total: number;
}

export interface CreateZoneRequest {
  name: string;
  description?: string;
}

export interface UpdateZoneRequest {
  name?: string;
  description?: string;
  is_active?: boolean;
}

// Table types
export interface TableData extends Table {}

export interface TableListData {
  tables: Table[];
  total: number;
}

export interface CreateTableRequest {
  name: string;
  zone_id?: number;
  zone?: string;
  capacity?: number;
}

export interface UpdateTableRequest {
  name?: string;
  zone_id?: number;
  capacity?: number;
}

// Category Attribute types
export interface CategoryAttributeData extends CategoryAttribute {
  attribute?: AttributeTemplate;
  options?: AttributeOption[];
}

export interface CategoryAttributeListData {
  category_attributes: CategoryAttribute[];
  total: number;
}

export interface CreateCategoryAttributeRequest {
  category_id: string | number;
  attribute_id: string | number;
  is_required?: boolean;
  display_order?: number;
  default_option_id?: number;
}

export interface UpdateCategoryAttributeRequest {
  is_required?: boolean;
  display_order?: number;
  default_option_id?: number;
}

// Product Attribute types (binding between Product and Attribute)
export interface ProductAttributeData extends ProductAttribute {
  attribute?: Attribute;
  options?: AttributeOption[];
}

export interface ProductAttributeListData {
  product_attributes: ProductAttribute[];
  total: number;
}

export interface CreateProductAttributeRequest {
  product_id: string;
  attribute_id: string;
  is_required?: boolean;
  display_order?: number;
  default_option_idx?: number;
}

export interface UpdateProductAttributeRequest {
  is_required?: boolean;
  display_order?: number;
  default_option_idx?: number;
}

// Specification Tag types
export interface SpecificationTagData {
  specification_id: number;
  tag_id: number;
  tag?: Tag;
  created_at: string;
}

export interface SpecificationTagListData {
  associations: SpecificationTagData[];
  total: number;
}

export interface CreateSpecificationTagsRequest {
  specification_id: number;
  tag_ids: number[];
}

// Health types
export interface HealthData {
  status: string;
  timestamp: string;
}

export interface ReadinessData {
  database: boolean;
  ready: boolean;
}

export interface LivenessData {
  alive: boolean;
}


// Note: Entity types (Product, Category, Tag, etc.) are exported from './models'
// which contains SurrealDB-aligned types with string IDs.
