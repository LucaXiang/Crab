import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import { HeldOrder, CartItem, PaymentRecord, CheckoutMode, DetailTab, PendingCashTx } from '@/core/domain/types';
import {
  calculateRemaining,
  isPaidInFull as checkIsPaidInFull
} from '@/utils/checkoutCalculations';

function calculateUnpaidItems(items: CartItem[], paidQuantities?: Record<string, number>): CartItem[] {
  if (!paidQuantities) return items;
  return items.map(item => {
      const key = item.instance_id || `${item.id}`;
      const paidQty = paidQuantities[key] || 0;
      const remainingQty = item.quantity - paidQty;
      if (remainingQty <= 0) return null;
      return { ...item, quantity: remainingQty };
  }).filter((i): i is CartItem => i !== null);
}

interface CheckoutState {
  // Base Data
  order: HeldOrder | null;
  currentOrderKey: string | null;
  checkoutOrder: HeldOrder | null;

  // UI State
  mode: CheckoutMode;
  activeTab: DetailTab;
  
  // Payment State
  paymentRecords: PaymentRecord[];
  unpaidItems: CartItem[];
  selectedQuantities: Record<number, number>;
  
  // Input State
  inputBuffer: string;
  isTyping: boolean;
  
  // Transaction State
  pendingCashTx: PendingCashTx | null;
  isCompleting: boolean;
  
  // Customer State
  customerCount: number;
  noteInput: string;
  recentCustomers: string[];

  // Split Bill State
  splitGuestCount: number;
  payingShares: Record<number, boolean>;
  stickySplitAmount: number | null;
  splitStrategyEvent: string | null;
  usedSplitByShares: boolean;
  usedCustomAmount: boolean;
  splitStartingRemaining: number | null;
  lastAmountSubmodeUsed: string | null;

  // Actions
  initialize: (order: HeldOrder) => void;
  setOrder: (order: HeldOrder) => void;
  setCurrentOrderKey: (key: string | null) => void;
  setCheckoutOrder: (order: HeldOrder | null) => void;
  reset: () => void;
  setMode: (mode: CheckoutMode) => void;
  setActiveTab: (tab: DetailTab) => void;
  setPaymentRecords: (updater: PaymentRecord[] | ((prev: PaymentRecord[]) => PaymentRecord[])) => void;
  setUnpaidItems: (updater: CartItem[] | ((prev: CartItem[]) => CartItem[])) => void;
  setSelectedQuantities: (updater: Record<number, number> | ((prev: Record<number, number>) => Record<number, number>)) => void;
  setInputBuffer: (updater: string | ((prev: string) => string)) => void;
  setIsTyping: (typing: boolean) => void;
  setPendingCashTx: (tx: PendingCashTx | null) => void;
  setIsCompleting: (completing: boolean) => void;
  setCustomerCount: (count: number) => void;
  setNoteInput: (note: string) => void;
  setRecentCustomers: (updater: string[] | ((prev: string[]) => string[])) => void;

  // Strategy Injection (策略模式)
  _computeStrategy?: (state: CheckoutState) => {
    previousPaid: number;
    currentSessionPaid: number;
    totalPaid: number;
    remaining: number;
    isPaidInFull: boolean;
    isPartialPaymentMade: boolean;
  };
  setComputeStrategy: (fn: CheckoutState['_computeStrategy']) => void;
  
  // Computed
  getComputed: () => {
    previousPaid: number;
    currentSessionPaid: number;
    totalPaid: number;
    remaining: number;
    isPaidInFull: boolean;
    isPartialPaymentMade: boolean;
  };
}

/**
 * CheckoutStore
 * 全局单例的结账状态管理（Zustand），实现：
 * - 单例模式：模块级创建，保证全局唯一
 * - 观察者模式：通过 subscribeWithSelector 精准订阅状态变化
 * - 策略模式：可注入可替换的计算策略（例如剩余金额计算）
 */
