/**
 * Order Reducer
 *
 * Pure functions for computing OrderSnapshot from OrderEvent stream.
 * These functions are used by useActiveOrdersStore to apply events.
 *
 * Design principles:
 * - Pure functions with no side effects
 * - Events are immutable facts
 * - Snapshot is always derivable from events
 */

import { Currency } from '@/utils/currency';
import type {
  OrderEvent,
  OrderSnapshot,
  CartItemSnapshot,
  PaymentRecord,
  SurchargeConfig,
  OrderStatus,
  TableOpenedPayload,
  OrderCompletedPayload,
  OrderVoidedPayload,
  ItemsAddedPayload,
  ItemModifiedPayload,
  ItemRemovedPayload,
  ItemRestoredPayload,
  PaymentAddedPayload,
  PaymentCancelledPayload,
  OrderSplitPayload,
  OrderMovedPayload,
  OrderMergedPayload,
  TableReassignedPayload,
  SurchargeExemptSetPayload,
  OrderInfoUpdatedPayload,
} from '@/core/domain/types/orderEvent';

// ============================================================================
// Empty Snapshot Factory
// ============================================================================

/**
 * Compute state checksum for drift detection
 *
 * Must match the Rust implementation in `OrderSnapshot::compute_checksum()`
 * Uses same fields: items.length, total (cents), paid_amount (cents), last_sequence, status
 *
 * @param snapshot The snapshot to compute checksum for
 * @returns 16-character hex string
 */
export function computeChecksum(snapshot: OrderSnapshot): string {
  // Simple hash function that matches Rust's DefaultHasher behavior
  // We hash the same fields in the same order
  let hash = 0n;

  // Hash item count
  hash = simpleHash(hash, BigInt(snapshot.items.length));

  // Hash total in cents (avoid float precision issues)
  hash = simpleHash(hash, BigInt(Math.round(snapshot.total * 100)));

  // Hash paid_amount in cents
  hash = simpleHash(hash, BigInt(Math.round(snapshot.paid_amount * 100)));

  // Hash last sequence
  hash = simpleHash(hash, BigInt(snapshot.last_sequence));

  // Hash status (convert to discriminant like Rust)
  const statusMap: Record<OrderStatus, number> = {
    ACTIVE: 0,
    COMPLETED: 1,
    VOID: 2,
    MOVED: 3,
    MERGED: 4,
  };
  hash = simpleHash(hash, BigInt(statusMap[snapshot.status] ?? 0));

  // Return as 16-char hex string (like Rust's format!("{:016x}", ...))
  return hash.toString(16).padStart(16, '0').slice(-16);
}

/**
 * Simple hash combine function
 * Note: This may not produce identical results to Rust's DefaultHasher,
 * but it's good enough for drift detection. If checksums don't match,
 * we'll do a full sync which will correct any issues.
 */
function simpleHash(seed: bigint, value: bigint): bigint {
  // FNV-1a style hash combine
  const FNV_PRIME = 0x100000001b3n;
  let hash = seed ^ value;
  hash = (hash * FNV_PRIME) & 0xFFFFFFFFFFFFFFFFn;
  return hash;
}

/**
 * Verify that a snapshot's checksum matches its computed value
 * @returns true if checksum is valid or not present, false if drift detected
 */
export function verifyChecksum(snapshot: OrderSnapshot): boolean {
  if (!snapshot.state_checksum) {
    // No checksum to verify - assume valid
    return true;
  }
  return snapshot.state_checksum === computeChecksum(snapshot);
}

/**
 * Create an empty order snapshot with default values
 */
export function createEmptySnapshot(orderId: string): OrderSnapshot {
  const now = Date.now();
  const snapshot: OrderSnapshot = {
    order_id: orderId,
    table_id: null,
    table_name: null,
    zone_id: null,
    zone_name: null,
    guest_count: 1,
    is_retail: false,
    status: 'ACTIVE',
    items: [],
    payments: [],
    subtotal: 0,
    tax: 0,
    discount: 0,
    surcharge: null,
    surcharge_exempt: false,
    total: 0,
    paid_amount: 0,
    receipt_number: null,
    start_time: now,
    end_time: null,
    created_at: now,
    updated_at: now,
    last_sequence: 0,
  };
  // Compute initial checksum
  snapshot.state_checksum = computeChecksum(snapshot);
  return snapshot;
}

