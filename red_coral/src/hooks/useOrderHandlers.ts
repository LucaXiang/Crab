import { useCallback, useMemo } from 'react';
import { CartItem, HeldOrder, Table, Zone } from '@/core/domain/types';
import { useOrderEventStore } from '@/core/stores/order/useOrderEventStore';
import { useCheckoutStore } from '@/core/stores/order/useCheckoutStore';
import { useCartStore } from '@/core/stores/cart/useCartStore';

interface UseOrderHandlersParams {
  handleTableSelectStore: (
    table: Table,
    guestCount: number,
    cart: CartItem[],
    totalAmount: number,
    enableIndividualMode?: boolean,
    isIndividualMode?: boolean,
    zone?: Zone
  ) => 'MERGED' | 'CREATED' | 'RETRIEVED' | 'EMPTY';
  voidOrder: (order: HeldOrder, reason?: string) => HeldOrder;
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
    (
      table: Table,
      guestCount: number,
      enableIndividualMode?: boolean,
      zone?: Zone
    ) => {
      const { cart, totalAmount } = useCartStore.getState();

      const result = handleTableSelectStore(
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
    (key: string | null) => {
      const store = useOrderEventStore.getState();
      const checkout = useCheckoutStore.getState();
      const { cart } = useCartStore.getState();

      if (key && !key.startsWith('RETAIL-')) {
        setCurrentOrderKey(key);
        const existing = store.getOrder(key);
        if (existing) {
          checkout.setCheckoutOrder(existing);
          setViewMode('checkout');
          return;
        }
      }

      if (cart.length === 0) return;

      if (!key || key.startsWith('RETAIL-')) {
        checkout.setCurrentOrderKey(null);
      }

      const retailKey = `RETAIL-${Date.now()}`;

      store.openTable({
        tableId: retailKey,
        tableName: 'Inmediata',
        guestCount: 1,
      });

      store.addItems(retailKey, cart);

      const createdOrder = store.getOrder(retailKey);

      if (createdOrder) {
        const retailOrder = {
          ...createdOrder,
          isRetail: true,
          tableName: 'Inmediata',
        };

        checkout.setCheckoutOrder(retailOrder);
        setCurrentOrderKey(retailKey);
        setViewMode('checkout');
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

  const handleCheckoutCancel = useCallback(() => {
    setViewMode('pos');
    setCheckoutOrder(null);
    setCurrentOrderKey(null);
  }, [setViewMode, setCheckoutOrder, setCurrentOrderKey]);

  const handleCheckoutVoid = useCallback(() => {
    const { checkoutOrder } = useCheckoutStore.getState();
    if (!checkoutOrder) return;
    voidOrder(checkoutOrder, 'Manual Void');
    setViewMode('pos');
    setCheckoutOrder(null);
    useCartStore.getState().clearCart();
    setCurrentOrderKey(null);
  }, [voidOrder, setViewMode, setCheckoutOrder, setCurrentOrderKey]);

  /**
   * Helper to void any potentially abandoned retail order that has a Receipt Number
   * This should be called when "Clear Cart" is clicked in Sidebar
   */
  

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
