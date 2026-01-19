/**
 * Print Types
 * Types for receipt, kitchen ticket, and label printing
 */

export interface ReceiptPrintConfig {
  orderId: string;
  printerId?: number;
  copyType: 'original' | 'copy' | 'reprint';
  includePaymentDetails?: boolean;
  includeKitchenTickets?: boolean;
}

export interface KitchenTicketPrintConfig {
  orderId: string;
  printerId?: number;
  items: string[]; // item IDs to print
  note?: string;
  priority?: 'normal' | 'rush';
}

export interface LabelPrintConfig {
  orderId: string;
  itemId: string;
  printerId?: number;
  templateId?: number;
  copies?: number;
}
