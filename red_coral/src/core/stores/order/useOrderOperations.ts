import { HeldOrder, CartItem, PaymentRecord, Table, Zone } from '@/core/domain/types';
import { Currency } from '@/utils/currency';
import { calculateDiscountAmount, calculateItemFinalPrice } from '@/utils/pricing';
import { useOrderEventStore } from './useOrderEventStore';
import { useReceiptStore } from './useReceiptStore';
import { useCheckoutStore } from './useCheckoutStore';

// ============================================================================
// Helper Functions (Pure Logic)
// ============================================================================

const applySurchargeToItems = (
  items: CartItem[],
  surchargePercentage: number,
  surchargePerItem: number
): CartItem[] => {
  return items.map(item => {
    const originalPrice = item.originalPrice ?? 0;
    const discountAmount = calculateDiscountAmount(originalPrice, item.discountPercent || 0);
    const discountedBase = Currency.sub(originalPrice, discountAmount);

    let itemSurchargeAmount = Currency.toDecimal(0);
    if (surchargePercentage > 0) {
      itemSurchargeAmount = Currency.floor2((discountedBase.toNumber() * surchargePercentage) / 100);
    } else if (surchargePerItem > 0) {
      itemSurchargeAmount = Currency.floor2(surchargePerItem);
    }

    const itemForCalc = { ...item, surcharge: itemSurchargeAmount.toNumber() };
    const finalItemPrice = calculateItemFinalPrice(itemForCalc);

    return {
      ...item,
      price: finalItemPrice.toNumber(),
      surcharge: itemSurchargeAmount.toNumber()
    };
  });
};

const calculateSurchargeInfo = (existingOrder: HeldOrder | undefined, zone: Zone | undefined) => {
  let surchargePerItem = 0;
  let surchargePercentage = 0;
  let orderSurchargeInfo = existingOrder?.surcharge;

  if (existingOrder?.surcharge) {
    if (existingOrder.surcharge.type === 'percentage') {
      surchargePercentage = existingOrder.surcharge.value || 0;
    } else {
      surchargePerItem = existingOrder.surcharge.value || 0;
    }
  } else if (zone?.surcharge_type && zone?.surcharge_amount) {
    if (zone.surcharge_type === 'percentage') {
      surchargePercentage = zone.surcharge_amount;
      orderSurchargeInfo = {
        type: 'percentage',
        amount: 0,
        total: 0,
        value: surchargePercentage,
        name: zone.name || undefined
      };
    } else {
      surchargePerItem = zone.surcharge_amount;
      orderSurchargeInfo = {
        type: 'fixed',
        amount: 0,
        total: 0,
        value: surchargePerItem,
        name: zone.name || undefined
      };
    }
  }

  return { surchargePerItem, surchargePercentage, orderSurchargeInfo };
};

const prepareItemsWithSurcharge = (
  cart: CartItem[],
  surchargePercentage: number,
  surchargePerItem: number
) => {
  let finalCart = [...cart];
  


  return applySurchargeToItems(finalCart, surchargePercentage, surchargePerItem);
};

// ============================================================================
// Internal Action Handlers
// ============================================================================

const handleMergeToOrder = (
  orderKey: string,
  existingOrder: HeldOrder,
  cart: CartItem[],
  surchargePercentage: number,
  surchargePerItem: number,
  enableIndividualMode?: boolean
) => {
  const store = useOrderEventStore.getState();

  // Ensure receipt number exists
  if (!existingOrder.receiptNumber || !existingOrder.receiptNumber.startsWith('FAC')) {
    const newReceiptNumber = useReceiptStore.getState().generateReceiptNumber();
    store.updateOrderInfo(orderKey, { receiptNumber: newReceiptNumber });
  }

  const itemsToAdd = prepareItemsWithSurcharge(
    cart, 
    surchargePercentage, 
    surchargePerItem
  );

  store.addItems(orderKey, itemsToAdd);
  return 'MERGED' as const;
};

const handleCreateNewOrder = (
  orderKey: string,
  table: Table,
  guestCount: number,
  zone: Zone | undefined,
  cart: CartItem[],
  surchargeInfo: { surchargePercentage: number, surchargePerItem: number, orderSurchargeInfo: any },
  enableIndividualMode?: boolean
) => {
  const store = useOrderEventStore.getState();
  const receiptNumber = useReceiptStore.getState().generateReceiptNumber();

  const itemsToAdd = prepareItemsWithSurcharge(
    cart,
    surchargeInfo.surchargePercentage,
    surchargeInfo.surchargePerItem
  );

  store.openTable({
    tableId: String(orderKey),
    tableName: table.name,
    guestCount: guestCount,
    zoneId: zone?.id ? String(zone.id) : undefined,
    zoneName: zone?.name,
    surcharge: surchargeInfo.orderSurchargeInfo,
    receiptNumber: receiptNumber
  });

  store.addItems(String(orderKey), itemsToAdd);
  return 'CREATED' as const;
};

// Ensures an order exists in the store (Lazy Creation)
export const ensureActiveOrder = (order: HeldOrder) => {
  const store = useOrderEventStore.getState();
  const orderKey = order.key || String(order.tableId || '');
  const existing = store.getOrder(orderKey);

  if (!existing || (existing.items.length === 0 && order.items.length > 0)) {
    store.openTable({
      tableId: orderKey,
      tableName: order.tableName || '',
      guestCount: order.guestCount || 1,
      zoneName: order.zoneName || undefined,
      surcharge: order.surcharge,
      receiptNumber: order.receiptNumber // Pass through if available
    });
    if (order.items && order.items.length > 0) {
      store.addItems(orderKey, order.items);
    }
    return true; // Created
  }
  return false; // Existed
};

