/**
 * Label Template Types
 * Defines types for label template and label printing functionality
 */

/**
 * Label template field type
 */
export type LabelFieldType =
  | 'text'
  | 'barcode'
  | 'qrcode'
  | 'image'
  | 'separator'
  | 'datetime'
  | 'price'
  | 'counter';

/**
 * Label template field
 */
export interface LabelField {
  field_id: string;
  name: string;
  field_type: LabelFieldType;
  x: number;  // X position in pixels (base 203 DPI ≈ 8 px/mm)
  y: number;  // Y position in pixels
  width: number;
  height: number;
  font_size: number;
  font_weight?: string;
  font_family?: string;
  color?: string;
  rotate?: number;
  alignment?: 'left' | 'center' | 'right';
  /** Canonical flat key matching build_label_data() JSON keys (e.g. "product_name") */
  data_source: string;
  format?: string;  // Format pattern (e.g., for date/time)
  visible: boolean;
  label?: string;
  /** Custom format template for text (e.g. "€{price}"), or image hash for image fields */
  template?: string;
  source_type?: 'productImage' | 'qrCode' | 'barcode' | 'image';
  maintain_aspect_ratio?: boolean;
  /** Temporary local file path for pending image upload (editor only, not persisted) */
  _pending_image_path?: string;
  style?: string;
  align?: 'left' | 'center' | 'right';
  vertical_align?: 'top' | 'middle' | 'bottom';
  line_style?: 'solid' | 'dashed' | 'dotted';
}

/**
 * Label template
 */
export interface LabelTemplate {
  id: number;
  name: string;
  description?: string;
  width: number;  // Label width in mm
  height: number;  // Label height in mm
  padding: number;
  fields: LabelField[];
  is_default: boolean;
  is_active: boolean;
  created_at?: number;
  updated_at?: number;
  // UI-specific properties
  width_mm?: number;
  height_mm?: number;
  padding_mm_x?: number;
  padding_mm_y?: number;
  render_dpi?: number;
  test_data?: string;
}

/**
 * Create label template params
 */
export interface CreateLabelTemplateParams {
  name: string;
  description?: string;
  width: number;
  height: number;
  padding?: number;
  fields?: LabelField[];
  is_default?: boolean;
  is_active?: boolean;
  width_mm?: number;
  height_mm?: number;
  padding_mm_x?: number;
  padding_mm_y?: number;
  render_dpi?: number;
  test_data?: string;
}

/**
 * Update label template params
 */
export interface UpdateLabelTemplateParams extends Partial<CreateLabelTemplateParams> {
  id: number;
}

/**
 * Label print job
 */
export interface LabelPrintJob {
  id: number;
  template_id: number;
  data: Record<string, unknown>;
  quantity: number;
  status: 'pending' | 'printing' | 'completed' | 'failed';
  printer_id?: number;
  created_at: number;
  printed_at?: number;
  error?: string;
}

/**
 * Predefined label templates
 */
