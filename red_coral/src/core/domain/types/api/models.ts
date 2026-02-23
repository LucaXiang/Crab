/**
 * API Models - Aligned with edge-server shared models
 *
 * These types match the Rust shared::models crate.
 * All entity IDs are numbers (SQLite INTEGER PRIMARY KEY).
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
  id: number;
  name: string;
  color: string;
  display_order: number;
  is_active: boolean;
  /** 系统标签 */
  is_system: boolean;
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
  id: number;
  name: string;
  sort_order: number;
  /** Kitchen print destination IDs */
  kitchen_print_destinations: number[];
  /** Label print destination IDs */
  label_print_destinations: number[];
  /** Whether kitchen printing is enabled for this category */
  is_kitchen_print_enabled: boolean;
  is_label_print_enabled: boolean;
  is_active: boolean;
  /** Whether this is a virtual category (filters by tags instead of direct assignment) */
  is_virtual: boolean;
  /** Tag IDs for virtual category filtering */
  tag_ids: number[];
  /** Match mode for virtual category: "any" or "all" */
  match_mode: 'any' | 'all';
  /** Whether to display this category in POS */
  is_display: boolean;
}

export interface CategoryCreate {
  name: string;
  sort_order?: number;
  /** Kitchen print destination IDs */
  kitchen_print_destinations?: number[];
  /** Label print destination IDs */
  label_print_destinations?: number[];
  /** Whether kitchen printing is enabled */
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
  /** Whether this is a virtual category */
  is_virtual?: boolean;
  /** Tag IDs for virtual category filtering */
  tag_ids?: number[];
  /** Match mode: "any" or "all" */
  match_mode?: 'any' | 'all';
  /** Whether to display this category in POS */
  is_display?: boolean;
}

export interface CategoryUpdate {
  name?: string;
  sort_order?: number;
  /** Kitchen print destination IDs */
  kitchen_print_destinations?: number[];
  /** Label print destination IDs */
  label_print_destinations?: number[];
  /** Whether kitchen printing is enabled */
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
  is_active?: boolean;
  /** Whether this is a virtual category */
  is_virtual?: boolean;
  /** Tag IDs for virtual category filtering */
  tag_ids?: number[];
  /** Match mode: "any" or "all" */
  match_mode?: 'any' | 'all';
  /** Whether to display this category in POS */
  is_display?: boolean;
}

// ============ Product ============

/** Product spec (from product_spec table, id/product_id present on read) */
export interface ProductSpec {
  id?: number;
  product_id?: number;
  name: string;
  /** 小票显示名称 */
  receipt_name: string | null;
  /** Price in currency unit (e.g., 10.50 = €10.50) */
  price: number;
  display_order: number;
  is_default: boolean;
  /** 根规格 */
  is_root: boolean;
  is_active: boolean;
}

/** Product spec input (for create/update, without id/product_id) */
export interface ProductSpecInput {
  name: string;
  receipt_name?: string | null;
  price: number;
  display_order: number;
  is_default: boolean;
  is_root: boolean;
  is_active: boolean;
}

// NOTE: Product is now an alias for ProductFull
// Backend always returns full product data including attributes and tags
// This simplifies type handling across the frontend
export type Product = ProductFull;

export interface ProductCreate {
  name: string;
  image?: string;
  category_id: number;
  sort_order?: number;
  tax_rate?: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  /** 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_kitchen_print_enabled?: PrintState;
  /** 标签打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_label_print_enabled?: PrintState;
  /** 菜品编号 (POS 集成，全局唯一) */
  external_id?: number | null;
  tags?: number[];
  /** 规格列表 */
  specs: ProductSpecInput[];
}

export interface ProductUpdate {
  name?: string;
  image?: string;
  category_id?: number;
  sort_order?: number;
  tax_rate?: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  /** 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_kitchen_print_enabled?: PrintState;
  /** 标签打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_label_print_enabled?: PrintState;
  is_active?: boolean;
  /** 菜品编号 (POS 集成，全局唯一) */
  external_id?: number | null;
  tags?: number[];
  /** 规格列表 */
  specs?: ProductSpecInput[];
}

