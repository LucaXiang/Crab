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
  fontSize: number;
  fontWeight?: string;
  fontFamily?: string;
  color?: string;
  rotate?: number;
  alignment?: 'left' | 'center' | 'right';
  dataSource: string;  // Field data source path
  format?: string;  // Format pattern (e.g., for date/time)
  visible: boolean;
  // UI-specific properties for editor
  label?: string;
  template?: string;
  dataKey?: string;
  sourceType?: 'productImage' | 'qrCode' | 'barcode' | 'image';
  maintainAspectRatio?: boolean;
  /** Temporary local file path for pending image upload (editor only, not persisted) */
  _pendingImagePath?: string;
  style?: string;
  align?: 'left' | 'center' | 'right';
  verticalAlign?: 'top' | 'middle' | 'bottom';
  lineStyle?: 'solid' | 'dashed' | 'dotted';
}

/**
 * Label template
 */
export interface LabelTemplate {
  id: string;
  name: string;
  description?: string;
  width: number;  // Label width in mm
  height: number;  // Label height in mm
  padding: number;
  fields: LabelField[];
  isDefault: boolean;
  isActive: boolean;
  createdAt: string;
  updatedAt: string;
  // UI-specific properties
  widthMm?: number;
  heightMm?: number;
  paddingMmX?: number;
  paddingMmY?: number;
  renderDpi?: number;
  testData?: string;
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
  isDefault?: boolean;
  isActive?: boolean;
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
  templateId: string;
  data: Record<string, any>;
  quantity: number;
  status: 'pending' | 'printing' | 'completed' | 'failed';
  printerId?: number;
  createdAt: string;
  printedAt?: string;
  error?: string;
}

/**
 * Predefined label templates
 */
export const DEFAULT_LABEL_TEMPLATES: LabelTemplate[] = [
  {
    id: 'default-small',
    name: '小标签 (30x20)',
    description: '适用于小规格标签',
    width: 30,
    height: 20,
    widthMm: 30,
    heightMm: 20,
    padding: 1,
    isDefault: true,
    isActive: true,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    fields: [
      {
        id: 'name',
        name: '商品名称',
        type: 'text',
        x: 1,
        y: 2,
        width: 28,
        height: 4,
        fontSize: 10,
        fontWeight: 'bold',
        dataSource: 'product.name',
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
        fontSize: 12,
        fontWeight: 'bold',
        dataSource: 'product.price',
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
        fontSize: 8,
        dataSource: 'product.externalId',
        format: 'CODE128',
        visible: true,
      },
    ],
  },
  {
    id: 'default-standard',
    name: '标准标签 (40x30)',
    description: '适用于标准规格标签',
    width: 40,
    height: 30,
    widthMm: 40,
    heightMm: 30,
    padding: 2,
    isDefault: false,
    isActive: true,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    fields: [
      {
        id: 'name',
        name: '商品名称',
        type: 'text',
        x: 2,
        y: 2,
        width: 36,
        height: 6,
        fontSize: 14,
        fontWeight: 'bold',
        dataSource: 'product.name',
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
        fontSize: 10,
        dataSource: 'specification.name',
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
        fontSize: 12,
        fontWeight: 'bold',
        dataSource: 'product.price',
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
        fontSize: 10,
        dataSource: 'product.externalId',
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
        fontSize: 8,
        dataSource: 'print.time',
        format: 'yyyy-MM-dd HH:mm',
        alignment: 'center',
        visible: true,
      },
    ],
  },
  {
    id: 'default-kitchen',
    name: '厨打标签 (50x40)',
    description: '适用于厨房打印',
    width: 50,
    height: 40,
    widthMm: 50,
    heightMm: 40,
    padding: 3,
    isDefault: false,
    isActive: true,
    createdAt: new Date().toISOString(),
    updatedAt: new Date().toISOString(),
    fields: [
      {
        id: 'orderNum',
        name: '订单号',
        type: 'text',
        x: 3,
        y: 3,
        width: 44,
        height: 8,
        fontSize: 24,
        fontWeight: 'bold',
        dataSource: 'order.receiptNumber',
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
        fontSize: 18,
        fontWeight: 'bold',
        dataSource: 'order.tableName',
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
        fontSize: 18,
        fontWeight: 'bold',
        dataSource: 'item.quantity',
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
        fontSize: 16,
        fontWeight: 'bold',
        dataSource: 'item.productName',
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
        fontSize: 10,
        color: '#666666',
        dataSource: 'item.options',
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
      fontSize: 10,
      fontWeight: 'normal',
      fontFamily: 'Arial',
      color: '#000000',
      rotate: 0,
      alignment: 'left',
      dataSource: '',
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
          fontSize: 10,
          fontWeight: 'bold',
          dataSource: 'product.name',
        }),
        this.createField({
          name: '价格',
          type: 'price',
          x: 1,
          y: 10,
          width: 28,
          height: 5,
          fontSize: 12,
          dataSource: 'product.price',
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
        fontSize: 14,
        fontWeight: 'bold',
        dataSource: 'product.name',
        alignment: 'center',
      }),
      this.createField({
        name: '价格',
        type: 'price',
        x: 2,
        y: 14,
        width: width - 4,
        height: 6,
        fontSize: 16,
        dataSource: 'product.price',
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
  dataKey: string;
  style?: TextStyle;
  align?: TextAlign;
}

export interface ImageField {
  type: 'image';
  sourceType: 'productImage' | 'qrCode' | 'barcode';
  template?: string;
  maintainAspectRatio?: boolean;
}

export interface SeparatorField {
  type: 'separator';
  lineStyle?: 'solid' | 'dashed' | 'dotted';
}

export type FieldType = 'text' | 'image' | 'separator';

export interface TextStyle {
  fontSize: number;
  fontWeight?: string;
  color?: string;
  fontFamily?: string;
}

// Supported label fields for UI
export interface SupportedLabelField {
  key: string;
  type: 'text' | 'image' | 'separator';
  label: string;
  category: string;
  description: string;
  example: string;
  dataKey?: string;
  sourceType?: 'productImage' | 'qrCode' | 'barcode';
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
