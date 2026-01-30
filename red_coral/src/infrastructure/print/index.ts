/**
 * Print Service - Re-exports from printService
 */

import type { LabelTemplate, LabelField } from '@/core/domain/types/print';

export { printService, printService as default } from './printService';
export type { PrintService } from './printService';

// Re-export commonly used functions
export {
  printReceipt,
  reprintReceipt,
  printKitchenTicket,
  openCashDrawer,
  listPrinters,
} from './printService';

/**
 * Convert a LabelField from camelCase to snake_case for Rust backend
 */
function convertFieldToRust(field: LabelField): Record<string, unknown> {
  return {
    id: field.id,
    name: field.name,
    field_type: field.type,
    x: field.x,
    y: field.y,
    width: field.width,
    height: field.height,
    font_size: field.fontSize,
    font_weight: field.fontWeight,
    font_family: field.fontFamily,
    color: field.color,
    rotate: field.rotate,
    alignment: field.alignment,
    data_source: field.dataSource,
    format: field.format,
    visible: field.visible,
    label: field.label,
    template: field.template,
    data_key: field.dataKey,
    source_type: field.sourceType,
    maintain_aspect_ratio: field.maintainAspectRatio,
    style: field.style,
    align: field.align,
    vertical_align: field.verticalAlign,
    line_style: field.lineStyle,
  };
}

/**
 * Convert a LabelTemplate from camelCase to snake_case for Rust backend
 */
export function convertTemplateToRust(template: LabelTemplate): Record<string, unknown> {
  return {
    id: template.id,
    name: template.name,
    description: template.description,
    width: template.widthMm || template.width,
    height: template.heightMm || template.height,
    fields: template.fields.map(convertFieldToRust),
    is_default: template.isDefault,
    is_active: template.isActive,
    created_at: template.createdAt,
    updated_at: template.updatedAt,
    padding_mm_x: template.paddingMmX,
    padding_mm_y: template.paddingMmY,
    render_dpi: template.renderDpi,
    test_data: template.testData,
  };
}
