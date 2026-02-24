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
  data_source: string;  // Field data source path
  format?: string;  // Format pattern (e.g., for date/time)
  visible: boolean;
  // UI-specific properties for editor
  label?: string;
  template?: string;
  data_key?: string;
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
  created_at: number;
  updated_at: number;
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
}

/**
 * Update label template params
 */
export interface UpdateLabelTemplateParams extends Partial<CreateLabelTemplateParams> {
  id: string;
}

/**
 * Label print job
 */
export interface LabelPrintJob {
  id: string;
  template_id: string;
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
        data_source: 'product.name',
        visible: true,
      },
      {
        field_id: 'price',
        name: '价格',
        field_type: 'price',
        x: 8,
        y: 64,
        width: 224,
        height: 32,
        font_size: 12,
        font_weight: 'bold',
        data_source: 'product.price',
        format: '€{value}',
        visible: true,
      },
      {
        field_id: 'barcode',
        name: '条码',
        field_type: 'barcode',
        x: 8,
        y: 112,
        width: 224,
        height: 40,
        font_size: 8,
        data_source: 'product.externalId',
        format: 'CODE128',
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
        y: 16,
        width: 288,
        height: 48,
        font_size: 14,
        font_weight: 'bold',
        data_source: 'product.name',
        visible: true,
      },
      {
        field_id: 'spec',
        name: '规格',
        field_type: 'text',
        x: 16,
        y: 80,
        width: 160,
        height: 32,
        font_size: 10,
        data_source: 'specification.name',
        visible: true,
      },
      {
        field_id: 'price',
        name: '价格',
        field_type: 'price',
        x: 192,
        y: 80,
        width: 112,
        height: 32,
        font_size: 12,
        font_weight: 'bold',
        data_source: 'product.price',
        format: '€{value}',
        alignment: 'right',
        visible: true,
      },
      {
        field_id: 'barcode',
        name: '条码',
        field_type: 'barcode',
        x: 16,
        y: 128,
        width: 288,
        height: 80,
        font_size: 10,
        data_source: 'product.externalId',
        format: 'CODE128',
        visible: true,
      },
      {
        field_id: 'datetime',
        name: '打印时间',
        field_type: 'datetime',
        x: 16,
        y: 216,
        width: 288,
        height: 24,
        font_size: 8,
        data_source: 'print.time',
        format: 'yyyy-MM-dd HH:mm',
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
        data_source: 'order.receiptNumber',
        alignment: 'center',
        visible: true,
      },
      {
        field_id: 'table',
        name: '桌号',
        field_type: 'text',
        x: 24,
        y: 112,
        width: 176,
        height: 48,
        font_size: 18,
        font_weight: 'bold',
        data_source: 'order.tableName',
        visible: true,
      },
      {
        field_id: 'quantity',
        name: '数量',
        field_type: 'text',
        x: 224,
        y: 112,
        width: 152,
        height: 48,
        font_size: 18,
        font_weight: 'bold',
        data_source: 'item.quantity',
        alignment: 'right',
        visible: true,
      },
      {
        field_id: 'itemName',
        name: '菜品名称',
        field_type: 'text',
        x: 24,
        y: 184,
        width: 352,
        height: 80,
        font_size: 16,
        font_weight: 'bold',
        data_source: 'item.productName',
        alignment: 'center',
        visible: true,
      },
      {
        field_id: 'options',
        name: '加料',
        field_type: 'text',
        x: 24,
        y: 280,
        width: 352,
        height: 32,
        font_size: 10,
        color: '#666666',
        data_source: 'item.options',
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
          data_source: 'product.name',
        }),
        this.createField({
          name: '价格',
          field_type: 'price',
          x: 1,
          y: 10,
          width: 28,
          height: 5,
          font_size: 12,
          data_source: 'product.price',
          format: '€{value}',
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
        data_source: 'product.name',
        alignment: 'center',
      }),
      this.createField({
        name: '价格',
        field_type: 'price',
        x: 2,
        y: 14,
        width: width - 4,
        height: 6,
        font_size: 16,
        data_source: 'product.price',
        format: '€{value}',
        alignment: 'center',
      }),
    ];
  },
};

// Additional types for UI components
export type TextAlign = 'left' | 'center' | 'right';
export type VerticalAlign = 'top' | 'middle' | 'bottom';

export interface TextField {
  type: 'text';
  label: string;
  data_key: string;
  style?: TextStyle;
  align?: TextAlign;
}

export interface ImageField {
  type: 'image';
  source_type: 'productImage' | 'qrCode' | 'barcode';
  template?: string;
  maintain_aspect_ratio?: boolean;
}

export interface SeparatorField {
  type: 'separator';
  line_style?: 'solid' | 'dashed' | 'dotted';
}

export type FieldType = 'text' | 'image' | 'separator';

export interface TextStyle {
  font_size: number;
  font_weight?: string;
  color?: string;
  font_family?: string;
}

// Supported label fields for UI
export interface SupportedLabelField {
  key: string;
  type: 'text' | 'image' | 'separator';
  label: string;
  category: string;
  description: string;
  example: string;
  data_key?: string;
  source_type?: 'productImage' | 'qrCode' | 'barcode';
}

// key 必须与 edge-server/src/printing/executor.rs 的 build_label_data 一致
export const SUPPORTED_LABEL_FIELDS: SupportedLabelField[] = [
  { key: 'product_name', type: 'text', label: '商品名称', category: 'Product', description: '商品原始名称', example: 'Coffee' },
  { key: 'kitchen_name', type: 'text', label: '厨房显示名', category: 'Product', description: '厨房打印名(=商品名)', example: 'Coffee' },
  { key: 'category_name', type: 'text', label: '分类名称', category: 'Product', description: '商品分类', example: 'Drinks' },
  { key: 'spec_name', type: 'text', label: '规格名称', category: 'Product', description: '商品规格', example: 'Large' },
  { key: 'options', type: 'text', label: '选项', category: 'Product', description: '商品选项/加料', example: 'No sugar' },
  { key: 'quantity', type: 'text', label: '数量', category: 'Item', description: '商品数量', example: '2' },
  { key: 'index', type: 'text', label: '序号', category: 'Item', description: '商品在订单中的序号', example: '1/3' },
  { key: 'note', type: 'text', label: '备注', category: 'Item', description: '商品备注', example: '少辣' },
  { key: 'external_id', type: 'text', label: '外部ID', category: 'Product', description: '商品外部ID', example: '10042' },
  { key: 'table_name', type: 'text', label: '桌号', category: 'Order', description: '桌号名称', example: 'Mesa 5' },
  { key: 'queue_number', type: 'text', label: '叫号', category: 'Order', description: '零售叫号(#001格式)', example: '#001' },
  { key: 'time', type: 'text', label: '打印时间', category: 'Print', description: '打印时间(HH:MM)', example: '14:30' },
];