/** Attribute binding with full attribute data */
export interface AttributeBindingFull {
  /** Binding ID */
  id: number;
  /** Full attribute object */
  attribute: Attribute;
  is_required: boolean;
  display_order: number;
  default_option_ids?: number[];
  /** Whether this binding is inherited from the product's category */
  is_inherited?: boolean;
}


/** Full product with all related data */
export interface ProductFull {
  id: number;
  name: string;
  image: string;
  category_id: number;
  sort_order: number;
  tax_rate: number;
  receipt_name: string | null;
  kitchen_print_name: string | null;
  /** 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_kitchen_print_enabled: PrintState;
  /** 标签打印启用状态 (-1=继承, 0=禁用, 1=启用) */
  is_label_print_enabled: PrintState;
  is_active: boolean;
  /** 菜品编号 (POS 集成，全局唯一) */
  external_id: number | null;
  /** Product specs */
  specs: ProductSpec[];
  /** Attribute bindings with full attribute data */
  attributes: AttributeBindingFull[];
  /** Tags attached to this product */
  tags: Tag[];
}

// ============ Attribute ============

export interface AttributeOption {
  id: number;
  attribute_id: number;
  name: string;
  /** Price modifier in currency unit (e.g., 2.50 = €2.50) */
  price_modifier: number;
  display_order: number;
  is_active: boolean;
  receipt_name: string | null;
  kitchen_print_name: string | null;
  /** Enable quantity control for this option (default: false) */
  enable_quantity: boolean;
  /** Maximum quantity allowed (only effective when enable_quantity=true) */
  max_quantity: number | null;
}

/** Attribute option input (for create/update, without id/attribute_id/is_active) */
export interface AttributeOptionInput {
  name: string;
  price_modifier?: number;
  display_order?: number;
  receipt_name?: string | null;
  kitchen_print_name?: string | null;
  enable_quantity?: boolean;
  max_quantity?: number | null;
}

export interface Attribute {
  id: number;
  name: string;
  is_multi_select: boolean;
  max_selections: number | null;
  default_option_ids: number[] | null;
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
  max_selections?: number | null;
  default_option_ids?: number[];
  display_order?: number;
  show_on_receipt?: boolean;
  receipt_name?: string;
  show_on_kitchen_print?: boolean;
  kitchen_print_name?: string;
  options?: AttributeOptionInput[];
}

export interface AttributeUpdate {
  name?: string;
  is_multi_select?: boolean;
  max_selections?: number | null;
  default_option_ids?: number[] | null;
  display_order?: number;
  is_active?: boolean;
  show_on_receipt?: boolean;
  receipt_name?: string;
  show_on_kitchen_print?: boolean;
  kitchen_print_name?: string;
  options?: AttributeOptionInput[];
}

export interface AttributeBinding {
  id: number;
  /** Owner ID (product or category) */
  owner_id: number;
  /** Attribute ID */
  attribute_id: number;
  is_required: boolean;
  display_order: number;
  default_option_ids?: number[];
}


// ============ Printer ============

/** Physical connection method */
export type PrinterConnection = 'network' | 'driver';
/** Communication protocol */
export type PrinterProtocol = 'escpos' | 'tspl';
/** Print destination purpose */
export type PrintPurpose = 'kitchen' | 'label';

export interface Printer {
  id?: number;
  print_destination_id?: number;
  /** Physical connection method */
  connection: PrinterConnection;
  /** Communication protocol */
  protocol: PrinterProtocol;
  ip?: string;
  port?: number;
  driver_name?: string;
  priority: number;
  is_active: boolean;
}

// ============ Print Destination ============

export interface PrintDestination {
  id: number;
  name: string;
  description?: string;
  /** Purpose: kitchen or label */
  purpose: PrintPurpose;
  printers: Printer[];
  is_active: boolean;
}

export interface PrintDestinationCreate {
  name: string;
  description?: string;
  purpose?: PrintPurpose;
  printers?: Printer[];
  is_active?: boolean;
}

export interface PrintDestinationUpdate {
  name?: string;
  description?: string;
  purpose?: PrintPurpose;
  printers?: Printer[];
  is_active?: boolean;
}

// ============ Zone ============