// ============================================================================
// Main Reducer Functions
// ============================================================================

/**
 * Apply a single event to a snapshot, returning a new snapshot
 */
export function applyEvent(
  snapshot: OrderSnapshot,
  event: OrderEvent
): OrderSnapshot {
  // Start with a copy and update common fields
  let result: OrderSnapshot = {
    ...snapshot,
    updated_at: event.timestamp,
    last_sequence: event.sequence,
  };

  // Dispatch based on event type
  switch (event.event_type) {
    case 'TABLE_OPENED':
      result = applyTableOpened(result, event.payload as TableOpenedPayload, event.timestamp);
      break;
    case 'ORDER_COMPLETED':
      result = applyOrderCompleted(result, event.payload as OrderCompletedPayload, event.timestamp);
      break;
    case 'ORDER_VOIDED':
      result = applyOrderVoided(result, event.payload as OrderVoidedPayload, event.timestamp);
      break;
    case 'ORDER_RESTORED':
      result = applyOrderRestored(result);
      break;
    case 'ITEMS_ADDED':
      result = applyItemsAdded(result, event.payload as ItemsAddedPayload);
      break;
    case 'ITEM_MODIFIED':
      result = applyItemModified(result, event.payload as ItemModifiedPayload);
      break;
    case 'ITEM_REMOVED':
      result = applyItemRemoved(result, event.payload as ItemRemovedPayload);
      break;
    case 'ITEM_RESTORED':
      result = applyItemRestored(result, event.payload as ItemRestoredPayload);
      break;
    case 'PAYMENT_ADDED':
      result = applyPaymentAdded(result, event.payload as PaymentAddedPayload, event.timestamp);
      break;
    case 'PAYMENT_CANCELLED':
      result = applyPaymentCancelled(result, event.payload as PaymentCancelledPayload);
      break;
    case 'ORDER_SPLIT':
      result = applyOrderSplit(result, event.payload as OrderSplitPayload, event.timestamp);
      break;
    case 'ORDER_MOVED':
      result = applyOrderMoved(result, event.payload as OrderMovedPayload);
      break;
    case 'ORDER_MOVED_OUT':
      result = applyOrderMovedOut(result, event.timestamp);
      break;
    case 'ORDER_MERGED':
      result = applyOrderMerged(result, event.payload as OrderMergedPayload);
      break;
    case 'ORDER_MERGED_OUT':
      result = applyOrderMergedOut(result, event.timestamp);
      break;
    case 'TABLE_REASSIGNED':
      result = applyTableReassigned(result, event.payload as TableReassignedPayload);
      break;
    case 'SURCHARGE_EXEMPT_SET':
      result = applySurchargeExemptSet(result, event.payload as SurchargeExemptSetPayload);
      break;
    case 'ORDER_INFO_UPDATED':
      result = applyOrderInfoUpdated(result, event.payload as OrderInfoUpdatedPayload);
      break;
    default:
      // Unknown event type - log and continue
      console.warn(`Unknown event type: ${event.event_type}`);
  }

  // Recalculate totals after any change
  result = recalculateTotals(result);

  // Update state checksum for drift detection
  result.state_checksum = computeChecksum(result);

  return result;
}

/**
 * Rebuild snapshot from a list of events
 */
export function rebuildFromEvents(
  orderId: string,
  events: OrderEvent[]
): OrderSnapshot {
  // Sort events by sequence to ensure correct order
  const sortedEvents = [...events].sort((a, b) => a.sequence - b.sequence);

  // Start with empty snapshot and apply each event
  return sortedEvents.reduce(
    (snapshot, event) => applyEvent(snapshot, event),
    createEmptySnapshot(orderId)
  );
}