export const useCheckoutStore = create<CheckoutState>()(subscribeWithSelector((set, get) => ({
  // Initial State
  order: null,
  currentOrderKey: null,
  checkoutOrder: null,
  mode: 'retail',
  activeTab: 'items',
  paymentRecords: [],
  unpaidItems: [],
  selectedQuantities: {},
  inputBuffer: '',
  isTyping: false,
  pendingCashTx: null,
  isCompleting: false,
  customerCount: 1,
  noteInput: 'Customer 1',
  recentCustomers: ['Customer 1'],

  splitGuestCount: 1,
  payingShares: {},
  stickySplitAmount: null,
  splitStrategyEvent: null,
  usedSplitByShares: false,
  usedCustomAmount: false,
  splitStartingRemaining: null,
  lastAmountSubmodeUsed: null,

  // Actions
  initialize: (order) => set({
    order,
    mode: 'retail',
    activeTab: 'items',
    paymentRecords: [],
    unpaidItems: calculateUnpaidItems(order.items, order.paid_item_quantities),
    selectedQuantities: {},
    inputBuffer: '',
    isTyping: false,
    pendingCashTx: null,
    isCompleting: false,
    customerCount: 1,
    noteInput: 'Customer 1',
    recentCustomers: ['Customer 1'],
  }),

  setOrder: (order) => {
      // Recalculate unpaid items when order updates
      // Note: We do not reset session state (paymentRecords, etc.) here
      // as this might be an intermediate update.
      set({
          order,
          unpaidItems: calculateUnpaidItems(order.items, order.paid_item_quantities)
      });
  },

  setCurrentOrderKey: (key) => set({ currentOrderKey: key }),

  setCheckoutOrder: (order) => set({ checkoutOrder: order }),

  reset: () => set({
    order: null,
    currentOrderKey: null,
    checkoutOrder: null,
    mode: 'retail',
    activeTab: 'items',
    paymentRecords: [],
    unpaidItems: [],
    selectedQuantities: {},
    inputBuffer: '',
    isTyping: false,
    pendingCashTx: null,
    isCompleting: false,
    customerCount: 1,

    payingShares: {},
    stickySplitAmount: null,
    splitStrategyEvent: null,
    usedSplitByShares: false,
    usedCustomAmount: false,
    splitStartingRemaining: null,
    lastAmountSubmodeUsed: null,
  }),
  
  setMode: (mode) => set({ mode }),
  setActiveTab: (activeTab) => set({ activeTab }),
  
  setPaymentRecords: (updater) => set((state) => ({
    paymentRecords: typeof updater === 'function' ? updater(state.paymentRecords) : updater
  })),

  setUnpaidItems: (updater) => set((state) => ({
    unpaidItems: typeof updater === 'function' ? updater(state.unpaidItems) : updater
  })),

  setSelectedQuantities: (updater) => set((state) => ({
    selectedQuantities: typeof updater === 'function' ? updater(state.selectedQuantities) : updater
  })),

  setInputBuffer: (updater) => set((state) => ({
    inputBuffer: typeof updater === 'function' ? updater(state.inputBuffer) : updater
  })),

  setIsTyping: (isTyping) => set({ isTyping }),
  setPendingCashTx: (pendingCashTx) => set({ pendingCashTx }),
  setIsCompleting: (isCompleting) => set({ isCompleting }),
  setCustomerCount: (customerCount) => set({ customerCount }),
  setNoteInput: (noteInput) => set({ noteInput }),
  
  setRecentCustomers: (updater) => set((state) => ({
    recentCustomers: typeof updater === 'function' ? updater(state.recentCustomers) : updater
  })),

  // 可替换的计算策略（策略模式）
  // 允许外部设置不同的计算策略以适配业务变化
  // 默认不设置时，使用内置计算逻辑
  _computeStrategy: undefined as
    | undefined
    | ((state: CheckoutState) => {
        previousPaid: number;
        currentSessionPaid: number;
        totalPaid: number;
        remaining: number;
        isPaidInFull: boolean;
        isPartialPaymentMade: boolean;
      }),

  setComputeStrategy: (fn) => set({ _computeStrategy: fn }),

  getComputed: () => {
    const state = get();
    const order = state.order;
    if (!order) return {
        previousPaid: 0,
        currentSessionPaid: 0,
        totalPaid: 0,
        remaining: 0,
        isPaidInFull: false,
        isPartialPaymentMade: false
    };
    // 使用自定义策略（如果有）
    if (state._computeStrategy) {
      return state._computeStrategy(state);
    }

    const previousPaid = order.paid_amount || 0; // From previous split payments
    const currentSessionPaid = state.paymentRecords.reduce((sum, p) => sum + p.amount, 0);
    const totalPaid = previousPaid + currentSessionPaid;
    const remaining = calculateRemaining(order.total, totalPaid);
    const isPaidInFull = checkIsPaidInFull(order.total, totalPaid);
    const isPartialPaymentMade = totalPaid > 0 && remaining > 0.005;
    
    return {
        previousPaid,
        currentSessionPaid,
        totalPaid,
        remaining,
        isPaidInFull,
        isPartialPaymentMade
    };
  }
})));