export interface Zone {
  id: number;
  name: string;
  description: string | null;
  is_active: boolean;
}

interface ZoneCreate {
  name: string;
  description?: string;
}

interface ZoneUpdate {
  name?: string;
  description?: string;
  is_active?: boolean;
}

// ============ Dining Table ============

export interface DiningTable {
  id: number;
  name: string;
  zone_id: number;
  capacity: number;
  is_active: boolean;
}

interface DiningTableCreate {
  name: string;
  zone_id: number;
  capacity?: number;
}

interface DiningTableUpdate {
  name?: string;
  zone_id?: number;
  capacity?: number;
  is_active?: boolean;
}

// ============ Price Rule ============

export type RuleType = 'DISCOUNT' | 'SURCHARGE';
export type ProductScope = 'GLOBAL' | 'CATEGORY' | 'TAG' | 'PRODUCT';
export type AdjustmentType = 'PERCENTAGE' | 'FIXED_AMOUNT';

export interface PriceRule {
  id: number;
  name: string;
  display_name: string;
  receipt_name: string;
  description: string | null;
  rule_type: RuleType;
  product_scope: ProductScope;
  target_id: number | null;
  /** Zone scope: "all", "retail", or specific zone ID */
  zone_scope: string;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  is_stackable: boolean;
  is_exclusive: boolean;
  // Time fields
  valid_from: number | null;        // Unix millis (i64)
  valid_until: number | null;       // Unix millis (i64)
  active_days: number[] | null;     // [0=Sunday, 1=Monday, ...]
  active_start_time: string | null; // HH:MM format
  active_end_time: string | null;   // HH:MM format
  is_active: boolean;
  created_by: number | null;
  created_at: number;               // Unix millis (i64)
}

export interface PriceRuleCreate {
  name: string;
  display_name: string;
  receipt_name: string;
  description?: string;
  rule_type: RuleType;
  product_scope: ProductScope;
  target_id?: number;
  /** Zone scope: "all", "retail", or specific zone ID */
  zone_scope?: string;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  is_stackable?: boolean;
  is_exclusive?: boolean;
  // Time fields
  valid_from?: number;        // Unix millis (i64)
  valid_until?: number;       // Unix millis (i64)
  active_days?: number[];     // [0=Sunday, 1=Monday, ...]
  active_start_time?: string; // HH:MM format
  active_end_time?: string;   // HH:MM format
  created_by?: number;
}

export interface PriceRuleUpdate {
  name?: string;
  display_name?: string;
  receipt_name?: string;
  description?: string;
  rule_type?: RuleType;
  product_scope?: ProductScope;
  target_id?: number;
  /** Zone scope: "all", "retail", or specific zone ID */
  zone_scope?: string;
  adjustment_type?: AdjustmentType;
  adjustment_value?: number;
  is_stackable?: boolean;
  is_exclusive?: boolean;
  // Time fields
  valid_from?: number;        // Unix millis (i64)
  valid_until?: number;       // Unix millis (i64)
  active_days?: number[];     // [0=Sunday, 1=Monday, ...]
  active_start_time?: string; // HH:MM format
  active_end_time?: string;   // HH:MM format
  is_active?: boolean;
}

// ============ Marketing Group ============

export interface MarketingGroup {
  id: number;
  name: string;
  display_name: string;
  description: string | null;
  sort_order: number;
  points_earn_rate: number | null;
  created_at: number;
  updated_at: number;
}

export interface MarketingGroupCreate {
  name: string;
  display_name: string;
  description?: string | null;
  sort_order?: number;
  points_earn_rate?: number | null;
}

export interface MarketingGroupUpdate {
  name?: string;
  display_name?: string;
  description?: string | null;
  sort_order?: number;
  points_earn_rate?: number | null;
}

