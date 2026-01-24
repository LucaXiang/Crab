/**
 * API Models - Aligned with edge-server shared models
 *
 * These types match the Rust shared::models crate.
 * All IDs are strings (SurrealDB Thing format: "table:id")
 */

// ============ Common Types ============

/**
 * Print state (tri-state for products, binary for categories)
 * -1 = inherit from category (products only)
 *  0 = disabled
 *  1 = enabled
 */
export type PrintState = -1 | 0 | 1;

// ============ Tag ============

export interface Tag {
  id: string | null;
  name: string;
  color: string;
  display_order: number;
  /** 系统标签 */
  is_system: boolean;
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
  /** Kitchen print destination IDs */
  kitchen_print_destinations: string[];
  /** Label print destination IDs */
  label_print_destinations: string[];
  /** Whether kitchen printing is enabled for this category */
  is_kitchen_print_enabled: boolean;
  is_label_print_enabled: boolean;
  is_active: boolean;
  /** Whether this is a virtual category (filters by tags instead of direct assignment) */
  is_virtual: boolean;
  /** Tag IDs for virtual category filtering */
  tag_ids: string[];
  /** Match mode for virtual category: "any" or "all" */
  match_mode: 'any' | 'all';
  /** Whether to display this category in POS */
  is_display: boolean;
}

export interface CategoryCreate {
  name: string;
  sort_order?: number;
  /** Kitchen print destination IDs */
  kitchen_print_destinations?: string[];
  /** Label print destination IDs */
  label_print_destinations?: string[];
  /** Whether kitchen printing is enabled */
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
  /** Whether this is a virtual category */
  is_virtual?: boolean;
  /** Tag IDs for virtual category filtering */
  tag_ids?: string[];
  /** Match mode: "any" or "all" */
  match_mode?: 'any' | 'all';
  /** Whether to display this category in POS */
  is_display?: boolean;
}

export interface CategoryUpdate {
  name?: string;
  sort_order?: number;
  /** Kitchen print destination IDs */
  kitchen_print_destinations?: string[];
  /** Label print destination IDs */
  label_print_destinations?: string[];
  /** Whether kitchen printing is enabled */
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
  is_active?: boolean;
  /** Whether this is a virtual category */
  is_virtual?: boolean;
  /** Tag IDs for virtual category filtering */
  tag_ids?: string[];
  /** Match mode: "any" or "all" */
  match_mode?: 'any' | 'all';
  /** Whether to display this category in POS */
  is_display?: boolean;
}

// ============ Product ============

/** 嵌入式规格 (文档数据库风格) */
export interface EmbeddedSpec {
  name: string;
  /** 小票显示名称 */
  receipt_name?: string;
  /** Price in currency unit (e.g., 10.50 = €10.50) */
  price: number;
  display_order: number;
  is_default: boolean;
  /** 根规格 */
  is_root: boolean;
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
  /** 厨房打印目的地 */
  kitchen_print_destinations: string[];
  /** 标签打印目的地 */
  label_print_destinations: string[];
  /** 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_kitchen_print_enabled: PrintState;
  /** 标签打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_label_print_enabled: PrintState;
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
  /** 厨房打印目的地 */
  kitchen_print_destinations?: string[];
  /** 标签打印目的地 */
  label_print_destinations?: string[];
  /** 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_kitchen_print_enabled?: PrintState;
  /** 标签打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_label_print_enabled?: PrintState;
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
  /** 厨房打印目的地 */
  kitchen_print_destinations?: string[];
  /** 标签打印目的地 */
  label_print_destinations?: string[];
  /** 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_kitchen_print_enabled?: PrintState;
  /** 标签打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_label_print_enabled?: PrintState;
  is_active?: boolean;
  tags?: string[];
  /** 嵌入式规格 */
  specs?: EmbeddedSpec[];
}

