import { useCallback, useMemo } from 'react';
import { CartItem, HeldOrder, Table, Zone } from '@/core/domain/types';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { useCheckoutStore } from '@/core/stores/order/useCheckoutStore';
import { useCartStore } from '@/core/stores/cart/useCartStore';
import { toHeldOrder } from '@/core/stores/order/orderAdapter';
import * as orderOps from '@/core/stores/order/useOrderOperations';

interface UseOrderHandlersParams {
  handleTableSelectStore: (
    table: Table,
    guestCount: number,
    cart: CartItem[],
    totalAmount: number,
    enableIndividualMode?: boolean,
    isIndividualMode?: boolean,
    zone?: Zone
  ) => Promise<'MERGED' | 'CREATED' | 'RETRIEVED' | 'EMPTY'>;
  voidOrder: (order: HeldOrder, reason?: string) => Promise<HeldOrder>;
  setCheckoutOrder: (order: HeldOrder | null) => void;
  setCurrentOrderKey: (key: string | null) => void;
  setViewMode: (mode: 'pos' | 'checkout') => void;
  setShowTableScreen: (v: boolean) => void;
}

export function useOrderHandlers(params: UseOrderHandlersParams) {
  const {
    handleTableSelectStore,
    voidOrder,
    setCheckoutOrder,
    setCurrentOrderKey,
    setViewMode,
    setShowTableScreen,
  } = params;

  const handleTableSelect = useCallback(
    async (
      table: Table,
      guestCount: number,
      enableIndividualMode?: boolean,
      zone?: Zone
    ) => {
      const { cart, totalAmount } = useCartStore.getState();

      const result = await handleTableSelectStore(
        table,
        guestCount,
        cart,
        totalAmount,
        enableIndividualMode,
        undefined,
        zone
      );

      if (result === 'RETRIEVED') {
        setViewMode('checkout');
        setCurrentOrderKey(String(table.id));
      } else if (result === 'CREATED' || result === 'MERGED') {
        setCurrentOrderKey(String(table.id));
      }

      setShowTableScreen(false);
      if (cart.length > 0 && (result === 'MERGED' || result === 'CREATED')) {
        useCartStore.getState().clearCart();
      }
    },
    [handleTableSelectStore, setViewMode, setCurrentOrderKey, setShowTableScreen]
  );

  const handleManageTable = useCallback(() => {
    setShowTableScreen(true);
  }, [setShowTableScreen]);

  const handleCheckoutStart = useCallback(
    async (key: string | null) => {
      const store = useActiveOrdersStore.getState();
      const checkout = useCheckoutStore.getState();
      const { cart } = useCartStore.getState();

      // Try to retrieve existing non-retail order
      if (key) {
        const existingSnapshot = store.getOrder(key);
        if (existingSnapshot && !existingSnapshot.is_retail) {
          setCurrentOrderKey(key);
          checkout.setCheckoutOrder(toHeldOrder(existingSnapshot));
          setViewMode('checkout');
          return;
        }
      }

      if (cart.length === 0) return;

      // Clear key for retail orders (new order will be created)
      checkout.setCurrentOrderKey(null);

      // Create retail order via command
      const retailTable: Table = {
        id: null, // Will be assigned by backend for retail
        name: 'Inmediata',
        capacity: 1,
        zone: '',
        is_active: true,
      };

      try {
        const result = await orderOps.handleTableSelect(
          retailTable,
          1,
          cart,
          0,
          false,
          false,
          undefined
        );

        if (result === 'CREATED' || result === 'MERGED') {
          // Wait a bit for event to arrive and update store
          await new Promise(resolve => setTimeout(resolve, 100));

          // Find the created retail order
          const activeOrders = store.getActiveOrders();
          const retailOrder = activeOrders.find(o => o.is_retail === true);

          if (retailOrder) {
            const heldOrder = toHeldOrder(retailOrder);
            checkout.setCheckoutOrder(heldOrder);
            setCurrentOrderKey(heldOrder.key);
            setViewMode('checkout');
          }
        }
      } catch (error) {
        console.error('Failed to create retail order:', error);
      }
    },
    [setCurrentOrderKey, setViewMode]
  );

  const handleCheckoutComplete = useCallback(() => {
    useCartStore.getState().clearCart();

    setViewMode('pos');
    setCheckoutOrder(null);
    setCurrentOrderKey(null);
  }, [setViewMode, setCheckoutOrder, setCurrentOrderKey]);

  const handleCheckoutCancel = useCallback(async () => {
    const { checkoutOrder } = useCheckoutStore.getState();

    // For retail orders, void them when cancelled (audit requirement)
    // Retail orders should not remain active when user exits checkout
    if (checkoutOrder?.isRetail) {
      try {
        await voidOrder(checkoutOrder, 'Retail checkout cancelled');
      } catch (error) {
        console.error('Failed to void retail order on cancel:', error);
        // Continue with UI cleanup even if void fails
      }
    }

    setViewMode('pos');
    setCheckoutOrder(null);
    setCurrentOrderKey(null);
  }, [voidOrder, setViewMode, setCheckoutOrder, setCurrentOrderKey]);

  const handleCheckoutVoid = useCallback(async () => {
    const { checkoutOrder } = useCheckoutStore.getState();
    if (!checkoutOrder) return;

    await voidOrder(checkoutOrder, 'Manual Void');
    setViewMode('pos');
    setCheckoutOrder(null);
    useCartStore.getState().clearCart();
    setCurrentOrderKey(null);
  }, [voidOrder, setViewMode, setCheckoutOrder, setCurrentOrderKey]);

  return useMemo(
    () => ({
      handleTableSelect,
      handleManageTable,
      handleCheckoutStart,
      handleCheckoutComplete,
      handleCheckoutCancel,
      handleCheckoutVoid,
    }),
    [
      handleTableSelect,
      handleManageTable,
      handleCheckoutStart,
      handleCheckoutComplete,
      handleCheckoutCancel,
      handleCheckoutVoid,
    ]
  );
}
