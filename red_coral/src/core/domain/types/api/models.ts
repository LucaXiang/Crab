/**
 * API Models - Aligned with edge-server shared models
 *
 * These types match the Rust shared::models crate.
 * All IDs are strings (SurrealDB Thing format: "table:id")
 */

// ============ Common Types ============

/**
 * Label print state for products (tri-state inheritance)
 * -1 = inherit from category
 *  0 = disabled
 *  1 = enabled
 */
export type LabelPrintState = -1 | 0 | 1;

// ============ Tag ============

export interface Tag {
  id: string | null;
  name: string;
  color: string;
  display_order: number;
  is_active: boolean;
}

export interface TagCreate {
  name: string;
  color?: string;
  display_order?: number;
}

export interface TagUpdate {
  name?: string;
  color?: string;
  display_order?: number;
  is_active?: boolean;
}

// ============ Category ============

export interface Category {
  id: string | null;
  name: string;
  sort_order: number;
  print_destinations: string[];
  is_label_print_enabled: boolean;
  is_active: boolean;
  /** Whether this is a virtual category (filters by tags instead of direct assignment) */
  is_virtual: boolean;
  /** Tag IDs for virtual category filtering */
  tag_ids: string[];
  /** Match mode for virtual category: "any" or "all" */
  match_mode: 'any' | 'all';
}

export interface CategoryCreate {
  name: string;
  sort_order?: number;
  print_destinations?: string[];
  is_label_print_enabled?: boolean;
  /** Whether this is a virtual category */
  is_virtual?: boolean;
  /** Tag IDs for virtual category filtering */
  tag_ids?: string[];
  /** Match mode: "any" or "all" */
  match_mode?: 'any' | 'all';
}

export interface CategoryUpdate {
  name?: string;
  sort_order?: number;
  print_destinations?: string[];
  is_label_print_enabled?: boolean;
  is_active?: boolean;
  /** Whether this is a virtual category */
  is_virtual?: boolean;
  /** Tag IDs for virtual category filtering */
  tag_ids?: string[];
  /** Match mode: "any" or "all" */
  match_mode?: 'any' | 'all';
}

// ============ Product ============

/** 嵌入式规格 (文档数据库风格) */
export interface EmbeddedSpec {
  name: string;
  /** Price in cents */
  price: number;
  display_order: number;
  is_default: boolean;
  is_active: boolean;
  external_id: number | null;
}

export interface Product {
  id: string | null;
  name: string;
  image: string;
  category: string;
  sort_order: number;
  tax_rate: number;
  receipt_name: string | null;
  kitchen_print_name: string | null;
  print_destinations: string[];
  is_label_print_enabled: LabelPrintState;
  is_active: boolean;
  /** Tag references (String IDs) */
  tags: string[];
  /** 嵌入式规格数组 */
  specs: EmbeddedSpec[];
}

export interface ProductCreate {
  name: string;
  image?: string;
  category: string;
  sort_order?: number;
  tax_rate?: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  print_destinations?: string[];
  is_label_print_enabled?: LabelPrintState;
  tags?: string[];
  /** 嵌入式规格 */
  specs: EmbeddedSpec[];
}

export interface ProductUpdate {
  name?: string;
  image?: string;
  category?: string;
  sort_order?: number;
  tax_rate?: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  print_destinations?: string[];
  is_label_print_enabled?: LabelPrintState;
  is_active?: boolean;
  tags?: string[];
  /** 嵌入式规格 */
  specs?: EmbeddedSpec[];
}

/** Product attribute binding with full attribute data */
export interface ProductAttributeBinding {
  /** Relation ID (has_attribute edge) */
  id: string | null;
  /** Full attribute object */
  attribute: Attribute;
  is_required: boolean;
  display_order: number;
}