/**
 * 观察者模式：导出精细订阅接口，减少不必要渲染
 */
export const subscribeToCheckout = <T>(
  selector: (s: CheckoutState) => T,
  listener: (value: T, prevValue: T) => void
) => useCheckoutStore.subscribe(selector, listener);

// 性能优化：导出常用的精细选择器，避免整库订阅导致的重渲染
export const useCheckoutMode = () => useCheckoutStore((s) => s.mode);
export const useCheckoutActiveTab = () => useCheckoutStore((s) => s.activeTab);
export const useCheckoutPayments = () => useCheckoutStore((s) => s.paymentRecords);
export const useCheckoutUnpaidItems = () => useCheckoutStore((s) => s.unpaidItems);
export const useCheckoutSelectedQuantities = () => useCheckoutStore((s) => s.selectedQuantities);
export const useCheckoutRemainingBuffer = () => useCheckoutStore((s) => s.inputBuffer);
export const useCheckoutIsTyping = () => useCheckoutStore((s) => s.isTyping);
export const useCheckoutPendingCash = () => useCheckoutStore((s) => s.pendingCashTx);
export const useCheckoutIsCompleting = () => useCheckoutStore((s) => s.isCompleting);
export const useCheckoutCustomerCount = () => useCheckoutStore((s) => s.customerCount);
export const useCheckoutNoteInput = () => useCheckoutStore((s) => s.noteInput);
export const useCheckoutRecentCustomers = () => useCheckoutStore((s) => s.recentCustomers);
export const useCheckoutSplitGuestCount = () => useCheckoutStore((s) => s.splitGuestCount);
export const useCheckoutPayingShares = () => useCheckoutStore((s) => s.payingShares);
export const useCheckoutStickyAmount = () => useCheckoutStore((s) => s.stickySplitAmount);
export const useCheckoutSplitStrategyEvent = () => useCheckoutStore((s) => s.splitStrategyEvent);
export const useCheckoutUsedSplitByShares = () => useCheckoutStore((s) => s.usedSplitByShares);
export const useCheckoutUsedCustomAmount = () => useCheckoutStore((s) => s.usedCustomAmount);
export const useCheckoutSplitStartingRemaining = () => useCheckoutStore((s) => s.splitStartingRemaining);
export const useCheckoutLastAmountSubmodeUsed = () => useCheckoutStore((s) => s.lastAmountSubmodeUsed);
export const useCurrentOrderKey = () => useCheckoutStore((s) => s.currentOrderKey);
export const useCheckoutOrder = () => useCheckoutStore((s) => s.checkoutOrder);

// Actions export
export function useCheckoutActions() {
  const store = useCheckoutStore();
  return {
    initialize: store.initialize,
    setOrder: store.setOrder,
    setCurrentOrderKey: store.setCurrentOrderKey,
    setCheckoutOrder: store.setCheckoutOrder,
    reset: store.reset,
    setMode: store.setMode,
    setActiveTab: store.setActiveTab,
    setPaymentRecords: store.setPaymentRecords,
    setUnpaidItems: store.setUnpaidItems,
    setSelectedQuantities: store.setSelectedQuantities,
    setInputBuffer: store.setInputBuffer,
    setIsTyping: store.setIsTyping,
    setPendingCashTx: store.setPendingCashTx,
    setIsCompleting: store.setIsCompleting,
    setCustomerCount: store.setCustomerCount,
    setNoteInput: store.setNoteInput,
    setRecentCustomers: store.setRecentCustomers,
    setComputeStrategy: store.setComputeStrategy,
  };
}

// Retail service type state
export type RetailServiceType = 'dineIn' | 'takeout' | 'delivery';

// Create a simple store for retail service type
let _retailServiceType: RetailServiceType = 'dineIn';

export function getRetailServiceType(): RetailServiceType {
  return _retailServiceType;
}

export function setRetailServiceType(type: RetailServiceType): void {
  _retailServiceType = type;
}

export function useRetailServiceType(): RetailServiceType {
  return _retailServiceType;
}