export const DEFAULT_LABEL_TEMPLATES: LabelTemplate[] = [
  {
    id: -1,
    name: '小标签 (30x20)',
    description: '适用于小规格标签',
    width: 30,
    height: 20,
    width_mm: 30,
    height_mm: 20,
    padding: 1,
    is_default: true,
    is_active: true,
    created_at: Date.now(),
    updated_at: Date.now(),
    // Field coordinates are in pixels (base 203 DPI ≈ 8 px/mm)
    // 30mm × 8 = 240px wide, 20mm × 8 = 160px tall
    fields: [
      {
        field_id: 'name',
        name: '商品名称',
        field_type: 'text',
        x: 8,
        y: 16,
        width: 224,
        height: 32,
        font_size: 10,
        font_weight: 'bold',
        data_source: 'product_name',
        visible: true,
      },
      {
        field_id: 'spec',
        name: '规格',
        field_type: 'text',
        x: 8,
        y: 56,
        width: 224,
        height: 24,
        font_size: 8,
        data_source: 'spec_name',
        visible: true,
      },
      {
        field_id: 'barcode',
        name: '条码',
        field_type: 'barcode',
        x: 8,
        y: 88,
        width: 224,
        height: 40,
        font_size: 8,
        data_source: 'external_id',
        format: 'CODE128',
        visible: true,
      },
      {
        field_id: 'datetime',
        name: '打印时间',
        field_type: 'datetime',
        x: 8,
        y: 136,
        width: 224,
        height: 20,
        font_size: 7,
        data_source: 'time',
        alignment: 'center',
        visible: true,
      },
    ],
  },
  {
    id: -2,
    name: '标准标签 (40x30)',
    description: '适用于标准规格标签',
    width: 40,
    height: 30,
    width_mm: 40,
    height_mm: 30,
    padding: 2,
    is_default: false,
    is_active: true,
    created_at: Date.now(),
    updated_at: Date.now(),
    // 40mm × 8 = 320px wide, 30mm × 8 = 240px tall
    fields: [
      {
        field_id: 'name',
        name: '商品名称',
        field_type: 'text',
        x: 16,
        y: 12,
        width: 288,
        height: 44,
        font_size: 14,
        font_weight: 'bold',
        data_source: 'product_name',
        visible: true,
      },
      {
        field_id: 'spec',
        name: '规格',
        field_type: 'text',
        x: 16,
        y: 64,
        width: 288,
        height: 28,
        font_size: 10,
        data_source: 'spec_name',
        visible: true,
      },
      {
        field_id: 'barcode',
        name: '条码',
        field_type: 'barcode',
        x: 16,
        y: 100,
        width: 288,
        height: 80,
        font_size: 10,
        data_source: 'external_id',
        format: 'CODE128',
        visible: true,
      },
      {
        field_id: 'datetime',
        name: '打印时间',
        field_type: 'datetime',
        x: 16,
        y: 188,
        width: 288,
        height: 24,
        font_size: 8,
        data_source: 'time',
        alignment: 'center',
        visible: true,
      },
    ],
  },
  {
    id: -3,
    name: '厨打标签 (50x40)',
    description: '适用于厨房打印',
    width: 50,
    height: 40,
    width_mm: 50,
    height_mm: 40,
    padding: 3,
    is_default: false,
    is_active: true,
    created_at: Date.now(),
    updated_at: Date.now(),
    // 50mm × 8 = 400px wide, 40mm × 8 = 320px tall
    fields: [
      {
        field_id: 'orderNum',
        name: '订单号',
        field_type: 'text',
        x: 24,
        y: 24,
        width: 352,
        height: 64,
        font_size: 24,
        font_weight: 'bold',
        data_source: 'queue_number',
        alignment: 'center',
        visible: true,
      },
      {
        field_id: 'table',
        name: '桌号',
        field_type: 'text',
        x: 24,
        y: 108,
        width: 176,
        height: 48,
        font_size: 18,
        font_weight: 'bold',
        data_source: 'table_name',
        visible: true,
      },
      {
        field_id: 'quantity',
        name: '数量',
        field_type: 'text',
        x: 224,
        y: 108,
        width: 152,
        height: 48,
        font_size: 18,
        font_weight: 'bold',
        data_source: 'quantity',
        alignment: 'right',
        visible: true,
      },
      {
        field_id: 'itemName',
        name: '菜品名称',
        field_type: 'text',
        x: 24,
        y: 176,
        width: 352,
        height: 80,
        font_size: 16,
        font_weight: 'bold',
        data_source: 'kitchen_name',
        alignment: 'center',
        visible: true,
      },
      {
        field_id: 'options',
        name: '加料',
        field_type: 'text',
        x: 24,
        y: 268,
        width: 352,
        height: 32,
        font_size: 10,
        color: '#666666',
        data_source: 'options',
        alignment: 'center',
        visible: true,
      },
    ],
  },
];

/**
 * Label field helper functions
 */