/** Full product with all related data */
export interface ProductFull {
  id: string | null;
  name: string;
  image: string;
  category: string;
  sort_order: number;
  tax_rate: number;
  receipt_name: string | null;
  kitchen_print_name: string | null;
  print_destinations: string[];
  is_label_print_enabled: LabelPrintState;
  is_active: boolean;
  /** Embedded specifications */
  specs: EmbeddedSpec[];
  /** Attribute bindings with full attribute data */
  attributes: ProductAttributeBinding[];
  /** Tags attached to this product */
  tags: Tag[];
}

// ============ Attribute ============

export interface AttributeOption {
  name: string;
  /** Price modifier in cents */
  price_modifier: number;
  display_order: number;
  is_active: boolean;
  receipt_name: string | null;
  kitchen_print_name: string | null;
}

export type AttributeScope = 'global' | 'inherited';

export interface Attribute {
  id: string | null;
  name: string;
  scope: AttributeScope;
  excluded_categories: string[];
  is_multi_select: boolean;
  max_selections: number | null;
  default_option_idx: number | null;
  display_order: number;
  is_active: boolean;
  show_on_receipt: boolean;
  receipt_name: string | null;
  show_on_kitchen_print: boolean;
  kitchen_print_name: string | null;
  options: AttributeOption[];
}

export interface AttributeCreate {
  name: string;
  scope?: AttributeScope;
  excluded_categories?: string[];
  is_multi_select?: boolean;
  max_selections?: number;
  default_option_idx?: number;
  display_order?: number;
  show_on_receipt?: boolean;
  receipt_name?: string;
  show_on_kitchen_print?: boolean;
  kitchen_print_name?: string;
  options?: AttributeOption[];
}

export interface AttributeUpdate {
  name?: string;
  scope?: AttributeScope;
  excluded_categories?: string[];
  is_multi_select?: boolean;
  max_selections?: number;
  default_option_idx?: number;
  display_order?: number;
  is_active?: boolean;
  show_on_receipt?: boolean;
  receipt_name?: string;
  show_on_kitchen_print?: boolean;
  kitchen_print_name?: string;
  options?: AttributeOption[];
}

export interface HasAttribute {
  id: string | null;
  /** Product or Category ID */
  from: string;
  /** Attribute ID */
  to: string;
  is_required: boolean;
  display_order: number;
}

// ============ Embedded Printer ============

export type PrinterType = 'network' | 'driver';

export interface EmbeddedPrinter {
  printer_type: PrinterType;
  ip?: string;
  port?: number;
  driver_name?: string;
  priority: number;
  is_active: boolean;
}

// ============ Print Destination ============

export interface PrintDestination {
  id?: string;
  name: string;
  description?: string;
  printers: EmbeddedPrinter[];
  is_active: boolean;
}

export interface PrintDestinationCreate {
  name: string;
  description?: string;
  printers?: EmbeddedPrinter[];
  is_active?: boolean;
}

export interface PrintDestinationUpdate {
  name?: string;
  description?: string;
  printers?: EmbeddedPrinter[];
  is_active?: boolean;
}

// ============ Zone ============

export interface Zone {
  id: string | null;
  name: string;
  description: string | null;
  is_active: boolean;
}

export interface ZoneCreate {
  name: string;
  description?: string;
}

export interface ZoneUpdate {
  name?: string;
  description?: string;
  is_active?: boolean;
}

// ============ Dining Table ============

export interface DiningTable {
  id: string | null;
  name: string;
  zone: string;
  capacity: number;
  is_active: boolean;
}

export interface DiningTableCreate {
  name: string;
  zone: string;
  capacity?: number;
}

export interface DiningTableUpdate {
  name?: string;
  zone?: string;
  capacity?: number;
  is_active?: boolean;
}

// ============ Price Rule ============

export type RuleType = 'DISCOUNT' | 'SURCHARGE';
export type ProductScope = 'GLOBAL' | 'CATEGORY' | 'TAG' | 'PRODUCT';
export type AdjustmentType = 'PERCENTAGE' | 'FIXED_AMOUNT';
export type TimeMode = 'ALWAYS' | 'SCHEDULE' | 'ONETIME';

