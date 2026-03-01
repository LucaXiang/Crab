/**
 * Print Types
 * Types for receipt, kitchen ticket, and label printing
 */

export interface ReceiptPrintConfig {
  orderId: number;
  printerId?: number;
  copyType: 'original' | 'copy' | 'reprint';
  includePaymentDetails?: boolean;
  includeKitchenTickets?: boolean;
}

export interface KitchenTicketPrintConfig {
  orderId: number;
  printerId?: number;
  items: string[]; // item IDs to print
  note?: string;
  priority?: 'normal' | 'rush';
}

export interface LabelPrintConfig {
  orderId: number;
  itemId: string;
  printerId?: number;
  templateId?: number;
  copies?: number;
}