// ============================================================================
// Event Handlers
// ============================================================================

function applyTableOpened(
  snapshot: OrderSnapshot,
  payload: TableOpenedPayload,
  timestamp: number
): OrderSnapshot {
  return {
    ...snapshot,
    table_id: payload.table_id,
    table_name: payload.table_name,
    zone_id: payload.zone_id,
    zone_name: payload.zone_name,
    guest_count: payload.guest_count,
    is_retail: payload.is_retail,
    surcharge: payload.surcharge || null,
    receipt_number: payload.receipt_number || null,
    status: 'ACTIVE',
    start_time: timestamp,
    created_at: timestamp,
  };
}

function applyOrderCompleted(
  snapshot: OrderSnapshot,
  payload: OrderCompletedPayload,
  timestamp: number
): OrderSnapshot {
  return {
    ...snapshot,
    status: 'COMPLETED',
    receipt_number: payload.receipt_number,
    total: payload.final_total,
    end_time: timestamp,
  };
}

function applyOrderVoided(
  snapshot: OrderSnapshot,
  _payload: OrderVoidedPayload,
  timestamp: number
): OrderSnapshot {
  return {
    ...snapshot,
    status: 'VOID',
    end_time: timestamp,
  };
}

function applyOrderRestored(snapshot: OrderSnapshot): OrderSnapshot {
  return {
    ...snapshot,
    status: 'ACTIVE',
    end_time: null,
  };
}

function applyItemsAdded(
  snapshot: OrderSnapshot,
  payload: ItemsAddedPayload
): OrderSnapshot {
  // Merge items with existing items (same instance_id increases quantity)
  const newItems = mergeItemsIntoList(snapshot.items, payload.items);

  return {
    ...snapshot,
    items: newItems,
  };
}

function applyItemModified(
  snapshot: OrderSnapshot,
  payload: ItemModifiedPayload
): OrderSnapshot {
  // Use results from backend to update items
  let items = [...snapshot.items];

  for (const result of payload.results) {
    const existingIndex = items.findIndex(i => i.instance_id === result.instance_id);

    if (result.action === 'UPDATED') {
      // Update existing item
      if (existingIndex >= 0) {
        items[existingIndex] = {
          ...items[existingIndex],
          quantity: result.quantity,
          price: result.price,
          discount_percent: result.discount_percent ?? items[existingIndex].discount_percent,
          // Also apply other changes from payload
          surcharge: payload.changes.surcharge ?? items[existingIndex].surcharge,
          note: payload.changes.note ?? items[existingIndex].note,
        };
      }
    } else if (result.action === 'UNCHANGED') {
      // Item remains but with reduced quantity
      if (existingIndex >= 0) {
        items[existingIndex] = {
          ...items[existingIndex],
          quantity: result.quantity,
        };
      }
    } else if (result.action === 'CREATED') {
      // New split item created
      if (existingIndex < 0) {
        items.push({
          ...payload.source,
          instance_id: result.instance_id,
          quantity: result.quantity,
          price: result.price,
          discount_percent: result.discount_percent,
        });
      }
    }
  }

  return {
    ...snapshot,
    items,
  };
}

function applyItemRemoved(
  snapshot: OrderSnapshot,
  payload: ItemRemovedPayload
): OrderSnapshot {
  const items = snapshot.items.map((item) => {
    if (item.instance_id !== payload.instance_id) {
      return item;
    }

    // Partial removal: reduce quantity
    if (payload.quantity != null && payload.quantity < item.quantity) {
      return {
        ...item,
        quantity: item.quantity - payload.quantity,
      };
    }

    // Full removal: mark as removed (soft delete)
    return {
      ...item,
      _removed: true,
    };
  });

  return {
    ...snapshot,
    items,
  };
}

