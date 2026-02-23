// ── Product ──

export interface CatalogSpec {
  source_id: number;
  name: string;
  price: number;
  display_order: number;
  is_default: boolean;
  is_active: boolean;
  receipt_name: string | null;
  is_root: boolean;
}

export interface CatalogProduct {
  source_id: number;
  name: string;
  image: string;
  category_source_id: number;
  category_name: string | null;
  sort_order: number;
  tax_rate: number;
  receipt_name: string | null;
  kitchen_print_name: string | null;
  is_kitchen_print_enabled: number;
  is_label_print_enabled: number;
  is_active: boolean;
  external_id: number | null;
  specs: CatalogSpec[];
  tag_ids: number[];
}

export interface ProductSpecInput {
  name: string;
  price: number;
  display_order: number;
  is_default: boolean;
  is_active: boolean;
  receipt_name?: string;
  is_root: boolean;
}

export interface ProductCreate {
  name: string;
  image?: string;
  category_id: number;
  sort_order?: number;
  tax_rate?: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  is_kitchen_print_enabled?: number;
  is_label_print_enabled?: number;
  external_id?: number;
  tags?: number[];
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
  is_kitchen_print_enabled?: number;
  is_label_print_enabled?: number;
  is_active?: boolean;
  external_id?: number;
  tags?: number[];
  specs?: ProductSpecInput[];
}

// ── Category ──

export interface CatalogCategory {
  source_id: number;
  name: string;
  sort_order: number;
  is_kitchen_print_enabled: boolean;
  is_label_print_enabled: boolean;
  is_active: boolean;
  is_virtual: boolean;
  match_mode: string;
  is_display: boolean;
  kitchen_print_destinations: number[];
  label_print_destinations: number[];
  tag_ids: number[];
}

export interface CategoryCreate {
  name: string;
  sort_order?: number;
  is_virtual?: boolean;
  is_display?: boolean;
}

export interface CategoryUpdate {
  name?: string;
  sort_order?: number;
  is_virtual?: boolean;
  is_active?: boolean;
  is_display?: boolean;
}

// ── Tag ──

export interface CatalogTag {
  source_id: number;
  name: string;
  color: string;
  display_order: number;
  is_active: boolean;
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

// ── Attribute ──

export interface CatalogAttributeOption {
  source_id: number;
  name: string;
  price_modifier: number;
  display_order: number;
  is_active: boolean;
  receipt_name: string | null;
  kitchen_print_name: string | null;
  enable_quantity: boolean;
  max_quantity: number | null;
}

export interface CatalogAttribute {
  source_id: number;
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
  options: CatalogAttributeOption[];
}

export interface AttributeOptionInput {
  name: string;
  price_modifier: number;
  display_order: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  enable_quantity: boolean;
  max_quantity?: number;
}

export interface AttributeCreate {
  name: string;
  is_multi_select?: boolean;
  max_selections?: number;
  display_order?: number;
  options?: AttributeOptionInput[];
}

export interface AttributeUpdate {
  name?: string;
  is_multi_select?: boolean;
  max_selections?: number;
  display_order?: number;
  options?: AttributeOptionInput[];
  is_active?: boolean;
}

// ── Price Rule ──

export type RuleType = 'DISCOUNT' | 'SURCHARGE';
export type ProductScope = 'GLOBAL' | 'CATEGORY' | 'TAG' | 'PRODUCT';
export type AdjustmentType = 'PERCENTAGE' | 'FIXED_AMOUNT';

export interface PriceRule {
  source_id: number;
  name: string;
  display_name: string;
  receipt_name: string;
  description: string | null;
  rule_type: RuleType;
  product_scope: ProductScope;
  target_id: number | null;
  zone_scope: string;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  is_stackable: boolean;
  is_exclusive: boolean;
  valid_from: number | null;
  valid_until: number | null;
  active_days: number[] | null;
  active_start_time: string | null;
  active_end_time: string | null;
  is_active: boolean;
  created_by: number | null;
  created_at: number;
}

export interface PriceRuleCreate {
  name: string;
  display_name: string;
  receipt_name: string;
  description?: string;
  rule_type: RuleType;
  product_scope: ProductScope;
  target_id?: number;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  is_stackable?: boolean;
  is_exclusive?: boolean;
}

export interface PriceRuleUpdate {
  name?: string;
  display_name?: string;
  receipt_name?: string;
  description?: string;
  rule_type?: RuleType;
  product_scope?: ProductScope;
  target_id?: number;
  adjustment_type?: AdjustmentType;
  adjustment_value?: number;
  is_stackable?: boolean;
  is_exclusive?: boolean;
  is_active?: boolean;
}

// ── Employee ──

export interface Employee {
  id: number;
  username: string;
  display_name: string;
  role_id: number;
  is_active: boolean;
  is_system: boolean;
}

export interface EmployeeCreate {
  username: string;
  password: string;
  display_name?: string;
  role_id: number;
}

export interface EmployeeUpdate {
  username?: string;
  password?: string;
  display_name?: string;
  role_id?: number;
  is_active?: boolean;
}

// ── Zone ──

export interface Zone {
  id: number;
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

// ── Dining Table ──

export interface DiningTable {
  id: number;
  name: string;
  zone_id: number;
  capacity: number;
  is_active: boolean;
}

export interface DiningTableCreate {
  name: string;
  zone_id: number;
  capacity?: number;
}

export interface DiningTableUpdate {
  name?: string;
  zone_id?: number;
  capacity?: number;
  is_active?: boolean;
}

// ── CatalogOpResult ──

export interface CatalogOpResult {
  success: boolean;
  created_id?: number;
  data?: unknown;
  error?: string;
}
