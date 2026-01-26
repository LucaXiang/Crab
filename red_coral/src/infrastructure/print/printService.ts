/**
 * Print Service
 *
 * 本地打印功能通过 Tauri 命令调用：
 * - listPrinters: 获取本地驱动打印机列表
 * - openCashDrawer: 打开钱箱
 *
 * 收据/厨房票据打印由服务端处理。
 */

import { invoke } from '@tauri-apps/api/core';
import type { ReceiptPrintConfig, KitchenTicketPrintConfig, LabelPrintConfig } from '@/core/domain/types/print';
import { t } from '@/infrastructure/i18n';

/** API 响应格式 */
interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

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
  console.warn('[PrintService] 此打印功能由服务端处理，请使用对应的 API');
};

/**
 * 获取本地驱动打印机列表
 */
async function listPrintersImpl(): Promise<string[]> {
  try {
    const response = await invoke<ApiResponse<string[]>>('list_printers');
    if (response.code === 0) {
      return response.data;
    }
    console.warn('[PrintService] 获取打印机列表失败:', response.message);
    return [];
  } catch (error) {
    console.error('[PrintService] 获取打印机列表错误:', error);
    return [];
  }
}

/**
 * 打开钱箱
 * @param printerName 打印机名称，不传则使用默认打印机
 */
async function openCashDrawerImpl(printerName?: string): Promise<void> {
  try {
    const response = await invoke<ApiResponse<null>>('open_cash_drawer', {
      printer_name: printerName ?? null,
    });
    if (response.code !== 0) {
      throw new Error(response.message);
    }
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new Error(t('common.message.cash_drawer_failed', { message }));
  }
}

export const printService: PrintService = {
  printReceipt: notImplemented,
  reprintReceipt: notImplemented,
  printKitchenTicket: notImplemented,
  printMultipleKitchenTickets: notImplemented,
  printLabel: notImplemented,
  printMultipleLabels: notImplemented,
  openCashDrawer: openCashDrawerImpl,
  listPrinters: listPrintersImpl,
};

// Export individual functions
export const printReceipt = printService.printReceipt;
export const reprintReceipt = printService.reprintReceipt;
export const printKitchenTicket = printService.printKitchenTicket;
export const openCashDrawer = printService.openCashDrawer;
export const listPrinters = printService.listPrinters;

export default printService;
