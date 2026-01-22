/**
 * Print Service
 * Handles all printing operations (receipts, kitchen tickets, labels)
 */

import { invokeApi } from '@/infrastructure/api/tauri-client';
import type { ReceiptPrintConfig, KitchenTicketPrintConfig, LabelPrintConfig } from '@/core/domain/types/print';

export interface PrintService {
  printReceipt(config: ReceiptPrintConfig): Promise<void>;
  reprintReceipt(orderId: string): Promise<void>;
  printKitchenTicket(config: KitchenTicketPrintConfig): Promise<void>;
  printMultipleKitchenTickets(configs: KitchenTicketPrintConfig[]): Promise<void>;
  printLabel(config: LabelPrintConfig): Promise<void>;
  printMultipleLabels(configs: LabelPrintConfig[]): Promise<void>;
  openCashDrawer(printerName?: string): Promise<void>;
  listPrinters(): Promise<string[]>;
}

export const printService: PrintService = {
  printReceipt: (config) => invokeApi('print_receipt', { config }),
  reprintReceipt: (orderId) => invokeApi('reprint_receipt', { order_id: orderId }),
  printKitchenTicket: (config) => invokeApi('print_kitchen_ticket', { config }),
  printMultipleKitchenTickets: (configs) => invokeApi('print_multiple_kitchen_tickets', { configs }),
  printLabel: (config) => invokeApi('print_label', { config }),
  printMultipleLabels: (configs) => invokeApi('print_multiple_labels', { configs }),
  openCashDrawer: (printerName) => invokeApi('open_cash_drawer_cmd', { printer_name: printerName }),
  listPrinters: () => invokeApi('list_printers'),
};

// Export individual functions
export const printReceipt = printService.printReceipt;
export const reprintReceipt = printService.reprintReceipt;
export const printKitchenTicket = printService.printKitchenTicket;
export const openCashDrawer = printService.openCashDrawer;
export const listPrinters = printService.listPrinters;

export default printService;
