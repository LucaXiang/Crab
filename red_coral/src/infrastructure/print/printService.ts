/**
 * Print Service
 *
 * 本地打印功能通过 Tauri 命令调用：
 * - listPrinters: 获取本地驱动打印机列表
 * - openCashDrawer: 打开钱箱
 *
 * 收据/厨房票据/标签打印由服务端处理，前端无需调用。
 */

import { invoke } from '@tauri-apps/api/core';
import { logger } from '@/utils/logger';
import { t } from '@/infrastructure/i18n';

/** API 响应格式 */
interface ApiResponse<T> {
  code: number;
  message: string;
  data: T;
}

/**
 * 获取本地驱动打印机列表
 */
export async function listPrinters(): Promise<string[]> {
  try {
    const response = await invoke<ApiResponse<string[]>>('list_printers');
    if (response.code === 0) {
      return response.data;
    }
    logger.warn('Failed to list printers', { component: 'PrintService', detail: response.message });
    return [];
  } catch (error) {
    logger.error('Failed to list printers', error, { component: 'PrintService' });
    return [];
  }
}

/**
 * 打开钱箱
 * @param printerName 打印机名称，不传则使用默认打印机
 */
export async function openCashDrawer(printerName?: string): Promise<void> {
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
