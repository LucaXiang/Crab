export * from './error';

// API Response types - aligned with Rust server
export interface ApiResponse<T> {
  error_code: string | null;
  message: string;
  data?: T;
}

export interface DeleteResponse {
  deleted: number;
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
  total: number;
  page?: number;
  page_size?: number;
}

export interface CreateProductRequest {
  name: string;
  image?: string;
  category_id?: number;
  sort_order?: number;
  tax_rate?: number;
  has_multi_spec?: boolean;
  receipt_name?: string;
  kitchen_print_name?: string;
  kitchen_printer_id?: number;
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
  specifications?: CreateSpecificationRequest[];
  specifications_to_delete?: string[];
}

export interface UpdateProductRequest {
  name?: string;
  image?: string;
  category_id?: number;
  sort_order?: number;
  tax_rate?: number;
  has_multi_spec?: boolean;
  receipt_name?: string;
  kitchen_print_name?: string;
  kitchen_printer_id?: number;
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
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
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
}

export interface UpdateCategoryRequest {
  name?: string;
  sort_order?: number;
  kitchen_printer_id?: number;
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
export interface AttributeTemplateData extends AttributeTemplate {
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

export interface RolePermissionListData {
  permissions: RolePermission[];
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
  zone_id: number;
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
  category_id: number;
  attribute_id: number;
  is_required?: boolean;
  display_order?: number;
  default_option_id?: number;
}

export interface UpdateCategoryAttributeRequest {
  is_required?: boolean;
  display_order?: number;
  default_option_id?: number;
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

// Entity types matching the Rust project
export interface Role {
  id: number;
  uuid: string;
  name: string;
  display_name: string;
  description: string | null;
  is_system: boolean;
  created_at: string;
  updated_at: string;
}

export interface RolePermission {
  role_id: number;
  permission: string;
  created_at: string;
}

export interface User {
  id: number;
  uuid: string;
  username: string;
  display_name: string | null;
  password_hash: string;
  role_id: number;
  avatar: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface KitchenPrinter {
  id: number;
  uuid: string;
  name: string;
  printer_name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface Category {
  id: number;
  uuid: string;
  name: string;
  sort_order: number;
  kitchen_printer_id: number | null;
  is_kitchen_print_enabled: boolean;
  is_label_print_enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface Product {
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
}

export interface ProductSpecification {
  id: number;
  uuid: string;
  product_id: number;
  name: string;
  price: number;
  display_order: number;
  is_default: boolean;
  is_active: boolean;
  is_root: boolean;
  external_id: string | null;
  created_at: string;
  updated_at: string;
}

export interface Tag {
  id: number;
  uuid: string;
  name: string;
  color: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface SpecificationTag {
  specification_id: number;
  tag_id: number;
  created_at: string;
}

export interface AttributeTemplate {
  id: number;
  uuid: string;
  name: string;
  type_: string;
  display_order: number;
  is_active: boolean;
  show_on_receipt: boolean;
  receipt_name: string | null;
  kitchen_printer_id: number | null;
  is_global: boolean;
  created_at: string;
  updated_at: string;
}

export interface AttributeOption {
  id: number;
  uuid: string;
  attribute_id: number;
  name: string;
  value_code: string;
  price_modifier: number;
  is_default: boolean;
  display_order: number;
  is_active: boolean;
  receipt_name: string | null;
  created_at: string;
  updated_at: string;
}

export interface ProductAttribute {
  id: number;
  uuid: string;
  product_id: number;
  attribute_id: number;
  is_required: boolean;
  display_order: number;
  default_option_id: number | null;
  created_at: string;
  updated_at: string;
}

export interface CategoryAttribute {
  id: number;
  uuid: string;
  category_id: number;
  attribute_id: number;
  is_required: boolean;
  display_order: number;
  default_option_id: number | null;
  created_at: string;
  updated_at: string;
}

export interface Zone {
  id: number;
  uuid: string;
  name: string;
  description: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface Table {
  id: number;
  uuid: string;
  name: string;
  zone_id: number;
  capacity: number;
  created_at: string;
  updated_at: string;
}

export interface PriceAdjustmentRule {
  id: number;
  uuid: string;
  name: string;
  display_name: string;
  receipt_name: string;
  description: string;
  rule_type: PriceAdjustmentRuleType;
  product_scope: ProductScope;
  target_id: number;
  zone_scope: ZoneScope;
  adjustment_type: PriceAdjustmentType;
  adjustment_value: number;
  priority: number;
  is_stackable: boolean;
  time_mode: PriceAdjustmentTimeMode;
  start_time: string;
  end_time: string;
  schedule_config_json: string;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface Order {
  id: number;
  uuid: string;
  order_number: string;
  status: OrderStatus;
  total_amount: number;
  discount_amount: number;
  tax_amount: number;
  paid_amount: number;
  customer_id: number | null;
  table_id: number | null;
  note: string | null;
  created_at: string;
  updated_at: string;
}

export interface OrderEvent {
  id: number;
  uuid: string;
  order_id: number;
  status: OrderStatus;
  note: string | null;
  created_at: string;
}

export interface OrderItem {
  id: number;
  uuid: string;
  order_id: number;
  product_id: number;
  specification_id: number | null;
  name: string;
  quantity: number;
  unit_price: number;
  total_price: number;
  note: string | null;
  created_at: string;
  updated_at: string;
}

export interface OrderItemOption {
  id: number;
  uuid: string;
  order_item_id: number;
  attribute_id: number;
  option_id: number;
  name: string;
  price_modifier: number;
  created_at: string;
}

export interface Payment {
  id: number;
  uuid: string;
  order_id: number;
  amount: number;
  payment_method: string;
  status: string;
  transaction_id: string | null;
  note: string | null;
  created_at: string;
  updated_at: string;
}

export interface SystemState {
  id: number;
  key: string;
  value: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface AuditLog {
  id: number;
  uuid: string;
  user_id: number | null;
  category: AuditCategory;
  severity: AuditSeverity;
  action: string;
  resource: string | null;
  resource_id: string | null;
  details: string | null;
  ip_address: string | null;
  created_at: string;
}

// Enums
export enum PriceAdjustmentRuleType {
  Discount = 'discount',
  Promotion = 'promotion',
  TimeBased = 'time_based',
  CategoryBased = 'category_based',
}

export enum ProductScope {
  All = 'all',
  Specific = 'specific',
  Category = 'category',
}

export enum ZoneScope {
  All = 0,
  Specific = 1,
}

export enum PriceAdjustmentType {
  Fixed = 'fixed',
  Percentage = 'percentage',
}

export enum PriceAdjustmentTimeMode {
  None = 'none',
  Daily = 'daily',
  Weekly = 'weekly',
  Range = 'range',
}

export enum OrderStatus {
  Pending = 'pending',
  Confirmed = 'confirmed',
  Preparing = 'preparing',
  Ready = 'ready',
  Served = 'served',
  Completed = 'completed',
  Cancelled = 'cancelled',
}

export enum AuditCategory {
  Auth = 'auth',
  User = 'user',
  Order = 'order',
  Product = 'product',
  Payment = 'payment',
  System = 'system',
}

export enum AuditSeverity {
  Info = 'info',
  Warning = 'warning',
  Error = 'error',
}
