/**
 * API Models - Aligned with edge-server shared models
 *
 * These types match the Rust shared::models crate.
 * All IDs are strings (SurrealDB Thing format: "table:id")
 */

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
  kitchen_printer: string | null;
  is_kitchen_print_enabled: boolean;
  is_label_print_enabled: boolean;
  is_active: boolean;
}

export interface CategoryCreate {
  name: string;
  sort_order?: number;
  kitchen_printer?: string;
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
}

export interface CategoryUpdate {
  name?: string;
  sort_order?: number;
  kitchen_printer?: string;
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
  is_active?: boolean;
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
  kitchen_printer: string | null;
  /** -1=inherit, 0=disabled, 1=enabled */
  is_kitchen_print_enabled: number;
  is_label_print_enabled: number;
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
  kitchen_printer?: string;
  is_kitchen_print_enabled?: number;
  is_label_print_enabled?: number;
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
  kitchen_printer?: string;
  is_kitchen_print_enabled?: number;
  is_label_print_enabled?: number;
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
  default_option_idx: number | null;
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
  kitchen_printer: string | null;
  /** -1=inherit, 0=disabled, 1=enabled */
  is_kitchen_print_enabled: number;
  is_label_print_enabled: number;
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
  value_code: string | null;
  /** Price modifier in cents */
  price_modifier: number;
  is_default: boolean;
  display_order: number;
  is_active: boolean;
  receipt_name: string | null;
}

export interface Attribute {
  id: string | null;
  name: string;
  /** single_select or multi_select */
  attr_type: string;
  display_order: number;
  is_active: boolean;
  show_on_receipt: boolean;
  receipt_name: string | null;
  kitchen_printer: string | null;
  is_global: boolean;
  options: AttributeOption[];
}

export interface AttributeCreate {
  name: string;
  attr_type?: string;
  display_order?: number;
  show_on_receipt?: boolean;
  receipt_name?: string;
  kitchen_printer?: string;
  is_global?: boolean;
  options?: AttributeOption[];
}

export interface AttributeUpdate {
  name?: string;
  attr_type?: string;
  display_order?: number;
  is_active?: boolean;
  show_on_receipt?: boolean;
  receipt_name?: string;
  kitchen_printer?: string;
  is_global?: boolean;
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
  default_option_idx: number | null;
}

// ============ Kitchen Printer ============

export interface KitchenPrinter {
  id: string | null;
  name: string;
  printer_name: string | null;
  description: string | null;
  is_active: boolean;
}

export interface KitchenPrinterCreate {
  name: string;
  printer_name?: string;
  description?: string;
}

export interface KitchenPrinterUpdate {
  name?: string;
  printer_name?: string;
  description?: string;
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
  time_mode: TimeMode;
  start_time: string | null;
  end_time: string | null;
  schedule_config: ScheduleConfig | null;
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
  time_mode?: TimeMode;
  start_time?: string;
  end_time?: string;
  schedule_config?: ScheduleConfig;
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
  time_mode?: TimeMode;
  start_time?: string;
  end_time?: string;
  schedule_config?: ScheduleConfig;
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

export type OrderStatus = 'OPEN' | 'PAID' | 'VOID';

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
  status: OrderStatus;
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
  status: OrderStatus;
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

/** Alias for PriceRule (legacy name) */
export type PriceAdjustmentRule = PriceRule;
export type PriceAdjustmentRuleCreate = PriceRuleCreate;
export type PriceAdjustmentRuleUpdate = PriceRuleUpdate;

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
