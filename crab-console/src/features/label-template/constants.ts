import type { LabelField, LabelTemplate } from '@/core/types/store';
import { LabelFieldType, LabelFieldAlignment } from '@/core/types/store';

// ── Supported label fields ──

export interface SupportedLabelField {
  key: string;
  type: 'text' | 'image' | 'separator';
  label: string;
  category: string;
  description: string;
  example: string;
}

// key must match edge-server/src/printing/executor.rs build_label_data
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

// ── Label field helpers ──

export const LabelFieldHelpers = {
  createField(overrides: Partial<LabelField> = {}): LabelField {
    return {
      field_id: crypto.randomUUID(),
      name: '',
      field_type: LabelFieldType.Text,
      x: 0,
      y: 0,
      width: 20,
      height: 10,
      font_size: 10,
      font_weight: 'normal',
      font_family: 'Arial',
      color: '#000000',
      rotate: 0,
      alignment: LabelFieldAlignment.Left,
      data_source: '',
      format: '',
      visible: true,
      ...overrides,
    };
  },
};

// ── Default templates ──

export const DEFAULT_LABEL_TEMPLATES: Omit<LabelTemplate, 'id' | 'created_at' | 'updated_at'>[] = [
  {
    name: 'Etiqueta pequeña (30×20)',
    description: 'Para etiquetas pequeñas',
    width: 30,
    height: 20,
    width_mm: 30,
    height_mm: 20,
    padding: 1,
    is_default: true,
    is_active: true,
    fields: [
      LabelFieldHelpers.createField({
        field_id: 'name',
        name: '商品名称',
        field_type: LabelFieldType.Text,
        x: 8, y: 16, width: 224, height: 32,
        font_size: 10, font_weight: 'bold',
        data_source: 'product.name',
      }),
      LabelFieldHelpers.createField({
        field_id: 'price',
        name: '价格',
        field_type: LabelFieldType.Price,
        x: 8, y: 64, width: 224, height: 32,
        font_size: 12, font_weight: 'bold',
        data_source: 'product.price',
        format: '€{value}',
      }),
      LabelFieldHelpers.createField({
        field_id: 'barcode',
        name: '条码',
        field_type: LabelFieldType.Barcode,
        x: 8, y: 112, width: 224, height: 40,
        font_size: 8,
        data_source: 'product.externalId',
        format: 'CODE128',
      }),
    ],
  },
  {
    name: 'Etiqueta estándar (40×30)',
    description: 'Para etiquetas estándar',
    width: 40,
    height: 30,
    width_mm: 40,
    height_mm: 30,
    padding: 2,
    is_default: false,
    is_active: true,
    fields: [
      LabelFieldHelpers.createField({
        field_id: 'name',
        name: '商品名称',
        field_type: LabelFieldType.Text,
        x: 16, y: 16, width: 288, height: 48,
        font_size: 14, font_weight: 'bold',
        data_source: 'product.name',
      }),
      LabelFieldHelpers.createField({
        field_id: 'price',
        name: '价格',
        field_type: LabelFieldType.Price,
        x: 192, y: 80, width: 112, height: 32,
        font_size: 12, font_weight: 'bold',
        data_source: 'product.price',
        format: '€{value}',
        alignment: LabelFieldAlignment.Right,
      }),
      LabelFieldHelpers.createField({
        field_id: 'barcode',
        name: '条码',
        field_type: LabelFieldType.Barcode,
        x: 16, y: 128, width: 288, height: 80,
        font_size: 10,
        data_source: 'product.externalId',
        format: 'CODE128',
      }),
    ],
  },
];
