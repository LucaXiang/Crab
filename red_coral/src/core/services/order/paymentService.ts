/**
 * paymentService - 支付硬件操作服务
 *
 * 职责：打开钱箱
 *
 * 注意：支付逻辑由后端 Event Sourcing 处理，
 * 收据打印由服务端处理。
 */

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