export interface ScheduleConfig {
  days_of_week: number[] | null;
  start_time: string | null;
  end_time: string | null;
}

export interface PriceRule {
  id: string | null;
  name: string;
  display_name: string;
  receipt_name: string;
  description: string | null;
  rule_type: RuleType;
  product_scope: ProductScope;
  target: string | null;
  zone_scope: number;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  priority: number;
  is_stackable: boolean;
  is_exclusive: boolean;
  time_mode: TimeMode;
  start_time: string | null;
  end_time: string | null;
  schedule_config: ScheduleConfig | null;
  // Time fields (snake_case to match Rust)
  valid_from: number | null;        // milliseconds since epoch
  valid_until: number | null;       // milliseconds since epoch
  active_days: number[] | null;     // [0=Sunday, 1=Monday, ...]
  active_start_time: string | null; // HH:MM format
  active_end_time: string | null;   // HH:MM format
  is_active: boolean;
  created_by: string | null;
}

export interface PriceRuleCreate {
  name: string;
  display_name: string;
  receipt_name: string;
  description?: string;
  rule_type: RuleType;
  product_scope: ProductScope;
  target?: string;
  zone_scope?: number;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  priority?: number;
  is_stackable?: boolean;
  is_exclusive?: boolean;
  time_mode?: TimeMode;
  start_time?: string;
  end_time?: string;
  schedule_config?: ScheduleConfig;
  // Time fields (snake_case to match Rust)
  valid_from?: number;        // milliseconds since epoch
  valid_until?: number;       // milliseconds since epoch
  active_days?: number[];     // [0=Sunday, 1=Monday, ...]
  active_start_time?: string; // HH:MM format
  active_end_time?: string;   // HH:MM format
  created_by?: string;
}

export interface PriceRuleUpdate {
  name?: string;
  display_name?: string;
  receipt_name?: string;
  description?: string;
  rule_type?: RuleType;
  product_scope?: ProductScope;
  target?: string;
  zone_scope?: number;
  adjustment_type?: AdjustmentType;
  adjustment_value?: number;
  priority?: number;
  is_stackable?: boolean;
  is_exclusive?: boolean;
  time_mode?: TimeMode;
  start_time?: string;
  end_time?: string;
  schedule_config?: ScheduleConfig;
  // Time fields (snake_case to match Rust)
  valid_from?: number;        // milliseconds since epoch
  valid_until?: number;       // milliseconds since epoch
  active_days?: number[];     // [0=Sunday, 1=Monday, ...]
  active_start_time?: string; // HH:MM format
  active_end_time?: string;   // HH:MM format
  is_active?: boolean;
}

// ============ Employee ============

export interface EmployeeResponse {
  id: string;
  username: string;
  role: string;
  is_active: boolean;
}

export interface EmployeeCreate {
  username: string;
  password: string;
  role: string;
}

export interface EmployeeUpdate {
  username?: string;
  password?: string;
  role?: string;
  is_active?: boolean;
}

// ============ Order ============

/** REST API Order status (matches Rust backend) */
export type OrderApiStatus = 'OPEN' | 'PAID' | 'VOID';

export interface OrderItemAttribute {
  attr_id: string;
  option_idx: number;
  name: string;
  price: number;
}

export interface OrderItem {
  spec: string;
  name: string;
  spec_name: string | null;
  price: number;
  quantity: number;
  attributes: OrderItemAttribute[];
  discount_amount: number;
  surcharge_amount: number;
  note: string | null;
  is_sent: boolean;
}

export interface OrderPayment {
  method: string;
  amount: number;
  time: string;
  reference: string | null;
}

