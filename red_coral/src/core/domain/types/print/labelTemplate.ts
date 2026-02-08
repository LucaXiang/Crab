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
  id: string;
  name: string;
  type: LabelFieldType;
  x: number;  // X position in mm
  y: number;  // Y position in mm
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
  data: Record<string, any>;
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
    fields: [
      {
        id: 'name',
        name: '商品名称',
        type: 'text',
        x: 1,
        y: 2,
        width: 28,
        height: 4,
        font_size: 10,
        font_weight: 'bold',
        data_source: 'product.name',
        visible: true,
      },
      {
        id: 'price',
        name: '价格',
        type: 'price',
        x: 1,
        y: 8,
        width: 28,
        height: 4,
        font_size: 12,
        font_weight: 'bold',
        data_source: 'product.price',
        format: '€{value}',
        visible: true,
      },
      {
        id: 'barcode',
        name: '条码',
        type: 'barcode',
        x: 1,
        y: 14,
        width: 28,
        height: 5,
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
    fields: [
      {
        id: 'name',
        name: '商品名称',
        type: 'text',
        x: 2,
        y: 2,
        width: 36,
        height: 6,
        font_size: 14,
        font_weight: 'bold',
        data_source: 'product.name',
        visible: true,
      },
      {
        id: 'spec',
        name: '规格',
        type: 'text',
        x: 2,
        y: 10,
        width: 20,
        height: 4,
        font_size: 10,
        data_source: 'specification.name',
        visible: true,
      },
      {
        id: 'price',
        name: '价格',
        type: 'price',
        x: 24,
        y: 10,
        width: 14,
        height: 4,
        font_size: 12,
        font_weight: 'bold',
        data_source: 'product.price',
        format: '€{value}',
        alignment: 'right',
        visible: true,
      },
      {
        id: 'barcode',
        name: '条码',
        type: 'barcode',
        x: 2,
        y: 16,
        width: 36,
        height: 10,
        font_size: 10,
        data_source: 'product.externalId',
        format: 'CODE128',
        visible: true,
      },
      {
        id: 'datetime',
        name: '打印时间',
        type: 'datetime',
        x: 2,
        y: 28,
        width: 36,
        height: 3,
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
    fields: [
      {
        id: 'orderNum',
        name: '订单号',
        type: 'text',
        x: 3,
        y: 3,
        width: 44,
        height: 8,
        font_size: 24,
        font_weight: 'bold',
        data_source: 'order.receiptNumber',
        alignment: 'center',
        visible: true,
      },
      {
        id: 'table',
        name: '桌号',
        type: 'text',
        x: 3,
        y: 14,
        width: 22,
        height: 6,
        font_size: 18,
        font_weight: 'bold',
        data_source: 'order.tableName',
        visible: true,
      },
      {
        id: 'quantity',
        name: '数量',
        type: 'text',
        x: 28,
        y: 14,
        width: 19,
        height: 6,
        font_size: 18,
        font_weight: 'bold',
        data_source: 'item.quantity',
        alignment: 'right',
        visible: true,
      },
      {
        id: 'itemName',
        name: '菜品名称',
        type: 'text',
        x: 3,
        y: 23,
        width: 44,
        height: 10,
        font_size: 16,
        font_weight: 'bold',
        data_source: 'item.productName',
        alignment: 'center',
        visible: true,
      },
      {
        id: 'options',
        name: '加料',
        type: 'text',
        x: 3,
        y: 35,
        width: 44,
        height: 4,
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
      id: crypto.randomUUID(),
      name: '',
      type: 'text',
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
          type: 'text',
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
          type: 'price',
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
        type: 'text',
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
        type: 'price',
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

export const SUPPORTED_LABEL_FIELDS: SupportedLabelField[] = [
  { key: 'product.name', type: 'text', label: '商品名称', category: 'Product', description: '商品名称', example: 'Coffee' },
  { key: 'product.price', type: 'text', label: '商品价格', category: 'Product', description: '商品价格', example: '$10.00' },
  { key: 'product.barcode', type: 'text', label: '商品条码', category: 'Product', description: '商品条码', example: '123456789' },
  { key: 'specification.name', type: 'text', label: '规格名称', category: 'Specification', description: '商品规格', example: 'Large' },
  { key: 'product.image', type: 'image', label: '商品图片', category: 'Product', description: '商品图片', example: '/path/to/image.png' },
  { key: 'order.receiptNumber', type: 'text', label: '订单号', category: 'Order', description: '订单编号', example: 'ORD-001' },
  { key: 'order.tableName', type: 'text', label: '桌号', category: 'Order', description: '桌号名称', example: 'Table 5' },
  { key: 'item.quantity', type: 'text', label: '数量', category: 'Item', description: '商品数量', example: '2' },
  { key: 'item.productName', type: 'text', label: '商品名', category: 'Item', description: '订单中的商品名', example: 'Coffee' },
  { key: 'item.options', type: 'text', label: '选项', category: 'Item', description: '商品选项/加料', example: 'No sugar' },
  { key: 'print.time', type: 'text', label: '打印时间', category: 'Print', description: '打印时间戳', example: '2024-01-01 12:00' },
];
