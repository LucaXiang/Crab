import { useShallow } from 'zustand/react/shallow';
import { useActiveOrdersStore } from './useActiveOrdersStore';
import * as orderOps from './useOrderOperations';

// Re-export sub-stores
export {
  useDraftOrders,
  useDraftOrdersCount
} from './useDraftOrderStore';

// Import stores for combined selectors
import { useDraftOrderStore } from './useDraftOrderStore';
import { useCheckoutStore } from './useCheckoutStore';
import { useReceiptStore } from './useReceiptStore';

// --- Selectors ---

// Active Orders (uses new event-sourcing store)
export const useHeldOrders = () => {
  const snapshots = useActiveOrdersStore(useShallow((state) => state.getActiveOrders()));
  return snapshots;
};

export const useHeldOrdersCount = () => useActiveOrdersStore((state) =>
  state.getActiveOrders().filter(o => o.is_retail !== true).length
);

// Combined actions selector
export const useOrderActions = () => {
  const draftActions = useDraftOrderStore(
    useShallow((state) => ({
      saveDraft: state.saveDraft,
      restoreDraft: state.restoreDraft,
      deleteDraft: state.deleteDraft,
    }))
  );

  const checkoutActions = useCheckoutStore(
    useShallow((state) => ({
      setCheckoutOrder: state.setCheckoutOrder,
      setCurrentOrderKey: state.setCurrentOrderKey,
    }))
  );

  const receiptActions = useReceiptStore(
    useShallow((state) => ({
      generateReceiptNumber: state.generateReceiptNumber,
    }))
  );

  return {
    ...draftActions,
    ...checkoutActions,
    ...receiptActions,
    handleTableSelect: orderOps.handleTableSelect,
    completeOrder: orderOps.completeOrder,
    voidOrder: orderOps.voidOrder,
    partialSettle: orderOps.partialSettle,
  };
};
