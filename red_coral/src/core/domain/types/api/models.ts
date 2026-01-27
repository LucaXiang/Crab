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

// NOTE: Product is now an alias for ProductFull
// Backend always returns full product data including attributes and tags
// This simplifies type handling across the frontend
export type Product = ProductFull;

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

// ============ Type Aliases for Backward Compatibility ============

/** Alias for DiningTable */
export type Table = DiningTable;
export type TableCreate = DiningTableCreate;
export type TableUpdate = DiningTableUpdate;

/** Alias for Attribute (legacy name) */
export type AttributeTemplate = Attribute;
export type AttributeTemplateCreate = AttributeCreate;
export type AttributeTemplateUpdate = AttributeUpdate;

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
/**
 * User for auth store - aligned with shared::client::UserInfo
 * Note: created_at/updated_at are frontend-only fields for compatibility
 */
export interface User {
  id: string;
  username: string;
  display_name: string;
  role_id: string;
  role_name: string;
  permissions: string[];
  is_system: boolean;
  is_active: boolean;
  // Frontend-only fields (not from backend)
  created_at?: string;
  updated_at?: string;
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

// ============ Store Info ============

/**
 * Store information (singleton per tenant)
 * Used for receipts, labels, and business info display
 */
export interface StoreInfo {
  id: string | null;
  name: string;
  address: string;
  /** Tax identification number (NIF) */
  nif: string;
  logo_url: string | null;
  phone: string | null;
  email: string | null;
  website: string | null;
  /**
   * Business day cutoff time (HH:MM format, e.g., "06:00")
   * Used for shift cross-day detection and daily report calculation
   * Default "00:00" (midnight), bars/nightclubs can set to "06:00"
   */
  business_day_cutoff: string;
  created_at: string | null;
  updated_at: string | null;
}

export interface StoreInfoUpdate {
  name?: string;
  address?: string;
  nif?: string;
  logo_url?: string | null;
  phone?: string | null;
  email?: string | null;
  website?: string | null;
  /** Business day cutoff time (HH:MM format) */
  business_day_cutoff?: string;
}

// ============ Label Template (API DTOs) ============

// Re-export LabelTemplate from print types for convenience
export type { LabelTemplate, LabelField } from '../print/labelTemplate';

export interface LabelTemplateCreate {
  name: string;
  description?: string;
  width: number;
  height: number;
  padding?: number;
  fields?: import('../print/labelTemplate').LabelField[];
  is_default?: boolean;
  is_active?: boolean;
  width_mm?: number;
  height_mm?: number;
  padding_mm_x?: number;
  padding_mm_y?: number;
  render_dpi?: number;
  test_data?: string;
}

export interface LabelTemplateUpdate {
  name?: string;
  description?: string;
  width?: number;
  height?: number;
  padding?: number;
  fields?: import('../print/labelTemplate').LabelField[];
  is_default?: boolean;
  is_active?: boolean;
  width_mm?: number;
  height_mm?: number;
  padding_mm_x?: number;
  padding_mm_y?: number;
  render_dpi?: number;
  test_data?: string;
}

// ============ Shift (班次管理) ============

/** Shift status */
export type ShiftStatus = 'OPEN' | 'CLOSED';

/**
 * Shift record - represents an operator's work shift
 * Used for cash tracking and shift management
 */
export interface Shift {
  id: string | null;
  /** Operator employee ID (RecordId format: "employee:xxx") */
  operator_id: string;
  /** Operator display name */
  operator_name: string;
  /** Shift status */
  status: ShiftStatus;
  /** Shift start time (ISO 8601) */
  start_time: string;
  /** Shift end time (ISO 8601), null if still open */
  end_time: string | null;
  /** Starting cash amount */
  starting_cash: number;
  /** Expected cash amount (starting + cash payments received) */
  expected_cash: number;
  /** Actual cash counted at close */
  actual_cash: number | null;
  /** Cash variance (actual - expected) */
  cash_variance: number | null;
  /** Whether shift was closed abnormally (power failure, etc.) */
  abnormal_close: boolean;
  /** Last heartbeat timestamp */
  last_active_at: string | null;
  /** Notes */
  note: string | null;
  created_at: string | null;
  updated_at: string | null;
}

export interface ShiftCreate {
  /** Operator employee ID */
  operator_id: string;
  /** Operator display name */
  operator_name: string;
  /** Starting cash amount (default 0) */
  starting_cash?: number;
  /** Notes */
  note?: string;
}

export interface ShiftClose {
  /** Actual cash counted */
  actual_cash: number;
  /** Notes */
  note?: string;
}

export interface ShiftForceClose {
  /** Notes */
  note?: string;
}

export interface ShiftUpdate {
  /** Update starting cash (only when OPEN) */
  starting_cash?: number;
  /** Notes */
  note?: string;
}

// ============ Daily Report (日结报告) ============

/** Tax breakdown by rate (Spain: 0%, 4%, 10%, 21%) */
export interface TaxBreakdown {
  /** Tax rate (0, 4, 10, 21) */
  tax_rate: number;
  /** Net amount (before tax) */
  net_amount: number;
  /** Tax amount */
  tax_amount: number;
  /** Gross amount (after tax) */
  gross_amount: number;
  /** Number of orders with this tax rate */
  order_count: number;
}

/** Payment method breakdown */
export interface PaymentMethodBreakdown {
  /** Payment method name */
  method: string;
  /** Total amount */
  amount: number;
  /** Number of payments */
  count: number;
}

/**
 * Daily Report - end-of-day settlement report
 * Contains aggregated sales data for a business date
 */
export interface DailyReport {
  id: string | null;
  /** Business date (YYYY-MM-DD format) */
  business_date: string;
  /** Total number of orders */
  total_orders: number;
  /** Completed orders count */
  completed_orders: number;
  /** Voided orders count */
  void_orders: number;
  /** Total sales amount */
  total_sales: number;
  /** Total paid amount */
  total_paid: number;
  /** Total unpaid amount */
  total_unpaid: number;
  /** Voided order total amount */
  void_amount: number;
  /** Total tax collected */
  total_tax: number;
  /** Total discount applied */
  total_discount: number;
  /** Total surcharge applied */
  total_surcharge: number;
  /** Tax breakdown by rate */
  tax_breakdowns: TaxBreakdown[];
  /** Payment breakdown by method */
  payment_breakdowns: PaymentMethodBreakdown[];
  /** When the report was generated (ISO 8601) */
  generated_at: string | null;
  /** Who generated the report (employee ID) */
  generated_by_id: string | null;
  /** Who generated the report (name) */
  generated_by_name: string | null;
  /** Notes */
  note: string | null;
}

export interface DailyReportGenerate {
  /** Business date to generate report for (YYYY-MM-DD) */
  business_date: string;
  /** Notes */
  note?: string;
}