// ============================================================================
// Exported Operations
// ============================================================================

export const handleTableSelect = (
  table: Table,
  guestCount: number,
  cart: CartItem[],
  totalAmount: number,
  enableIndividualMode?: boolean,
  isIndividualMode?: boolean,
  zone?: Zone
): 'MERGED' | 'CREATED' | 'RETRIEVED' | 'EMPTY' => {
  const orderKey = String(table.id);
  const store = useOrderEventStore.getState();
  const checkoutStore = useCheckoutStore.getState();

  const existingOrder = store.getOrder(orderKey);

  // 1. If cart has items, we are ADDING (Merge) or CREATING
  if (cart.length > 0) {
    const surchargeInfo = calculateSurchargeInfo(existingOrder, zone);

    if (existingOrder && existingOrder.status === 'ACTIVE') {
      return handleMergeToOrder(
        orderKey, 
        existingOrder, 
        cart, 
        surchargeInfo.surchargePercentage, 
        surchargeInfo.surchargePerItem, 
        enableIndividualMode
      );
    } else {
      return handleCreateNewOrder(
        orderKey,
        table,
        guestCount,
        zone,
        cart,
        surchargeInfo,
        enableIndividualMode || isIndividualMode
      );
    }
  } 
  
  // 2. RETRIEVE Logic (No items in cart)
  if (existingOrder) {
    // Ensure receipt number consistency on retrieve
    if (!existingOrder.receiptNumber || !existingOrder.receiptNumber.startsWith('FAC')) {
      const newReceiptNumber = useReceiptStore.getState().generateReceiptNumber();
      store.updateOrderInfo(orderKey, { receiptNumber: newReceiptNumber });
      // We don't need to re-fetch immediately for UI update as store subscription handles it,
      // but for setCheckoutOrder we might want the latest.
      const updated = store.getOrder(orderKey);
      checkoutStore.setCheckoutOrder(updated || existingOrder);
    } else {
      checkoutStore.setCheckoutOrder(existingOrder);
    }
    return 'RETRIEVED';
  }

  return 'EMPTY';
};

export const completeOrder = (
  order: HeldOrder,
  newPayments: PaymentRecord[],
): HeldOrder => {
  const store = useOrderEventStore.getState();
  const receiptStore = useReceiptStore.getState();

  const orderKey = order.key || String(order.tableId || '');

  // 1. Ensure Receipt Number
  let finalReceiptNumber = order.receiptNumber;
  const existing = store.getOrder(orderKey);

  // Prefer existing valid receipt number
  if ((!finalReceiptNumber || !finalReceiptNumber.startsWith('FAC')) && existing?.receiptNumber?.startsWith('FAC')) {
      finalReceiptNumber = existing.receiptNumber;
  }
  // Generate if missing
  if (!finalReceiptNumber || !finalReceiptNumber.startsWith('FAC')) {
      finalReceiptNumber = receiptStore.generateReceiptNumber();
  }

  // Assign final receipt number to the order object for creation
  const orderWithReceipt = { ...order, receiptNumber: finalReceiptNumber };

  // 2. Ensure Order Exists (Lazy Create)
  ensureActiveOrder(orderWithReceipt);

  // 3. Add Payments
  newPayments.forEach(payment => {
    store.addPayment(orderKey, payment);
  });

  // 4. Complete Order
  store.completeOrder(orderKey, finalReceiptNumber);

  // 5. Cleanup
  import('@/core/stores/order/usePaymentStore').then(({ usePaymentStore }) => {
      usePaymentStore.getState().clearSession(orderKey);
  });

  // 6. Retail kitchen ticket after checkout
  try {
    const updated = store.getOrder(orderKey) || orderWithReceipt;
    const isRetail = orderKey.startsWith('RETAIL-') || (updated as any).isRetail === true;
    if (isRetail) {
      import('@/services/printService').then(({ printKitchenTicketLegacy }) => {
        printKitchenTicketLegacy(updated, false, false, 'retail', updated.items || []).catch(() => {});
      });
    }
  } catch {}

  return store.getOrder(orderKey) || order;
};

export const voidOrder = (
  order: HeldOrder,
  reason?: string
): HeldOrder => {
  const store = useOrderEventStore.getState();
  const orderKey = order.key || String(order.tableId || '');

  store.voidOrder(orderKey, reason);

  // Cleanup
  import('@/core/stores/order/usePaymentStore').then(({ usePaymentStore }) => {
      usePaymentStore.getState().clearSession(orderKey);
  });

  return store.getOrder(orderKey) || order;
};

export const partialSettle = (
  order: HeldOrder,
  newPayments: PaymentRecord[],
): HeldOrder => {
  const store = useOrderEventStore.getState();
  const checkoutStore = useCheckoutStore.getState();

  const orderKey = order.key || String(order.tableId || '');

  // 1. Ensure Order Exists
  ensureActiveOrder(order);

  // 2. Add Payments



  // 3. Sync Checkout Store
  const updatedOrder = store.getOrder(orderKey);
  if (checkoutStore.checkoutOrder?.key === orderKey && updatedOrder) {
    checkoutStore.setCheckoutOrder(updatedOrder);
  }

  return updatedOrder || order;
};