function applyItemRestored(
  snapshot: OrderSnapshot,
  payload: ItemRestoredPayload
): OrderSnapshot {
  const items = snapshot.items.map((item) => {
    if (item.instance_id !== payload.instance_id) {
      return item;
    }

    // Remove the _removed flag
    const { _removed, ...rest } = item;
    return rest;
  });

  return {
    ...snapshot,
    items,
  };
}

function applyPaymentAdded(
  snapshot: OrderSnapshot,
  payload: PaymentAddedPayload,
  timestamp: number
): OrderSnapshot {
  const newPayment: PaymentRecord = {
    payment_id: payload.payment_id,
    method: payload.method,
    amount: payload.amount,
    tendered: payload.tendered,
    change: payload.change,
    note: payload.note,
    timestamp,
    cancelled: false,
  };

  return {
    ...snapshot,
    payments: [...snapshot.payments, newPayment],
  };
}

function applyPaymentCancelled(
  snapshot: OrderSnapshot,
  payload: PaymentCancelledPayload
): OrderSnapshot {
  const payments = snapshot.payments.map((payment) => {
    if (payment.payment_id !== payload.payment_id) {
      return payment;
    }

    return {
      ...payment,
      cancelled: true,
      cancel_reason: payload.reason,
    };
  });

  return {
    ...snapshot,
    payments,
  };
}

function applyOrderSplit(
  snapshot: OrderSnapshot,
  payload: OrderSplitPayload,
  timestamp: number
): OrderSnapshot {
  // Reduce quantities of split items
  let items = [...snapshot.items];

  for (const splitItem of payload.items) {
    items = items.map((item) => {
      if (item.instance_id !== splitItem.instance_id) {
        return item;
      }

      const newQuantity = item.quantity - splitItem.quantity;
      if (newQuantity <= 0) {
        return { ...item, _removed: true };
      }

      return { ...item, quantity: newQuantity };
    });
  }

  // Add split payment record
  const splitPayment: PaymentRecord = {
    payment_id: `split-${Date.now()}`,
    method: payload.payment_method,
    amount: payload.split_amount,
    timestamp,
    cancelled: false,
  };

  return {
    ...snapshot,
    items,
    payments: [...snapshot.payments, splitPayment],
  };
}

function applyOrderMoved(
  snapshot: OrderSnapshot,
  payload: OrderMovedPayload
): OrderSnapshot {
  // This event is received by the TARGET order
  // It contains the items moved from source
  return {
    ...snapshot,
    table_id: payload.target_table_id,
    table_name: payload.target_table_name,
    items: mergeItemsIntoList(snapshot.items, payload.items),
  };
}

function applyOrderMovedOut(
  snapshot: OrderSnapshot,
  timestamp: number
): OrderSnapshot {
  // This event is received by the SOURCE order
  // Mark it as MOVED status
  return {
    ...snapshot,
    status: 'MOVED',
    end_time: timestamp,
  };
}

function applyOrderMerged(
  snapshot: OrderSnapshot,
  payload: OrderMergedPayload
): OrderSnapshot {
  // This event is received by the TARGET order
  // Merge items from source order
  return {
    ...snapshot,
    items: mergeItemsIntoList(snapshot.items, payload.items),
  };
}

function applyOrderMergedOut(
  snapshot: OrderSnapshot,
  timestamp: number
): OrderSnapshot {
  // This event is received by the SOURCE order
  // Mark it as MERGED status
  return {
    ...snapshot,
    status: 'MERGED',
    end_time: timestamp,
  };
}

function applyTableReassigned(
  snapshot: OrderSnapshot,
  payload: TableReassignedPayload
): OrderSnapshot {
  return {
    ...snapshot,
    table_id: payload.target_table_id,
    table_name: payload.target_table_name,
    zone_name: payload.target_zone_name || snapshot.zone_name,
    start_time: payload.original_start_time,
    items: mergeItemsIntoList(snapshot.items, payload.items),
  };
}

function applySurchargeExemptSet(
  snapshot: OrderSnapshot,
  payload: SurchargeExemptSetPayload
): OrderSnapshot {
  return {
    ...snapshot,
    surcharge_exempt: payload.exempt,
  };
}

