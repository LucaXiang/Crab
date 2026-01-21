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

  // Label printing
  printLabel(config: LabelPrintConfig): Promise<void>;
  printMultipleLabels(configs: LabelPrintConfig[]): Promise<void>;

  // Cash drawer
  openCashDrawer(printerName?: string): Promise<void>;

  // List printers
  listPrinters(): Promise<string[]>;
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
};

// Export individual functions
export const printReceipt = printService.printReceipt;
export const reprintReceipt = printService.reprintReceipt;
export const printKitchenTicket = printService.printKitchenTicket;
export const openCashDrawer = printService.openCashDrawer;
export const listPrinters = printService.listPrinters;

export default printService;