export const LabelFieldHelpers = {
  /**
   * Create a new label field
   */
  createField(overrides: Partial<LabelField> = {}): LabelField {
    return {
      field_id: crypto.randomUUID(),
      name: '',
      field_type: 'text',
      x: 0,
      y: 0,
      width: 20,
      height: 10,
      font_size: 10,
      font_weight: 'normal',
      font_family: 'Arial',
      color: '#000000',
      rotate: 0,
      alignment: 'left',
      data_source: '',
      format: '',
      visible: true,
      ...overrides,
    };
  },

  /**
   * Get default fields for a label size
   */
  getDefaultFields(width: number, height: number): LabelField[] {
    if (width <= 30) {
      return [
        this.createField({
          name: '商品名称',
          field_type: 'text',
          x: 1,
          y: 2,
          width: 28,
          height: 6,
          font_size: 10,
          font_weight: 'bold',
          data_source: 'product_name',
        }),
        this.createField({
          name: '规格',
          field_type: 'text',
          x: 1,
          y: 10,
          width: 28,
          height: 5,
          font_size: 9,
          data_source: 'spec_name',
        }),
      ];
    }
    return [
      this.createField({
        name: '商品名称',
        field_type: 'text',
        x: 2,
        y: 2,
        width: width - 4,
        height: 8,
        font_size: 14,
        font_weight: 'bold',
        data_source: 'product_name',
        alignment: 'center',
      }),
      this.createField({
        name: '规格',
        field_type: 'text',
        x: 2,
        y: 14,
        width: width - 4,
        height: 6,
        font_size: 10,
        data_source: 'spec_name',
        alignment: 'center',
      }),
    ];
  },
};

export type TextAlign = 'left' | 'center' | 'right';
export type VerticalAlign = 'top' | 'middle' | 'bottom';

// Supported label fields for UI — key must match build_label_data() JSON keys
export interface SupportedLabelField {
  key: string;
  type: 'text' | 'image' | 'separator';
  label: string;
  category: string;
  description: string;
  example: string;
  source_type?: 'productImage' | 'qrCode' | 'barcode';
}

// key 必须与 edge-server/src/printing/executor.rs 的 build_label_data 一致。
// 添加新字段：1) build_label_data 加 key  2) 这里加条目  3) PrintItemContext 扩展
export const SUPPORTED_LABEL_FIELDS: SupportedLabelField[] = [
  // Product
  { key: 'product_name', type: 'text', label: '商品名称', category: 'Product', description: '商品原始名称', example: 'Coffee' },
  { key: 'kitchen_name', type: 'text', label: '厨房显示名', category: 'Product', description: '厨房打印名(=商品名)', example: 'Coffee' },
  { key: 'category_name', type: 'text', label: '分类名称', category: 'Product', description: '商品分类', example: 'Drinks' },
  { key: 'external_id', type: 'text', label: '外部ID', category: 'Product', description: '商品外部ID', example: '10042' },
  // Specification
  { key: 'spec_name', type: 'text', label: '规格名称', category: 'Product', description: '商品规格', example: 'Large' },
  // Item
  { key: 'quantity', type: 'text', label: '数量', category: 'Item', description: '商品数量', example: '2' },
  { key: 'index', type: 'text', label: '序号', category: 'Item', description: '商品在订单中的序号', example: '1/3' },
  { key: 'options', type: 'text', label: '选项', category: 'Item', description: '商品选项/加料', example: 'No sugar' },
  { key: 'note', type: 'text', label: '备注', category: 'Item', description: '商品备注', example: '少辣' },
  // Order
  { key: 'table_name', type: 'text', label: '桌号', category: 'Order', description: '桌号名称', example: 'Mesa 5' },
  { key: 'queue_number', type: 'text', label: '叫号', category: 'Order', description: '零售叫号(#001格式)', example: '#001' },
  // Print
  { key: 'time', type: 'text', label: '时间', category: 'Print', description: '打印时间(HH:MM)', example: '14:30' },
  { key: 'date', type: 'text', label: '日期', category: 'Print', description: '打印日期(YYYY-MM-DD)', example: '2026-02-26' },
  { key: 'datetime', type: 'text', label: '日期时间', category: 'Print', description: '打印日期时间', example: '2026-02-26 14:30' },
];
