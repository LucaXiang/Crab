/**
 * paymentService - 支付硬件操作服务
 *
 * 职责：打开钱箱、打印收据
 */

import type { HeldOrder } from '@/core/domain/types';
import { logger } from '@/utils/logger';

/**
 * 打开钱箱
 */
export const openCashDrawer = async (): Promise<void> => {
  try {
    const { openCashDrawer: open } = await import('@/infrastructure/print');
    await open();
  } catch (error) {
    logger.warn('Cash drawer failed to open', { component: 'paymentService', action: 'openCashDrawer', error });
    // 钱箱打开失败不应阻止支付流程
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
