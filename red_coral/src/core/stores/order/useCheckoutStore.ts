import { create } from 'zustand';
import { HeldOrder } from '@/core/domain/types';
import { useActiveOrdersStore } from './useActiveOrdersStore';

interface CheckoutState {
  currentOrderKey: string | number | null;
  checkoutOrder: HeldOrder | null;

  setCurrentOrderKey: (key: string | number | null) => void;
  setCheckoutOrder: (order: HeldOrder | null) => void;
  reset: () => void;
}

export const useCheckoutStore = create<CheckoutState>()((set) => ({
  currentOrderKey: null,
  checkoutOrder: null,

  setCurrentOrderKey: (key) => set({ currentOrderKey: key }),
  setCheckoutOrder: (order) => set({ checkoutOrder: order }),
  reset: () => set({ currentOrderKey: null, checkoutOrder: null }),
}));

export const useCurrentOrderKey = () => useCheckoutStore((s) => s.currentOrderKey);

export const useCheckoutOrder = () => {
  const currentOrderKey = useCheckoutStore((s) => s.currentOrderKey);
  const fallbackOrder = useCheckoutStore((s) => s.checkoutOrder);

  const orderFromStore = useActiveOrdersStore((state) => {
    if (currentOrderKey == null) return null;
    // 零售订单: currentOrderKey 是 order_id (string)，直接从 Map 查找
    if (typeof currentOrderKey === 'string') {
      const directMatch = state.orders.get(currentOrderKey);
      if (directMatch && directMatch.status === 'ACTIVE') {
        return directMatch;
      }
    }
    // 堂食订单: currentOrderKey 是 table_id (number)，按 table_id 遍历查找
    for (const order of state.orders.values()) {
      if (order.table_id === currentOrderKey && order.status === 'ACTIVE') {
        return order;
      }
    }
    return null;
  });

  return (orderFromStore as HeldOrder | null) ?? fallbackOrder;
};

// Actions export — uses getState() to avoid subscribing to store changes
export function useCheckoutActions() {
  const s = useCheckoutStore.getState();
  return {
    setCurrentOrderKey: s.setCurrentOrderKey,
    setCheckoutOrder: s.setCheckoutOrder,
    reset: s.reset,
  };
}

// Retail service type state
export type RetailServiceType = 'dineIn' | 'takeout';

const useRetailServiceTypeStore = create<{
  serviceType: RetailServiceType;
  set: (type: RetailServiceType) => void;
}>((set) => ({
  serviceType: 'dineIn',
  set: (type) => set({ serviceType: type }),
}));

export function getRetailServiceType(): RetailServiceType {
  return useRetailServiceTypeStore.getState().serviceType;
}

export function setRetailServiceType(type: RetailServiceType): void {
  useRetailServiceTypeStore.getState().set(type);
}

export function useRetailServiceType(): RetailServiceType {
  return useRetailServiceTypeStore((s) => s.serviceType);
}

/** Map frontend RetailServiceType to backend ServiceType */
export function toBackendServiceType(type: RetailServiceType): import('@/core/domain/types/orderEvent').ServiceType {
  switch (type) {
    case 'dineIn': return 'DINE_IN';
    case 'takeout': return 'TAKEOUT';
  }
}
