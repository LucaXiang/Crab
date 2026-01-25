/**
 * Order Actions Hook
 *
 * Combines actions from multiple stores for convenient use in UI components.
 * This is a facade pattern - it doesn't manage state itself.
 */

import { useShallow } from 'zustand/react/shallow';
import { useDraftOrderStore } from './useDraftOrderStore';
import { useCheckoutStore } from './useCheckoutStore';
import * as orderOps from './useOrderOperations';

/**
 * Combined actions selector for order operations
 */
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

  return {
    ...draftActions,
    ...checkoutActions,
    handleTableSelect: orderOps.handleTableSelect,
    completeOrder: orderOps.completeOrder,
    voidOrder: orderOps.voidOrder,
    partialSettle: orderOps.partialSettle,
  };
};
