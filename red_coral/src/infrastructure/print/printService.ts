/**
 * Print Service
 *
 * 本地打印功能通过 Tauri 命令调用：
 * - listPrinters: 获取本地驱动打印机列表
 * - openCashDrawer: 打开钱箱
 * - printReceipt: 打印收据
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

// ── Receipt Data Types (与 Rust ReceiptData 对齐) ──

export interface ReceiptStoreInfo {
  name: string;
  address: string;
  nif: string;
  phone: string | null;
  email: string | null;
  website: string | null;
  logo_url: string | null;
}

export interface ReceiptSurchargeInfo {
  name: string;
  type: string;
  value: number;
  amount: number;
}

export interface ReceiptDiscountInfo {
  name: string;
  type: string;
  value: number;
  amount: number;
}

export interface ReceiptRuleAdjustment {
  name: string;
  rule_type: string;
  adjustment_type: string;
  value: number;
  amount: number;
}

export interface ReceiptSelectedOption {
  attribute_name: string;
  option_name: string;
  receipt_name: string | null;
  price_modifier: number;
  show_on_receipt: boolean;
}

export interface ReceiptItem {
  name: string;
  quantity: number;
  price: number;
  total: number;
  tax_rate: number | null;
  discount_percent: number | null;
  original_price: number | null;
  selected_options: ReceiptSelectedOption[] | null;
  spec_name: string | null;
}

export interface ReceiptData {
  order_id: string;
  timestamp: string;
  table_name: string;
  zone_name: string | null;
  guest_count: number | null;
  opened_at: string | null;
  checkout_time: string | null;
  void_reason: string | null;
  reprint: boolean;
  pre_payment: boolean;
  store_info: ReceiptStoreInfo | null;
  surcharge: ReceiptSurchargeInfo | null;
  discount: ReceiptDiscountInfo | null;
  rule_adjustments: ReceiptRuleAdjustment[];
  items: ReceiptItem[];
  total_amount: number;
  queue_number: number | null;
  qr_data: string | null;
}

// ── Service Functions ──

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

/**
 * 打印收据
 * @param printerName Windows 驱动打印机名称
 * @param receipt 收据数据
 */
export async function printReceipt(printerName: string | null, receipt: ReceiptData): Promise<void> {
  const response = await invoke<ApiResponse<null>>('print_receipt', {
    printer_name: printerName,
    receipt,
  });
  if (response.code !== 0) {
    throw new Error(response.message);
  }
}
