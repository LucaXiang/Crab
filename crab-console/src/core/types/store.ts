export interface StoreDetail {
  id: number;
  entity_id: string;
  name?: string;
  address?: string;
  phone?: string;
  nif?: string;
  email?: string;
  website?: string;
  business_day_cutoff?: string;
  device_id: string;
  is_online: boolean;
  last_sync_at: number | null;
  registered_at: number;
  store_info: Record<string, unknown> | null;
}

// ── Product ──

export interface StoreSpec {
  source_id: number;
  name: string;
  price: number;
  display_order: number;
  is_default: boolean;
  is_active: boolean;
  receipt_name: string | null;
  is_root: boolean;
}

export interface StoreProduct {
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
  specs: StoreSpec[];
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

export interface StoreCategory {
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
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
  kitchen_print_destinations?: number[];
  label_print_destinations?: number[];
  is_virtual?: boolean;
  tag_ids?: number[];
  match_mode?: string;
  is_display?: boolean;
}

export interface CategoryUpdate {
  name?: string;
  sort_order?: number;
  is_kitchen_print_enabled?: boolean;
  is_label_print_enabled?: boolean;
  kitchen_print_destinations?: number[];
  label_print_destinations?: number[];
  is_virtual?: boolean;
  tag_ids?: number[];
  match_mode?: string;
  is_active?: boolean;
  is_display?: boolean;
}

// ── Tag ──

export interface StoreTag {
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

export interface StoreAttributeOption {
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

export interface StoreAttribute {
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
  options: StoreAttributeOption[];
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
  default_option_ids?: number[];
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
  zone_scope?: string;
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  is_stackable?: boolean;
  is_exclusive?: boolean;
  valid_from?: number;
  valid_until?: number;
  active_days?: number[];
  active_start_time?: string;
  active_end_time?: string;
}

export interface PriceRuleUpdate {
  name?: string;
  display_name?: string;
  receipt_name?: string;
  description?: string;
  rule_type?: RuleType;
  product_scope?: ProductScope;
  target_id?: number;
  zone_scope?: string;
  adjustment_type?: AdjustmentType;
  adjustment_value?: number;
  is_stackable?: boolean;
  is_exclusive?: boolean;
  valid_from?: number;
  valid_until?: number;
  active_days?: number[];
  active_start_time?: string;
  active_end_time?: string;
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
  created_at: number;
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

// ── Label Template ──

export enum LabelFieldType {
  Text = 'text',
  Barcode = 'barcode',
  Qrcode = 'qrcode',
  Image = 'image',
  Separator = 'separator',
  Datetime = 'datetime',
  Price = 'price',
  Counter = 'counter',
}

export enum LabelFieldAlignment {
  Left = 'left',
  Center = 'center',
  Right = 'right',
}

export enum LabelVerticalAlign {
  Top = 'top',
  Middle = 'middle',
  Bottom = 'bottom',
}

export type LabelImageSourceType = 'productImage' | 'qrCode' | 'barcode' | 'image';
export type LabelLineStyle = 'solid' | 'dashed' | 'dotted';

export interface LabelField {
  field_id: string;
  name: string;
  field_type: LabelFieldType;
  x: number;
  y: number;
  width: number;
  height: number;
  font_size: number;
  font_weight?: string;
  font_family?: string;
  color?: string;
  rotate?: number;
  alignment?: LabelFieldAlignment;
  data_source: string;
  format?: string;
  visible: boolean;
  label?: string;
  template?: string;
  data_key?: string;
  source_type?: LabelImageSourceType;
  maintain_aspect_ratio?: boolean;
  /** Temporary blob URL for pending image upload (editor only, not persisted) */
  _pending_blob_url?: string;
  style?: string;
  align?: LabelFieldAlignment;
  vertical_align?: LabelVerticalAlign;
  line_style?: LabelLineStyle;
}

export interface LabelTemplate {
  id: number;
  name: string;
  description?: string;
  width: number;
  height: number;
  padding: number;
  fields: LabelField[];
  is_default: boolean;
  is_active: boolean;
  created_at: number;
  updated_at: number;
  width_mm?: number;
  height_mm?: number;
  padding_mm_x?: number;
  padding_mm_y?: number;
  render_dpi?: number;
  test_data?: string;
}

export interface LabelFieldInput {
  field_id: string;
  name: string;
  field_type: LabelFieldType;
  x: number;
  y: number;
  width: number;
  height: number;
  font_size?: number;
  font_weight?: string;
  font_family?: string;
  color?: string;
  rotate?: number;
  alignment?: LabelFieldAlignment;
  data_source?: string;
  format?: string;
  visible?: boolean;
  template?: string;
  data_key?: string;
  source_type?: LabelImageSourceType;
  maintain_aspect_ratio?: boolean;
  style?: string;
  align?: LabelFieldAlignment;
  vertical_align?: LabelVerticalAlign;
  line_style?: LabelLineStyle;
}

export interface LabelTemplateCreate {
  name: string;
  description?: string;
  width: number;
  height: number;
  padding?: number;
  fields?: LabelFieldInput[];
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
  fields?: LabelFieldInput[];
  is_default?: boolean;
  is_active?: boolean;
  width_mm?: number;
  height_mm?: number;
  padding_mm_x?: number;
  padding_mm_y?: number;
  render_dpi?: number;
  test_data?: string;
}

// ── Attribute Option Independent CRUD ──

export interface AttributeOptionCreate {
  name: string;
  price_modifier?: number;
  display_order?: number;
  receipt_name?: string;
  kitchen_print_name?: string;
  enable_quantity?: boolean;
  max_quantity?: number;
}

export interface AttributeOptionUpdate {
  name?: string;
  price_modifier?: number;
  display_order?: number;
  is_active?: boolean;
  receipt_name?: string;
  kitchen_print_name?: string;
  enable_quantity?: boolean;
  max_quantity?: number;
}

// ── Attribute Binding ──

export interface BindAttributeRequest {
  owner_type: 'product' | 'category';
  owner_id: number;
  attribute_id: number;
  is_required?: boolean;
  display_order?: number;
  default_option_ids?: number[];
}

export interface UnbindAttributeRequest {
  binding_id: number;
}

// ── Sort Order ──

export interface SortOrderItem {
  id: number;
  sort_order: number;
}

// ── Bulk Delete ──

export interface BulkDeleteRequest {
  ids: number[];
}

// ── Store Info ──

export interface StoreInfo {
  name: string;
  address: string | null;
  phone: string | null;
  nif: string | null;
  email: string | null;
  website: string | null;
  business_day_cutoff: string | null;
  currency_code: string | null;
  currency_symbol: string | null;
  currency_decimal_places: number | null;
  logo: string | null;
  timezone: string | null;
  receipt_header: string | null;
  receipt_footer: string | null;
}

export interface StoreInfoUpdate {
  name?: string;
  address?: string;
  phone?: string;
  nif?: string;
  email?: string;
  website?: string;
  business_day_cutoff?: string;
  currency_code?: string;
  currency_symbol?: string;
  currency_decimal_places?: number;
  logo?: string;
  timezone?: string;
  receipt_header?: string;
  receipt_footer?: string;
}

// ── StoreOpResult ──

export interface StoreOpResult {
  success: boolean;
  created_id?: number;
  data?: unknown;
  error?: string;
}
