/**
 * paymentService - 支付业务逻辑服务层
 *
 * 职责：
 * 1. 封装支付流程（现金、刷卡）
 * 2. 打开钱箱
 * 3. 打印小票
 * 4. 支付验证
 */

import { HeldOrder, PaymentRecord, CartItem } from '@/core/domain/types';
import { Currency } from '@/utils/currency';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';

// i18n placeholder - these messages are shown in checkout context with t() function

interface CashPaymentParams {
  amount: number;
  tenderedAmount: number;
  note?: string;
}

interface CardPaymentParams {
  amount: number;
  note?: string;
}


/**
 * 处理现金支付
 * 1. 验证金额
 * 2. 打开钱箱
 * 3. 返回支付记录
 */
export const processCashPayment = async (
  params: CashPaymentParams
): Promise<PaymentRecord> => {
  const { amount, tenderedAmount, note } = params;

  // 验证金额
  if (Currency.lt(tenderedAmount, amount)) {
    throw new Error('PAYMENT_AMOUNT_INSUFFICIENT');
  }

  // 打开钱箱
  try {
    const { openCashDrawer } = await import('@/infrastructure/print');
    await openCashDrawer();
  } catch (error) {
    // 钱箱打开失败不应阻止支付流程 - 这个用 toast.warning 显示
    logger.warn('Cash drawer failed to open, but payment continues', { component: 'paymentService', action: 'processCashPayment', error });
  }

  // 计算找零
  const change = Currency.sub(tenderedAmount, amount);

  // 创建支付记录
  const payment: PaymentRecord = {
    id: `pay-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    method: 'Cash',
    amount,
    timestamp: Date.now(),
    tendered: tenderedAmount,
    change: change.toNumber(),
    note: note, // Only use custom note if provided
  };

  return payment;
};

/**
 * 处理刷卡支付
 * 1. 验证金额
 * 2. 返回支付记录
 */
export const processCardPayment = async (
  params: CardPaymentParams
): Promise<PaymentRecord> => {
  const { amount, note } = params;

  // 验证金额
  if (Currency.lte(amount, 0)) {
    throw new Error('PAYMENT_AMOUNT_MUST_BE_POSITIVE');
  }

  // 创建支付记录
  const payment: PaymentRecord = {
    id: `pay-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
    method: 'VISA',
    amount,
    timestamp: Date.now(),
    note: note, // Only use custom note if provided
  };

  return payment;
};

/**
 * 打印订单小票
 */
export const printOrderReceipt = async (
  order: HeldOrder,
  printerName?: string
): Promise<void> => {
  try {
    const { printReceipt } = await import('@/infrastructure/print/printService');
    const orderId = order.receiptNumber || order.key || '';
    await printReceipt({
      orderId,
      printerId: printerName ? parseInt(printerName) : undefined,
      copyType: 'original'
    });
    // Toast messages handled by caller
  } catch (error) {
    logger.error('Failed to print receipt', error, { component: 'paymentService', action: 'printOrderReceipt' });
    throw new Error('RECEIPT_PRINT_FAILED');
  }
};

/**
 * 验证支付金额是否足够
 */
export const validatePaymentAmount = (
  totalPaid: number,
  orderTotal: number
): { isValid: boolean; remaining: number } => {
  const remaining = Currency.sub(orderTotal, totalPaid);
  const isValid = Currency.lte(remaining, 0.01); // 允许 1 分钱的误差

  return { isValid, remaining: Currency.max(0, remaining).toNumber() };
};

/**
 * 计算商品分单的未支付商品
 */
export const calculateUnpaidItems = (
  items: CartItem[],
  paidQuantities: Record<string, number>
): CartItem[] => {
  const unpaidItems: CartItem[] = [];

  items.forEach((item) => {
    if (item._removed) return;
    const itemKey = item.instanceId || item.id;
    const paidQty = paidQuantities[itemKey] || 0;
    const unpaidQty = item.quantity - paidQty;

    if (unpaidQty > 0) {
      unpaidItems.push({
        ...item,
        quantity: unpaidQty,
      });
    }
  });

  return unpaidItems;
};

/**
 * 计算商品分单的总金额
 */

/**
 * 计算金额分单的每人金额
 */

/**
 * 验证分单模式是否可用
 */