export interface MgDiscountRule {
  id: number;
  marketing_group_id: number;
  name: string;
  display_name: string;
  receipt_name: string;
  product_scope: ProductScope;
  target_id: number | null;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface MgDiscountRuleCreate {
  name: string;
  display_name: string;
  receipt_name: string;
  product_scope: ProductScope;
  target_id?: number | null;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
}

export interface MgDiscountRuleUpdate {
  name?: string;
  display_name?: string;
  receipt_name?: string;
  product_scope?: ProductScope;
  target_id?: number | null;
  adjustment_type?: AdjustmentType;
  adjustment_value?: number;
  is_active?: boolean;
}

export interface MarketingGroupDetail extends MarketingGroup {
  discount_rules: MgDiscountRule[];
  stamp_activities: StampActivityDetail[];
}

// ============ Member ============

interface Member {
  id: number;
  name: string;
  phone: string | null;
  card_number: string | null;
  marketing_group_id: number;
  birthday: string | null;
  email: string | null;
  points_balance: number;
  total_spent: number;
  notes: string | null;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface MemberCreate {
  name: string;
  phone?: string | null;
  card_number?: string | null;
  marketing_group_id: number;
  birthday?: string | null;
  email?: string | null;
  notes?: string | null;
}

export interface MemberUpdate {
  name?: string;
  phone?: string | null;
  card_number?: string | null;
  marketing_group_id?: number;
  birthday?: string | null;
  email?: string | null;
  notes?: string | null;
  is_active?: boolean;
}

export interface MemberWithGroup extends Member {
  marketing_group_name: string;
}

// ============ Stamp ============

export type RewardStrategy = 'ECONOMIZADOR' | 'GENEROSO' | 'DESIGNATED';
export type StampTargetType = 'CATEGORY' | 'PRODUCT';

interface StampActivity {
  id: number;
  marketing_group_id: number;
  name: string;
  display_name: string;
  stamps_required: number;
  reward_quantity: number;
  reward_strategy: RewardStrategy;
  designated_product_id: number | null;
  is_cyclic: boolean;
  is_active: boolean;
  created_at: number;
  updated_at: number;
}

export interface StampActivityCreate {
  name: string;
  display_name: string;
  stamps_required: number;
  reward_quantity?: number;
  reward_strategy?: RewardStrategy;
  designated_product_id?: number | null;
  is_cyclic?: boolean;
  stamp_targets: StampTargetInput[];
  reward_targets: StampTargetInput[];
}

export interface StampActivityUpdate {
  name?: string;
  display_name?: string;
  stamps_required?: number;
  reward_quantity?: number;
  reward_strategy?: RewardStrategy;
  designated_product_id?: number | null;
  is_cyclic?: boolean;
  is_active?: boolean;
  stamp_targets?: StampTargetInput[];
  reward_targets?: StampTargetInput[];
}

export interface StampTargetInput {
  target_type: StampTargetType;
  target_id: number;
}

export interface StampTarget {
  id: number;
  stamp_activity_id: number;
  target_type: StampTargetType;
  target_id: number;
}

export interface StampRewardTarget {
  id: number;
  stamp_activity_id: number;
  target_type: StampTargetType;
  target_id: number;
}

interface MemberStampProgress {
  id: number;
  member_id: number;
  stamp_activity_id: number;
  current_stamps: number;
  completed_cycles: number;
  last_stamp_at: number | null;
  updated_at: number;
}

export interface StampActivityDetail extends StampActivity {
  stamp_targets: StampTarget[];
  reward_targets: StampRewardTarget[];
}

export interface MemberStampProgressDetail {
  stamp_activity_id: number;
  stamp_activity_name: string;
  stamps_required: number;
  current_stamps: number;
  completed_cycles: number;
  is_redeemable: boolean;
  is_cyclic: boolean;
  reward_strategy: RewardStrategy;
  reward_quantity: number;
  designated_product_id: number | null;
  /** Stamp targets for dynamic order progress calculation */
  stamp_targets: StampTarget[];
  /** Reward targets for redemption matching */
  reward_targets: StampRewardTarget[];
}

// ============ Applied MG Rule ============

export interface AppliedMgRule {
  rule_id: number;
  name: string;
  display_name: string;
  receipt_name: string;
  product_scope: ProductScope;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  calculated_amount: number;
  skipped: boolean;
}

// ============ Employee ============

export interface Employee {
  id: number;
  username: string;
  display_name: string;
  role_id: number;
  is_system: boolean;
  is_active: boolean;
  created_at: number;
}


interface EmployeeCreate {
  username: string;
  password: string;
  display_name?: string;
  role_id: number;
}

interface EmployeeUpdate {
  username?: string;
  password?: string;
  display_name?: string;
  role_id?: number;
  is_active?: boolean;
}

// ============ Table Short Names ============

export type Table = DiningTable;
type TableCreate = DiningTableCreate;
type TableUpdate = DiningTableUpdate;


// ============ Role ============

export interface Role {
  id: number;
  name: string;
  display_name: string;
  description: string | null;
  permissions: string[];
  is_system: boolean;
  is_active: boolean;
}

interface RoleCreate {
  name: string;
  display_name?: string;
  description?: string;
  permissions?: string[];
}

interface RoleUpdate {
  name?: string;
  display_name?: string;
  description?: string;
  permissions?: string[];
  is_active?: boolean;
}

export interface RolePermission {
  role_id: number;
  permission: string;
}

// ============ User (Frontend representation) ============

/**
 * User for auth store - aligned with shared::client::UserInfo
 */
export interface User {
  id: number;
  username: string;
  display_name: string;
  role_id: number;
  role_name: string;
  permissions: string[];
  is_system: boolean;
  is_active: boolean;
  created_at: number;
}

// ============ Product/Category Attribute Bindings ============

/**
 * ProductAttribute represents an AttributeBinding where owner is a Product
 */
export interface ProductAttribute extends AttributeBinding {
  /** The attribute details when fetched with relations */
  attribute?: Attribute;
}

/**
 * CategoryAttribute represents an AttributeBinding where owner is a Category
 */
interface CategoryAttribute extends AttributeBinding {
  /** The attribute details when fetched with relations */
  attribute?: Attribute;
}

// ============ Kitchen Printing ============

/**
 * 打印上下文 (完整 JSON，模板自取所需字段)
 * Aligned with edge-server printing types
 */
interface PrintItemContext {
  // 分类
  category_id: number;
  category_name: string;

