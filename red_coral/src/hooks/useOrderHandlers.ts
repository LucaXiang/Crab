import { useCallback, useMemo } from 'react';
import { logger } from '@/utils/logger';
import { CartItem, HeldOrder, Table, Zone } from '@/core/domain/types';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { useCheckoutStore } from '@/core/stores/order/useCheckoutStore';
import { useCartStore } from '@/core/stores/cart/useCartStore';
import * as orderOps from '@/core/stores/order/commands';
import type { VoidOrderOptions } from '@/core/stores/order/commands';

import { toast } from '@/presentation/components/Toast';
import {
  savePendingRetailOrder,
  clearPendingRetailOrder,
} from '@/core/stores/order/retailOrderTracker';
import { t } from '@/infrastructure/i18n';

interface UseOrderHandlersParams {
  handleTableSelectStore: (
    table: Table,
    guestCount: number,
    cart: CartItem[],
    zone?: Zone
  ) => Promise<'MERGED' | 'CREATED' | 'RETRIEVED' | 'EMPTY'>;
  voidOrder: (orderId: string, options?: VoidOrderOptions) => Promise<void>;
  setCheckoutOrder: (order: HeldOrder | null) => void;
  setCurrentOrderKey: (key: string | number | null) => void;
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
      const { cart } = useCartStore.getState();

      try {
        const result = await handleTableSelectStore(
          table,
          guestCount,
          cart,
          zone
        );

        if (result === 'RETRIEVED') {
          setViewMode('checkout');
          setCurrentOrderKey(table.id);
        } else if (result === 'CREATED' || result === 'MERGED') {
          setCurrentOrderKey(table.id);
        }

        setShowTableScreen(false);
        if (cart.length > 0 && (result === 'MERGED' || result === 'CREATED')) {
          useCartStore.getState().clearCart();
        }
      } catch (error) {
        // Handle TABLE_OCCUPIED and other errors
        const message = error instanceof Error ? error.message : t('common.message.operation_failed');
        if (message.includes('occupied') || message.includes('7002')) {
          toast.error(t('common.message.table_occupied_refresh', { table: table.name || table.id }));
        } else {
          toast.error(message);
        }
        logger.error('Table select failed', error);
      }
    },
    [handleTableSelectStore, setViewMode, setCurrentOrderKey, setShowTableScreen]
  );

  const handleManageTable = useCallback(() => {
    setShowTableScreen(true);
  }, [setShowTableScreen]);

  const handleCheckoutStart = useCallback(
    async (key: string | number | null) => {
      const store = useActiveOrdersStore.getState();
      const checkout = useCheckoutStore.getState();
      const { cart } = useCartStore.getState();

      // Try to retrieve existing non-retail order (dine-in checkout)
      if (key && typeof key === 'string') {
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
        // service_type 在结单时设置，不在开台时传入
        const orderId = await orderOps.createRetailOrder(cart);

        // Save to local storage for recovery after crash/power outage
        savePendingRetailOrder(orderId);

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
          logger.error('Retail order created but not found in store after polling');
          toast.error(t('common.message.order_load_failed'));
        }
      } catch (error) {
        logger.error('Failed to create retail order', error);
        toast.error(t('common.message.retail_order_failed'));
      }
    },
    [setCurrentOrderKey, setViewMode]
  );

  const handleCheckoutComplete = useCallback(() => {
    // Clear pending retail order tracker (order completed successfully)
    clearPendingRetailOrder();

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
        await voidOrder(checkoutOrder.order_id, { voidType: 'CANCELLED', note: 'Retail checkout cancelled' });
        // Clear pending retail order tracker after successful void
        clearPendingRetailOrder();
      } catch (error) {
        logger.error('Failed to void retail order on cancel', error);
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

    await voidOrder(checkoutOrder.order_id, { voidType: 'CANCELLED', note: 'Manual Void' });
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
