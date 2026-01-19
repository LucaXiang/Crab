/**
 * Print Service
 * Provides receipt and label printing functionality
 */

import { invoke } from '@tauri-apps/api/core';
import type { LabelTemplate } from '@/core/domain/types/print';

/**
 * Print a receipt
 */
export async function printReceipt(orderId: string): Promise<void> {
  return invoke('print_receipt', { orderId });
}

/**
 * Reprint a receipt
 */
export async function reprintReceipt(orderId: string): Promise<void> {
  return invoke('reprint_receipt', { orderId });
}

/**
 * Open cash drawer
 */
export async function openCashDrawer(): Promise<void> {
  return invoke('open_cash_drawer');
}

/**
 * Print kitchen ticket
 */
export async function printKitchenTicket(orderId: string): Promise<void> {
  return invoke('print_kitchen_ticket', { orderId });
}

/**
 * List available printers
 */
export async function listPrinters(): Promise<Array<{ name: string; type: string }>> {
  return invoke('list_printers');
}

/**
 * Print item label
 */
export async function printItemLabel(itemId: string, templateId?: string): Promise<void> {
  return invoke('print_item_label', { itemId, templateId });
}

/**
 * Convert label template to Rust format
 * Converts camelCase to snake_case for backend compatibility
 */
export function convertTemplateToRust(template: LabelTemplate): Record<string, unknown> {
  return {
    id: template.id,
    name: template.name,
    description: template.description,
    width: template.width,
    height: template.height,
    padding: template.padding,
    fields: template.fields.map(field => ({
      id: field.id,
      name: field.name,
      type: field.type,
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
    })),
    is_default: template.isDefault,
    is_active: template.isActive,
  };
}
