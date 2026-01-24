/**
 * Print Service
 *
 * NOTE: 打印功能现在由服务端 (edge-server) 处理。
 * 前端不再需要直接调用打印命令。
 * 保留此文件仅用于类型导出和向后兼容。
 */

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

// 打印功能已移至服务端，这些函数现在是空操作
const notImplemented = async () => {
  console.warn('[PrintService] 打印功能已由服务端处理，前端调用无效');
};

export const printService: PrintService = {
  printReceipt: notImplemented,
  reprintReceipt: notImplemented,
  printKitchenTicket: notImplemented,
  printMultipleKitchenTickets: notImplemented,
  printLabel: notImplemented,
  printMultipleLabels: notImplemented,
  openCashDrawer: notImplemented,
  listPrinters: async () => [],
};

// Export individual functions
export const printReceipt = printService.printReceipt;
export const reprintReceipt = printService.reprintReceipt;
export const printKitchenTicket = printService.printKitchenTicket;
export const openCashDrawer = printService.openCashDrawer;
export const listPrinters = printService.listPrinters;

export default printService;
