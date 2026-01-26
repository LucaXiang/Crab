/**
 * paymentService - 支付硬件操作服务
 *
 * 职责：
 * 1. 打开钱箱
 * 2. 打印小票
 *
 * 注意：支付逻辑由后端 Event Sourcing 处理
 */

import { HeldOrder } from '@/core/domain/types';
import { logger } from '@/utils/logger';
import { usePrinterStore } from '@/core/stores/printer';

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
 * 打印订单小票
 * 如果启用了"打印后自动开钱箱"，则在打印成功后自动打开钱箱
 */
export const printOrderReceipt = async (
  order: HeldOrder,
  printerName?: string
): Promise<void> => {
  try {
    const { printReceipt } = await import('@/infrastructure/print/printService');
    const orderId = order.receipt_number || order.order_id;
    await printReceipt({
      orderId,
      printerId: printerName ? parseInt(printerName) : undefined,
      copyType: 'original'
    });

    // 检查是否启用了"打印后自动开钱箱"
    const { autoOpenCashDrawerAfterReceipt } = usePrinterStore.getState();
    if (autoOpenCashDrawerAfterReceipt) {
      await openCashDrawer();
    }
  } catch (error) {
    logger.error('Failed to print receipt', error, { component: 'paymentService', action: 'printOrderReceipt' });
    throw new Error('RECEIPT_PRINT_FAILED');
  }
};
