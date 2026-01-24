import { useCallback, useMemo } from 'react';
import { CartItem, HeldOrder, Table, Zone } from '@/core/domain/types';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { useCheckoutStore } from '@/core/stores/order/useCheckoutStore';
import { useCartStore } from '@/core/stores/cart/useCartStore';
import * as orderOps from '@/core/stores/order/useOrderOperations';
import { toast } from '@/presentation/components/Toast';

interface UseOrderHandlersParams {
  handleTableSelectStore: (
    table: Table,
    guestCount: number,
    cart: CartItem[],
    totalAmount: number,
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
      zone?: Zone
    ) => {
      const { cart, totalAmount } = useCartStore.getState();

      try {
        const result = await handleTableSelectStore(
          table,
          guestCount,
          cart,
          totalAmount,
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
      } catch (error) {
        // Handle TABLE_OCCUPIED and other errors
        const message = error instanceof Error ? error.message : '操作失败';
        if (message.includes('已被占用')) {
          toast.error(`桌台 ${table.name || table.id} 已被占用，请刷新列表`);
        } else {
          toast.error(message);
        }
        console.error('[handleTableSelect] Error:', error);
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

      // Try to retrieve existing non-retail order (dine-in checkout)
      if (key) {
        const existingSnapshot = store.getOrder(key);
        if (existingSnapshot && !existingSnapshot.is_retail) {
          setCurrentOrderKey(key);
          checkout.setCheckoutOrder(existingSnapshot);
          setViewMode('checkout');
          return;
        }
      }

      // Retail checkout: create new order with cart items
      if (cart.length === 0) return;

      try {
        // Create retail order and get order_id directly (no waiting for WebSocket)
        const orderId = await orderOps.createRetailOrder(cart);

        // Clear cart immediately after successful creation
        useCartStore.getState().clearCart();

        // Poll for the order in store (WebSocket event should arrive quickly)
        let retailOrder = store.getOrder(orderId);
        let attempts = 0;
        while (!retailOrder && attempts < 10) {
          await new Promise(resolve => setTimeout(resolve, 50));
          retailOrder = store.getOrder(orderId);
          attempts++;
        }

        if (retailOrder) {
          checkout.setCheckoutOrder(retailOrder);
          setCurrentOrderKey(orderId);
          setViewMode('checkout');
        } else {
          console.error('Retail order created but not found in store after polling');
          toast.error('订单创建成功但加载失败，请重试');
        }
      } catch (error) {
        console.error('Failed to create retail order:', error);
        toast.error('创建零售订单失败');
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
    if (checkoutOrder?.is_retail) {
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
