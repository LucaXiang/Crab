import { create } from 'zustand';
import { subscribeWithSelector } from 'zustand/middleware';
import { HeldOrder, CartItem, PaymentRecord, CheckoutMode, DetailTab, PendingCashTx } from '@/core/domain/types';

function calculateUnpaidItems(items: CartItem[], paidQuantities?: Record<string, number>): CartItem[] {
  if (!paidQuantities) return items;
  return items.map(item => {
      const paidQty = paidQuantities[item.instance_id] || 0;
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

    const previousPaid = order.paid_amount; // From previous split payments
    const currentSessionPaid = state.paymentRecords.reduce((sum, p) => sum + p.amount, 0);
    const totalPaid = previousPaid + currentSessionPaid;
    // Use server-computed remaining_amount, adjusted for current session payments
    const remaining = Math.max(0, order.remaining_amount - currentSessionPaid);
    const isPaidInFull = remaining <= 0.01;
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
export const useCurrentOrderKey = () => useCheckoutStore((s) => s.currentOrderKey);

// 直接从 useActiveOrdersStore 获取订单数据（单一数据源）
import { useActiveOrdersStore } from './useActiveOrdersStore';

export const useCheckoutOrder = () => {
  // currentOrderKey 是 table ID，不是 order ID
  const currentOrderKey = useCheckoutStore((s) => s.currentOrderKey);
  const fallbackOrder = useCheckoutStore((s) => s.checkoutOrder);
  
  // 从 useActiveOrdersStore 按 table_id 查找订单
  // 当 orders Map 更新时，selector 会重新执行，组件会重新渲染
  const orderFromStore = useActiveOrdersStore((state) => {
    if (!currentOrderKey) return null;
    for (const order of state.orders.values()) {
      if (order.table_id === currentOrderKey && order.status === 'ACTIVE') {
        return order;
      }
    }
    return null;
  });
  
  // 优先使用 store 数据（单一数据源），fallback 到 checkoutOrder
  return (orderFromStore as HeldOrder | null) ?? fallbackOrder;
};

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
  };
}

// Retail service type state
export type RetailServiceType = 'dineIn' | 'takeout';

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

/** Map frontend RetailServiceType to backend ServiceType */
export function toBackendServiceType(type: RetailServiceType): import('@/core/domain/types/orderEvent').ServiceType {
  switch (type) {
    case 'dineIn': return 'DINE_IN';
    case 'takeout': return 'TAKEOUT';
  }
}
