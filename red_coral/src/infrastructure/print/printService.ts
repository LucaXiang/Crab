/**
 * Print Service
 * Handles all printing operations (receipts, kitchen tickets, labels)
 */

import { invoke } from '@tauri-apps/api/core';
import type { ReceiptPrintConfig, KitchenTicketPrintConfig, LabelPrintConfig } from '@/core/domain/types/print';

export interface PrintService {
  // Receipt printing
  printReceipt(config: ReceiptPrintConfig): Promise<void>;
  reprintReceipt(orderId: string): Promise<void>;

  // Kitchen ticket printing
  printKitchenTicket(config: KitchenTicketPrintConfig): Promise<void>;
  printMultipleKitchenTickets(configs: KitchenTicketPrintConfig[]): Promise<void>;

  // Legacy kitchen ticket printing (accepts individual parameters)
  printKitchenTicketLegacy(order: any, isReprint: boolean, isVoid: boolean, mode: string, items: any[]): Promise<void>;

  // Label printing
  printLabel(config: LabelPrintConfig): Promise<void>;
  printMultipleLabels(configs: LabelPrintConfig[]): Promise<void>;

  // Cash drawer
  openCashDrawer(printerName?: string): Promise<void>;

  // List printers
  listPrinters(): Promise<string[]>;

  // Status
  getPrinterStatus(): Promise<PrinterStatus>;
  getPrintJobs(): Promise<PrintJob[]>;
}

export interface PrinterStatus {
  connected: boolean;
  printerName: string;
  paperStatus: 'ok' | 'low' | 'out';
  error?: string;
}

export interface PrintJob {
  id: string;
  type: 'receipt' | 'kitchen' | 'label';
  status: 'pending' | 'printing' | 'completed' | 'failed';
  createdAt: string;
  completedAt?: string;
  error?: string;
}

export const printService: PrintService = {
  async printReceipt(config: ReceiptPrintConfig): Promise<void> {
    await invoke('print_receipt', { config });
  },

  async reprintReceipt(orderId: string): Promise<void> {
    await invoke('reprint_receipt', { orderId });
  },

  async printKitchenTicket(config: KitchenTicketPrintConfig): Promise<void> {
    await invoke('print_kitchen_ticket', { config });
  },

  async printMultipleKitchenTickets(configs: KitchenTicketPrintConfig[]): Promise<void> {
    await invoke('print_multiple_kitchen_tickets', { configs });
  },

  async printKitchenTicketLegacy(order: any, isReprint: boolean, isVoid: boolean, mode: string, items: any[]): Promise<void> {
    // Build KitchenTicketData from legacy parameters
    const data = {
      printer_name: order.kitchenPrinterName || '',
      table_name: order.tableName || order.tableName || '',
      order_id: order.receiptNumber || order.id,
      items: items.map((item: any) => ({
        name: item.name,
        quantity: item.quantity,
        category: item.category,
        selected_options: item.selectedOptions?.map((opt: any) => opt.name) || [],
        notes: item.notes || '',
        external_id: String(item.externalId || ''),
      })),
      timestamp: Date.now(),
      is_reprint: isReprint,
      server_name: order.serverName || '',
      mode,
      service_type: order.retailServiceType || null,
    };
    await invoke('print_kitchen_ticket_cmd', { data });
  },

  async printLabel(config: LabelPrintConfig): Promise<void> {
    await invoke('print_label', { config });
  },

  async printMultipleLabels(configs: LabelPrintConfig[]): Promise<void> {
    await invoke('print_multiple_labels', { configs });
  },

  async openCashDrawer(printerName?: string): Promise<void> {
    await invoke('open_cash_drawer_cmd', { printer_name: printerName });
  },

  async listPrinters(): Promise<string[]> {
    return invoke('list_printers');
  },

  async getPrinterStatus(): Promise<PrinterStatus> {
    return invoke('get_printer_status');
  },

  async getPrintJobs(): Promise<PrintJob[]> {
    return invoke('get_print_jobs');
  },
};

// Export individual functions
export const printReceipt = printService.printReceipt;
export const reprintReceipt = printService.reprintReceipt;
export const printKitchenTicket = printService.printKitchenTicket;
export const printKitchenTicketLegacy = printService.printKitchenTicketLegacy;
export const openCashDrawer = printService.openCashDrawer;
export const listPrinters = printService.listPrinters;

export default printService;