/** Attribute binding with full attribute data */
export interface AttributeBindingFull {
  /** Relation ID (has_attribute edge) */
  id: string | null;
  /** Full attribute object */
  attribute: Attribute;
  is_required: boolean;
  display_order: number;
  default_option_idx?: number;
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
  /** 厨房打印目的地 */
  kitchen_print_destinations: string[];
  /** 标签打印目的地 */
  label_print_destinations: string[];
  /** 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_kitchen_print_enabled: PrintState;
  /** 标签打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_label_print_enabled: PrintState;
  is_active: boolean;
  /** Embedded specifications */
  specs: EmbeddedSpec[];
  /** Attribute bindings with full attribute data */
  attributes: AttributeBindingFull[];
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

export interface Attribute {
  id: string | null;
  name: string;
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

export interface AttributeBinding {
  id: string | null;
  /** Product or Category ID */
  from: string;
  /** Attribute ID */
  to: string;
  is_required: boolean;
  display_order: number;
  default_option_idx?: number;
}


// ============ Embedded Printer ============

export type PrinterType = 'network' | 'driver';
export type PrinterFormat = 'escpos' | 'label';

export interface EmbeddedPrinter {
  printer_type: PrinterType;
  /** Printer format: escpos (厨房单/小票) | label (标签) */
  printer_format: PrinterFormat;
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

export interface PriceRule {
  id: string | null;
  name: string;
  display_name: string;
  receipt_name: string;
  description: string | null;
  rule_type: RuleType;
  product_scope: ProductScope;
  target: string | null;
  /** Zone scope: "zone:all", "zone:retail", or specific zone ID like "zone:xxx" */
  zone_scope: string;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  priority: number;
  is_stackable: boolean;
  is_exclusive: boolean;
  // Time fields
  valid_from: string | null;        // ISO 8601 datetime string
  valid_until: string | null;       // ISO 8601 datetime string
  active_days: number[] | null;     // [0=Sunday, 1=Monday, ...]
  active_start_time: string | null; // HH:MM format
  active_end_time: string | null;   // HH:MM format
  is_active: boolean;
  created_by: string | null;
  created_at: string;               // ISO 8601 datetime string
}

export interface PriceRuleCreate {
  name: string;
  display_name: string;
  receipt_name: string;
  description?: string;
  rule_type: RuleType;
  product_scope: ProductScope;
  target?: string;
  /** Zone scope: "zone:all", "zone:retail", or specific zone ID */
  zone_scope?: string;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  priority?: number;
  is_stackable?: boolean;
  is_exclusive?: boolean;
  // Time fields
  valid_from?: string;        // ISO 8601 datetime string
  valid_until?: string;       // ISO 8601 datetime string
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
  /** Zone scope: "zone:all", "zone:retail", or specific zone ID */
  zone_scope?: string;
  adjustment_type?: AdjustmentType;
  adjustment_value?: number;
  priority?: number;
  is_stackable?: boolean;
  is_exclusive?: boolean;
  // Time fields
  valid_from?: string;        // ISO 8601 datetime string
  valid_until?: string;       // ISO 8601 datetime string
  active_days?: number[];     // [0=Sunday, 1=Monday, ...]
  active_start_time?: string; // HH:MM format
  active_end_time?: string;   // HH:MM format
  is_active?: boolean;
}

// ============ Employee ============

export interface Employee {
  id?: string;
  username: string;
  display_name: string;
  role: string;
  is_system: boolean;
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

/// Order event types for archived orders (matches db::models::order::OrderEventType)
export type OrderEventType =
  // Lifecycle
  | 'TABLE_OPENED'
  | 'ORDER_COMPLETED'
  | 'ORDER_VOIDED'
  | 'ORDER_RESTORED'
  // Items
  | 'ITEMS_ADDED'
  | 'ITEM_MODIFIED'
  | 'ITEM_REMOVED'
  | 'ITEM_RESTORED'
  // Payments
  | 'PAYMENT_ADDED'
  | 'PAYMENT_CANCELLED'
  // Split
  | 'ORDER_SPLIT'
  // Table operations
  | 'ORDER_MOVED'
  | 'ORDER_MOVED_OUT'
  | 'ORDER_MERGED'
  | 'ORDER_MERGED_OUT'
  | 'TABLE_REASSIGNED'
  // Other
  | 'ORDER_INFO_UPDATED'
  // Price Rules
  | 'RULE_SKIP_TOGGLED';

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
  id: string;
  username: string;
  display_name: string | null;
  role_id: string;
  role_name?: string;
  avatar: string | null;
  is_active: boolean;
  is_system: boolean;
  created_at: string;
  updated_at: string;
}

// ============ Product/Category Attribute Bindings ============

/**
 * ProductAttribute represents a AttributeBinding relation where 'in' is a Product
 */
export interface ProductAttribute extends AttributeBinding {
  /** The attribute details when fetched with relations */
  attribute?: Attribute;
}

/**
 * CategoryAttribute represents a AttributeBinding relation where 'in' is a Category
 */
export interface CategoryAttribute extends AttributeBinding {
  /** The attribute details when fetched with relations */
  attribute?: Attribute;
}

// ============ Kitchen Printing ============

/**
 * 打印上下文 (完整 JSON，模板自取所需字段)
 * Aligned with edge-server printing types
 */
export interface PrintItemContext {
  // 分类
  category_id: string;
  category_name: string;

  // 商品
  product_id: string;
  external_id: number | null; // 商品编号 (root spec)
  kitchen_name: string; // 厨房打印名称
  product_name: string; // 原始商品名

  // 规格
  spec_name: string | null;

  // 数量
  quantity: number;
  index: string | null; // 标签用："2/5"

  // 属性/做法
  options: string[];

  // 备注
  note: string | null;

  // 打印目的地
  kitchen_destinations: string[];
  label_destinations: string[];
}

/** 厨房订单菜品 */
export interface KitchenOrderItem {
  context: PrintItemContext;
}

/**
 * 一次点单的厨房记录（对应一个 ItemsAdded 事件）
 * Used for kitchen order display and reprint
 */
export interface KitchenOrder {
  /** Kitchen order ID (= event_id) */
  id: string;
  /** Parent order ID */
  order_id: string;
  /** Table name (if applicable) */
  table_name: string | null;
  /** Unix timestamp (seconds) */
  created_at: number;
  /** Items in this kitchen order */
  items: KitchenOrderItem[];
  /** Number of times this order has been printed */
  print_count: number;
}

/**
 * 标签打印记录（单品级别）
 * Each item in an order can have multiple labels (one per quantity unit)
 */
export interface LabelPrintRecord {
  /** Label record ID (UUID) */
  id: string;
  /** Parent order ID */
  order_id: string;
  /** Related kitchen order ID */
  kitchen_order_id: string;
  /** Table name (if applicable) */
  table_name: string | null;
  /** Unix timestamp (seconds) */
  created_at: number;
  /** Print context for this label */
  context: PrintItemContext;
  /** Number of times this label has been printed */
  print_count: number;
}

/** Response for kitchen order list */
export interface KitchenOrderListResponse {
  items: KitchenOrder[];
  total: number | null;
}