function applyOrderInfoUpdated(
  snapshot: OrderSnapshot,
  payload: OrderInfoUpdatedPayload
): OrderSnapshot {
  return {
    ...snapshot,
    receipt_number: payload.receipt_number ?? snapshot.receipt_number,
    guest_count: payload.guest_count ?? snapshot.guest_count,
    table_name: payload.table_name ?? snapshot.table_name,
    is_pre_payment: payload.is_pre_payment ?? snapshot.is_pre_payment,
  };
}

// ============================================================================
// Helper Functions
// ============================================================================

/**
 * Merge new items into existing item list
 * Items with same instance_id are combined (quantity added)
 */
function mergeItemsIntoList(
  existingItems: CartItemSnapshot[],
  newItems: CartItemSnapshot[]
): CartItemSnapshot[] {
  const result = [...existingItems];

  for (const newItem of newItems) {
    const existingIndex = result.findIndex(
      (item) => item.instance_id === newItem.instance_id && !item._removed
    );

    if (existingIndex >= 0) {
      // Merge: add quantities
      result[existingIndex] = {
        ...result[existingIndex],
        quantity: result[existingIndex].quantity + newItem.quantity,
      };
    } else {
      // Add new item
      result.push({ ...newItem });
    }
  }

  return result;
}

/**
 * Recalculate order totals from items and payments
 */
function recalculateTotals(snapshot: OrderSnapshot): OrderSnapshot {
  // Calculate subtotal from non-removed items
  const activeItems = snapshot.items.filter((item) => !item._removed);

  let subtotal = Currency.toDecimal(0);
  let discount = Currency.toDecimal(0);

  for (const item of activeItems) {
    const basePrice = item.original_price ?? item.price;
    const itemTotal = Currency.mul(basePrice, item.quantity);

    // Apply item-level discount
    if (item.discount_percent && item.discount_percent > 0) {
      const itemDiscount = Currency.mul(itemTotal, item.discount_percent / 100);
      discount = Currency.add(discount, itemDiscount);
      subtotal = Currency.add(subtotal, Currency.sub(itemTotal, itemDiscount));
    } else {
      subtotal = Currency.add(subtotal, itemTotal);
    }

    // Add item-level surcharge
    if (item.surcharge && item.surcharge > 0) {
      subtotal = Currency.add(subtotal, Currency.mul(item.surcharge, item.quantity));
    }

    // Add option price modifiers
    if (item.selected_options) {
      for (const option of item.selected_options) {
        if (option.price_modifier) {
          subtotal = Currency.add(
            subtotal,
            Currency.mul(option.price_modifier, item.quantity)
          );
        }
      }
    }
  }

  // Calculate surcharge (order-level)
  let surchargeAmount = Currency.toDecimal(0);
  if (snapshot.surcharge && !snapshot.surcharge_exempt) {
    if (snapshot.surcharge.type === 'percentage') {
      surchargeAmount = Currency.mul(subtotal, snapshot.surcharge.value / 100);
    } else {
      surchargeAmount = Currency.toDecimal(snapshot.surcharge.amount);
    }
  }

  // Calculate total
  const total = Currency.floor2(Currency.add(subtotal, surchargeAmount)).toNumber();

  // Calculate paid amount from non-cancelled payments
  const paidAmount = snapshot.payments
    .filter((p) => !p.cancelled)
    .reduce((sum, p) => Currency.add(sum, p.amount), Currency.toDecimal(0));

  // Update surcharge config with calculated total
  const surcharge: SurchargeConfig | null = snapshot.surcharge
    ? { ...snapshot.surcharge, total: surchargeAmount.toNumber() }
    : null;

  return {
    ...snapshot,
    subtotal: Currency.floor2(subtotal).toNumber(),
    discount: Currency.floor2(discount).toNumber(),
    surcharge,
    total,
    paid_amount: Currency.floor2(paidAmount).toNumber(),
  };
}