export interface Order {
  id: string | null;
  receipt_number: string;
  zone_name: string | null;
  table_name: string | null;
  status: OrderApiStatus;
  start_time: string;
  end_time: string | null;
  guest_count: number | null;
  total_amount: number;
  paid_amount: number;
  discount_amount: number;
  surcharge_amount: number;
  items: OrderItem[];
  payments: OrderPayment[];
  prev_hash: string;
  curr_hash: string;
  created_at: string | null;
}

export type OrderEventType =
  | 'CREATED'
  | 'ITEM_ADDED'
  | 'ITEM_REMOVED'
  | 'ITEM_UPDATED'
  | 'PAID'
  | 'PARTIAL_PAID'
  | 'VOID'
  | 'REFUND'
  | 'TABLE_CHANGED'
  | 'GUEST_COUNT_CHANGED';

export interface OrderEvent {
  id: string | null;
  event_type: OrderEventType;
  timestamp: string;
  data: unknown | null;
  prev_hash: string;
  curr_hash: string;
}

export interface OrderCreate {
  receipt_number: string;
  zone_name?: string;
  table_name?: string;
  guest_count?: number;
  prev_hash: string;
}

export interface OrderAddItem {
  spec: string;
  name: string;
  spec_name?: string;
  price: number;
  quantity: number;
  attributes?: OrderItemAttribute[];
  note?: string;
}

export interface OrderAddPayment {
  method: string;
  amount: number;
  reference?: string;
}

export interface OrderUpdateTotals {
  total_amount: number;
  discount_amount: number;
  surcharge_amount: number;
}

export interface OrderUpdateStatus {
  status: OrderApiStatus;
}

export interface OrderUpdateHash {
  prev_hash: string;
  curr_hash: string;
}

export interface OrderAddEvent {
  event_type: OrderEventType;
  data?: unknown;
  prev_hash: string;
  curr_hash: string;
}

export interface OrderRemoveItem {
  index: number;
}

export interface InitGenesisRequest {
  genesis_hash: string;
}

export interface UpdateLastOrderRequest {
  order_id: string;
  order_hash: string;
}

export interface UpdateSyncStateRequest {
  synced_up_to_id: string;
  synced_up_to_hash: string;
}

// ============ Type Aliases for Backward Compatibility ============

/** Alias for DiningTable */
export type Table = DiningTable;
export type TableCreate = DiningTableCreate;
export type TableUpdate = DiningTableUpdate;

/** Alias for Attribute (legacy name) */
export type AttributeTemplate = Attribute;
export type AttributeTemplateCreate = AttributeCreate;
export type AttributeTemplateUpdate = AttributeUpdate;

/** Alias for OrderPayment */
export type Payment = OrderPayment;

// ============ Role ============

export interface Role {
  id: string;
  name: string;
  display_name: string;
  description: string | null;
  is_system: boolean;
  is_active: boolean;
}

export interface RoleCreate {
  name: string;
  display_name: string;
  description?: string;
}

export interface RoleUpdate {
  name?: string;
  display_name?: string;
  description?: string;
  is_active?: boolean;
}

export interface RolePermission {
  role_id: string;
  permission: string;
}

export interface RoleListData {
  roles: Role[];
}

export interface RolePermissionListData {
  permissions: RolePermission[];
}

// ============ User (Frontend representation) ============

/**
 * User type for frontend authentication state.
 * Maps to CurrentUser from login response with additional fields.
 * Note: password_hash is NOT included - it should never be sent to frontend.
 */
export interface User {
  id: number;
  uuid: string;
  username: string;
  display_name: string | null;
  role_id: number;
  role_name?: string;
  avatar: string | null;
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

// ============ Product/Category Attribute Bindings ============

/**
 * ProductAttribute represents a HasAttribute relation where 'in' is a Product
 */
export interface ProductAttribute extends HasAttribute {
  /** The attribute details when fetched with relations */
  attribute?: Attribute;
}

/**
 * CategoryAttribute represents a HasAttribute relation where 'in' is a Category
 */
export interface CategoryAttribute extends HasAttribute {
  /** The attribute details when fetched with relations */
  attribute?: Attribute;
}
