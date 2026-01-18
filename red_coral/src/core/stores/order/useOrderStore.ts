import { useShallow } from 'zustand/react/shallow';
import { useOrderEventStore } from './useOrderEventStore';
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

// Active Orders from Event Store
export const useHeldOrders = () => useOrderEventStore(useShallow((state) => state.getActiveOrders()));

export const useHeldOrdersCount = () => useOrderEventStore((state) =>
  state.getActiveOrders().filter(o => (o.key || String(o.tableId || '')).startsWith('RETAIL-') === false).length
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
