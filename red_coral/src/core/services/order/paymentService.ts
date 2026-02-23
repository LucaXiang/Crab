/**
 * paymentService - 支付硬件操作服务
 *
 * 职责：打开钱箱、打印收据
 */

import type { HeldOrder } from '@/core/domain/types';
import type { ArchivedOrderDetail } from '@/core/domain/types/archivedOrder';
import { logger } from '@/utils/logger';

/**
 * 打开钱箱
 *
 * 失败不阻止支付流程，但会 toast 提示用户。
 */
export const openCashDrawer = async (): Promise<void> => {
  try {
    const { usePrinterStore } = await import('@/core/stores/printer/usePrinterStore');
    const { openCashDrawer: open } = await import('@/infrastructure/print');
    const state = usePrinterStore.getState();
    const printer = state.cashDrawerPrinter || state.receiptPrinter || undefined;
    await open(printer);
  } catch (error) {
    logger.warn('Cash drawer failed to open', { component: 'paymentService', action: 'openCashDrawer', error });
    // 钱箱打开失败不阻止支付流程，但提示用户
    try {
      const { toast } = await import('@/presentation/components/Toast');
      toast.warning('Cash drawer failed to open');
    } catch { /* ignore toast failure */ }
  }
};

/**
 * 打印订单收据
 *
 * 打印失败不阻断结账，错误重新抛出让调用方决定是否显示 toast。
 */
export const printOrderReceipt = async (
  order: HeldOrder,
  printerName: string | null,
  reprint = false,
): Promise<void> => {
  const { printReceipt } = await import('@/infrastructure/print');
  const { buildReceiptData } = await import('./receiptBuilder');
  const { useStoreInfoStore } = await import('@/core/stores/settings/useStoreInfoStore');

  const storeInfo = useStoreInfoStore.getState().info;
  const receipt = buildReceiptData(order, storeInfo, { reprint });
  await printReceipt(printerName, receipt);
};

/**
 * 打印预付单（账单）
 */
export const printPrePaymentReceipt = async (
  order: HeldOrder,
  printerName: string | null,
): Promise<void> => {
  const { printReceipt } = await import('@/infrastructure/print');
  const { buildReceiptData } = await import('./receiptBuilder');
  const { useStoreInfoStore } = await import('@/core/stores/settings/useStoreInfoStore');

  const storeInfo = useStoreInfoStore.getState().info;
  const receipt = buildReceiptData(order, storeInfo, { prePayment: true });
  await printReceipt(printerName, receipt);
};

/**
 * 重打归档订单收据
 */
export const reprintArchivedReceipt = async (
  order: ArchivedOrderDetail,
  printerName: string | null,
): Promise<void> => {
  const { printReceipt } = await import('@/infrastructure/print');
  const { buildArchivedReceiptData } = await import('./receiptBuilder');
  const { useStoreInfoStore } = await import('@/core/stores/settings/useStoreInfoStore');

  const storeInfo = useStoreInfoStore.getState().info;
  const receipt = buildArchivedReceiptData(order, storeInfo);
  await printReceipt(printerName, receipt);
};
