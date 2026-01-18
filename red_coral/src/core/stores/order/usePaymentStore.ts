/**
 * usePaymentStore - 简化的支付状态管理
 *
 * 职责：
 * 1. 管理支付记录（现金、刷卡）
 * 2. 支付记录的挂起/恢复
 * 3. 计算已支付金额和剩余金额
 */

import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { PaymentRecord } from '@/core/domain/types';
import { Currency } from '@/utils/currency';


interface PendingCashPayment {
  amount: number;
  note?: string;
  timestamp: number;
}

interface PaymentSession {
  orderKey: string;
  paymentRecords: PaymentRecord[];
  pendingCashPayment: PendingCashPayment | null;
}

interface PaymentStore {
  // 当前支付会话（按订单 key 存储）
  sessions: Record<string, PaymentSession>;

  // Actions
  initSession: (orderKey: string) => void;
  clearSession: (orderKey: string) => void;

  // 支付记录管理
  addPayment: (orderKey: string, payment: Omit<PaymentRecord, 'id' | 'timestamp'>) => PaymentRecord;
  removePayment: (orderKey: string, paymentId: string) => void;

  // 现金支付流程
  setPendingCashPayment: (orderKey: string, payment: PendingCashPayment | null) => void;
  confirmCashPayment: (orderKey: string, tenderedAmount: number) => PaymentRecord;

  // 计算方法
  getTotalPaid: (orderKey: string) => number;
  getRemaining: (orderKey: string, orderTotal: number) => number;
  isPaidInFull: (orderKey: string, orderTotal: number) => boolean;
}

export const usePaymentStore = create<PaymentStore>((set, get) => ({
  sessions: {},

  initSession: (orderKey) => {
    set((state) => ({
      sessions: {
        ...state.sessions,
        [orderKey]: {
          orderKey,
          paymentRecords: [],
          pendingCashPayment: null,
        }
      }
    }));
  },

  clearSession: (orderKey) => {
    set((state) => {
      const newSessions = { ...state.sessions };
      delete newSessions[orderKey];
      return { sessions: newSessions };
    });
  },

  addPayment: (orderKey, payment) => {
    const timestamp = Date.now();
    const newPayment: PaymentRecord = {
      ...payment,
      id: `pay-${timestamp}-${Math.random().toString(36).substr(2, 9)}`,
      timestamp,
    };

    set((state) => {
      const session = state.sessions[orderKey];
      if (!session) return state;

      return {
        sessions: {
          ...state.sessions,
          [orderKey]: {
            ...session,
            paymentRecords: [...session.paymentRecords, newPayment],
          }
        }
      };
    });

    return newPayment;
  },

  removePayment: (orderKey, paymentId) => {
    set((state) => {
      const session = state.sessions[orderKey];
      if (!session) return state;

      return {
        sessions: {
          ...state.sessions,
          [orderKey]: {
            ...session,
            paymentRecords: session.paymentRecords.filter(p => p.id !== paymentId),
          }
        }
      };
    });
  },

  setPendingCashPayment: (orderKey, payment) => {
    set((state) => {
      const session = state.sessions[orderKey];
      if (!session) return state;

      return {
        sessions: {
          ...state.sessions,
          [orderKey]: {
            ...session,
            pendingCashPayment: payment,
          }
        }
      };
    });
  },

  confirmCashPayment: (orderKey, tenderedAmount) => {
    const session = get().sessions[orderKey];
    if (!session?.pendingCashPayment) {
      throw new Error('No pending cash payment');
    }

    const { amount, note } = session.pendingCashPayment;
    const change = Currency.sub(tenderedAmount, amount);

    const payment = get().addPayment(orderKey, {
      method: 'Cash',
      amount,
      tendered: tenderedAmount,
      change: change.toNumber(),
      note: note,
    });

    // Clear pending payment
    get().setPendingCashPayment(orderKey, null);

    return payment;
  },

  getTotalPaid: (orderKey) => {
    const session = get().sessions[orderKey];
    if (!session) return 0;

    return session.paymentRecords.reduce((sum, p) => Currency.add(sum, p.amount).toNumber(), 0);
  },

  getRemaining: (orderKey, orderTotal) => {
    const totalPaid = get().getTotalPaid(orderKey);
    return Currency.max(0, Currency.sub(orderTotal, totalPaid)).toNumber();
  },

  isPaidInFull: (orderKey, orderTotal) => {
    const remaining = get().getRemaining(orderKey, orderTotal);
    return Currency.lte(remaining, 0.01); // 允许 1 分钱的误差
  },
}));

// ============ Selectors ============

export const usePaymentSession = (orderKey: string) => {
  return usePaymentStore((state) => state.sessions[orderKey]);
};

 

export const usePaymentTotals = (orderKey: string, orderTotal: number) => {
  return usePaymentStore(
    useShallow((state) => {
      const totalPaid = state.getTotalPaid(orderKey);
      const remaining = state.getRemaining(orderKey, orderTotal);
      const isPaidInFull = state.isPaidInFull(orderKey, orderTotal);

      return { totalPaid, remaining, isPaidInFull };
    })
  );
};

export const usePaymentActions = () => {
  return usePaymentStore(
    useShallow((state) => ({
      initSession: state.initSession,
      clearSession: state.clearSession,
      addPayment: state.addPayment,
      removePayment: state.removePayment,
      setPendingCashPayment: state.setPendingCashPayment,
      confirmCashPayment: state.confirmCashPayment,
    }))
  );
};