  // 商品
  product_id: number;
  external_id: number | null; // 菜品编号
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
  kitchen_destinations: number[];
  label_destinations: number[];
}

/** 厨房订单菜品 */
interface KitchenOrderItem {
  context: PrintItemContext;
}

/**
 * 一次点单的厨房记录（对应一个 ItemsAdded 事件）
 * Used for kitchen order display and reprint
 */
interface KitchenOrder {
  /** Kitchen order ID (= event_id, UUID) */
  id: string;
  /** Parent order ID (UUID) */
  order_id: string;
  /** Table name (if applicable) */
  table_name: string | null;
  /** Unix millis */
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
interface LabelPrintRecord {
  /** Label record ID (UUID) */
  id: string;
  /** Parent order ID (UUID) */
  order_id: string;
  /** Related kitchen order ID (UUID) */
  kitchen_order_id: string;
  /** Table name (if applicable) */
  table_name: string | null;
  /** Unix millis */
  created_at: number;
  /** Print context for this label */
  context: PrintItemContext;
  /** Number of times this label has been printed */
  print_count: number;
}

/** Response for kitchen order list */
interface KitchenOrderListResponse {
  items: KitchenOrder[];
  total: number | null;
}

// ============ Store Info ============

/**
 * Store information (singleton per tenant)
 * Used for receipts, labels, and business info display
 */
export interface StoreInfo {
  id: number;
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
   * Default "02:00", bars/nightclubs can set to "06:00"
   */
  business_day_cutoff: string;
  created_at: number | null;
  updated_at: number | null;
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
export type { LabelTemplate } from '../print/labelTemplate';

export interface LabelTemplateCreate {
  name: string;
  description?: string;
  width: number;
  height: number;
  fields?: import('../print/labelTemplate').LabelField[];
  is_default?: boolean;
  is_active?: boolean;
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
  fields?: import('../print/labelTemplate').LabelField[];
  is_default?: boolean;
  is_active?: boolean;
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
  id: number;
  /** Operator employee ID */
  operator_id: number;
  /** Operator display name */
  operator_name: string;
  /** Shift status */
  status: ShiftStatus;
  /** Shift start time (Unix millis) */
  start_time: number;
  /** Shift end time (Unix millis), null if still open */
  end_time: number | null;
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
  /** Last heartbeat timestamp (Unix millis) */
  last_active_at: number | null;
  /** Notes */
  note: string | null;
  created_at: number | null;
  updated_at: number | null;
}

export interface ShiftCreate {
  /** Operator employee ID */
  operator_id: number;
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
  id: number;
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
  /** When the report was generated (Unix millis) */
  generated_at: number | null;
  /** Who generated the report (employee ID) */
  generated_by_id: number | null;
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

// ============ Audit Log (审计日志) ============

/** 审计操作类型 — 与 Rust AuditAction 枚举对齐 (snake_case) */
export type AuditAction =
  // 系统生命周期
  | 'system_startup'
  | 'system_shutdown'
  | 'system_abnormal_shutdown'
  | 'system_long_downtime'
  | 'resolve_system_issue'
  // 认证
  | 'login_success'
  | 'login_failed'
  | 'logout'
  | 'escalation_success'
  // 订单（财务关键 — 仅终结状态，中间操作由 OrderEvents 覆盖）
  | 'order_completed'
  | 'order_voided'
  | 'order_merged'
  // 管理操作
  | 'employee_created'
  | 'employee_updated'
  | 'employee_deleted'
  | 'role_created'
  | 'role_updated'
  | 'role_deleted'
  // 商品目录
  | 'product_created'
  | 'product_updated'
  | 'product_deleted'
  | 'category_created'
  | 'category_updated'
  | 'category_deleted'
  | 'tag_created'
  | 'tag_updated'
  | 'tag_deleted'
  | 'attribute_created'
  | 'attribute_updated'
  | 'attribute_deleted'
  // 价格规则
  | 'price_rule_created'
  | 'price_rule_updated'
  | 'price_rule_deleted'
  // 区域与桌台
  | 'zone_created'
  | 'zone_updated'
  | 'zone_deleted'
  | 'table_created'
  | 'table_updated'
  | 'table_deleted'
  // 打印
  | 'label_template_created'
  | 'label_template_updated'
  | 'label_template_deleted'
  | 'print_destination_created'
  | 'print_destination_updated'
  | 'print_destination_deleted'
  // 会员
  | 'member_created'
  | 'member_updated'
  | 'member_deleted'
  // 营销组
  | 'marketing_group_created'
  | 'marketing_group_updated'
  | 'marketing_group_deleted'
  // 班次
  | 'shift_opened'
  | 'shift_updated'
  | 'shift_closed'
  // 日结报告
  | 'daily_report_generated'
  // 系统配置
  | 'print_config_changed'
  | 'store_info_changed';

/** 审计日志条目 — 与 Rust AuditEntry 对齐 */
export interface AuditEntry {
  /** 全局递增序列号 */
  id: number;
  /** 时间戳 (Unix 毫秒) */
  timestamp: number;
  /** 操作类型 */
  action: AuditAction;
  /** 资源类型 (如 "system", "order", "employee") */
  resource_type: string;
  /** 资源 ID */
  resource_id: string;
  /** 操作人 ID (系统事件为 null) */
  operator_id: number | null;
  /** 操作人名称 */
  operator_name: string | null;
  /** 结构化详情 */
  details: Record<string, unknown>;
  /** 关联目标（可选，指向相关审计条目或资源） */
  target?: string | null;
  /** 前一条审计日志哈希 (SHA256) */
  prev_hash: string;
  /** 当前记录哈希 (SHA256) */
  curr_hash: string;
}

/** 审计日志查询响应 */
export interface AuditListResponse {
  items: AuditEntry[];
  total: number;
}

// ============ System Issues (系统问题) ============

/** 系统问题 — 与 Rust SystemIssueRow 对齐 */
export interface SystemIssue {
  id: number;
  source: string;
  kind: string;
  blocking: boolean;
  target?: string;
  params: Record<string, string>;
  title?: string;
  description?: string;
  options: string[];
  status: string;
  response?: string;
  resolved_by?: string;
  resolved_at?: number;
  created_at: number;
}

/** 解决系统问题请求 */
export interface ResolveSystemIssueRequest {
  id: number;
  response: string;
}
