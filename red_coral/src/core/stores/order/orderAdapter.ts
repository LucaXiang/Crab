/**
 * Order Adapter - Convert between OrderSnapshot (server) and UI-friendly format
 *
 * OrderSnapshot uses snake_case (from Rust backend)
 * UI components expect camelCase for consistency
 */

import type {
  OrderSnapshot,
  CartItemSnapshot,
  PaymentRecord as ServerPaymentRecord,
  ItemOption,
} from '@/core/domain/types/orderEvent';
import type { HeldOrder, CartItem, PaymentRecord, ItemAttributeSelection } from '@/core/domain/types';

/**
 * Convert ItemOption (backend) to ItemAttributeSelection (frontend)
 */
function toItemAttributeSelection(option: ItemOption): ItemAttributeSelection {
  return {
    attribute_id: option.attribute_id,
    attribute_name: option.attribute_name,
    option_idx: option.option_idx,
    name: option.attribute_name, // Use attribute name as display name
    value: option.option_name,   // Use option name as value
    price_modifier: option.price_modifier ?? undefined,
  };
}

/**
 * Convert CartItemSnapshot (snake_case) to CartItem (camelCase)
 */
export function toCartItem(snapshot: CartItemSnapshot): CartItem {
  return {
    id: snapshot.instance_id, // Use instance_id as the unique identifier
    instanceId: snapshot.instance_id,
    productId: snapshot.id, // CartItemSnapshot.id is the product ID
    name: snapshot.name,
    price: snapshot.price,
    originalPrice: snapshot.original_price ?? snapshot.price,
    quantity: snapshot.quantity,
    unpaidQuantity: snapshot.unpaid_quantity, // Computed by backend
    note: snapshot.note ?? undefined,
    discountPercent: snapshot.discount_percent ?? undefined,
    surcharge: snapshot.surcharge ?? undefined,
    selectedOptions: snapshot.selected_options?.map(toItemAttributeSelection),
    selectedSpecification: snapshot.selected_specification ? {
      id: snapshot.selected_specification.id,
      name: snapshot.selected_specification.name,
      receipt_name: snapshot.selected_specification.receipt_name ?? undefined,
      price: snapshot.selected_specification.price ?? undefined,
    } : undefined,
    _removed: snapshot._removed ?? false,
    authorizerId: snapshot.authorizer_id ?? undefined,
    authorizerName: snapshot.authorizer_name ?? undefined,
  };
}

/**
 * Convert ServerPaymentRecord (snake_case) to PaymentRecord (frontend)
 */
function toPaymentRecord(record: ServerPaymentRecord): PaymentRecord {
  return {
    id: record.payment_id,
    amount: record.amount,
    method: record.method,
    timestamp: record.timestamp,
    note: record.note ?? undefined,
    tendered: record.tendered ?? undefined,
    change: record.change ?? undefined,
  };
}

/**
 * Convert OrderSnapshot (server format) to HeldOrder (UI format)
 *
 * This is the main adapter function used throughout the app.
 */
export function toHeldOrder(snapshot: OrderSnapshot): HeldOrder {
  return {
    id: snapshot.order_id,
    key: snapshot.order_id, // Use order_id as key
    tableKey: snapshot.table_id ?? undefined,
    tableId: snapshot.table_id ? parseInt(snapshot.table_id, 10) || undefined : undefined,
    tableName: snapshot.table_name ?? undefined,
    zoneId: snapshot.zone_id ? parseInt(snapshot.zone_id, 10) || undefined : undefined,
    zoneName: snapshot.zone_name ?? undefined,
    guestCount: snapshot.guest_count,
    items: snapshot.items.map(toCartItem),
    subtotal: snapshot.subtotal,
    tax: snapshot.tax,
    discount: snapshot.discount,
    total: snapshot.total,
    paidAmount: snapshot.paid_amount,
    payments: snapshot.payments.map(toPaymentRecord),
    receiptNumber: snapshot.receipt_number ?? undefined,
    isPrePayment: snapshot.is_pre_payment,
    isRetail: snapshot.is_retail,
    status: snapshot.status,
    startTime: snapshot.start_time,
    endTime: snapshot.end_time ?? undefined,
    timeline: [], // Timeline not available in OrderSnapshot (built client-side if needed)
    createdAt: snapshot.created_at,
    updatedAt: snapshot.updated_at,
  };
}

/**
 * Convert array of OrderSnapshots to HeldOrders
 */
export function toHeldOrders(snapshots: OrderSnapshot[]): HeldOrder[] {
  return snapshots.map(toHeldOrder);
}

/**
 * Type guard to check if an order is a HeldOrder
 */
export function isHeldOrder(order: unknown): order is HeldOrder {
  return (
    typeof order === 'object' &&
    order !== null &&
    'items' in order &&
    'total' in order &&
    !('order_id' in order) // HeldOrder uses 'id', not 'order_id'
  );
}

/**
 * Type guard to check if an order is an OrderSnapshot
 */
export function isOrderSnapshot(order: unknown): order is OrderSnapshot {
  return (
    typeof order === 'object' &&
    order !== null &&
    'order_id' in order &&
    'items' in order
  );
}

// ============================================================================
// ES Event to Frontend Event Adapter
// ============================================================================

import type { OrderEvent as ESOrderEvent, EventPayload } from '@/core/domain/types/orderEvent';
import type { OrderEvent as FrontendOrderEvent, OrderEventType } from '@/core/domain/events/types';

/**
 * Convert CartItemSnapshot to frontend CartItem format for event data
 */
function snapshotToEventItem(snapshot: CartItemSnapshot): CartItem {
  return toCartItem(snapshot);
}

/**
 * Convert ES OrderEvent (from backend) to Frontend OrderEvent format
 *
 * This adapter bridges the new Event Sourcing API with the existing
 * frontend event reducer (adapters.ts). The key differences:
 * - ES uses snake_case, frontend uses camelCase
 * - ES payload has `type` field, frontend uses `data` object
 */
export function toFrontendEvent(esEvent: ESOrderEvent): FrontendOrderEvent {
  const baseEvent = {
    id: esEvent.event_id,
    timestamp: esEvent.timestamp,
    userId: esEvent.operator_id,
  };

  const payload = esEvent.payload;
  const eventType = payload.type as OrderEventType;

  switch (payload.type) {
    case 'TABLE_OPENED':
      return {
        ...baseEvent,
        type: 'TABLE_OPENED' as OrderEventType,
        data: {
          tableId: payload.table_id ?? '',
          tableName: payload.table_name ?? '',
          zoneId: payload.zone_id ?? undefined,
          zoneName: payload.zone_name ?? undefined,
          guestCount: payload.guest_count,
          receiptNumber: payload.receipt_number ?? undefined,
        },
      } as FrontendOrderEvent;

    case 'ITEMS_ADDED':
      return {
        ...baseEvent,
        type: 'ITEMS_ADDED' as OrderEventType,
        data: {
          items: payload.items.map(snapshotToEventItem),
        },
      } as FrontendOrderEvent;

    case 'ITEM_MODIFIED':
      return {
        ...baseEvent,
        type: 'ITEM_MODIFIED' as OrderEventType,
        data: {
          instanceId: payload.source.instance_id,
          itemName: payload.source.name,
          changes: {
            price: payload.changes.price ?? undefined,
            quantity: payload.changes.quantity ?? undefined,
            discountPercent: payload.changes.discount_percent ?? undefined,
            surcharge: payload.changes.surcharge ?? undefined,
            note: payload.changes.note ?? undefined,
          },
          previousValues: {
            price: payload.previous_values.price ?? undefined,
            quantity: payload.previous_values.quantity ?? undefined,
            discountPercent: payload.previous_values.discount_percent ?? undefined,
            surcharge: payload.previous_values.surcharge ?? undefined,
            note: payload.previous_values.note ?? undefined,
          },
        },
      } as FrontendOrderEvent;

    case 'ITEM_REMOVED':
      return {
        ...baseEvent,
        type: 'ITEM_REMOVED' as OrderEventType,
        data: {
          instanceId: payload.instance_id,
          itemName: payload.item_name,
          quantity: payload.quantity ?? undefined,
          reason: payload.reason ?? undefined,
        },
      } as FrontendOrderEvent;

    case 'ITEM_RESTORED':
      return {
        ...baseEvent,
        type: 'ITEM_RESTORED' as OrderEventType,
        data: {
          instanceId: payload.instance_id,
        },
      } as FrontendOrderEvent;

    case 'PAYMENT_ADDED':
      return {
        ...baseEvent,
        type: 'PAYMENT_ADDED' as OrderEventType,
        data: {
          payment: {
            id: payload.payment_id,
            method: payload.method,
            amount: payload.amount,
            tendered: payload.tendered ?? undefined,
            change: payload.change ?? undefined,
            note: payload.note ?? undefined,
            timestamp: esEvent.timestamp,
          },
        },
      } as FrontendOrderEvent;

    case 'PAYMENT_CANCELLED':
      return {
        ...baseEvent,
        type: 'PAYMENT_CANCELLED' as OrderEventType,
        data: {
          paymentId: payload.payment_id,
          reason: payload.reason ?? undefined,
        },
      } as FrontendOrderEvent;

    case 'ORDER_COMPLETED':
      return {
        ...baseEvent,
        type: 'ORDER_COMPLETED' as OrderEventType,
        data: {
          receiptNumber: payload.receipt_number,
          finalTotal: payload.final_total,
        },
      } as FrontendOrderEvent;

    case 'ORDER_VOIDED':
      return {
        ...baseEvent,
        type: 'ORDER_VOIDED' as OrderEventType,
        data: {
          reason: payload.reason ?? undefined,
        },
      } as FrontendOrderEvent;

    case 'ORDER_RESTORED':
      return {
        ...baseEvent,
        type: 'ORDER_RESTORED' as OrderEventType,
        data: {},
      } as FrontendOrderEvent;

    case 'ORDER_SPLIT':
      return {
        ...baseEvent,
        type: 'ORDER_SPLIT' as OrderEventType,
        data: {
          splitAmount: payload.split_amount,
          paymentMethod: payload.payment_method,
          items: payload.items.map(item => ({
            instanceId: item.instance_id,
            name: item.name,
            quantity: item.quantity,
            price: 0, // Not available in SplitItem
          })),
        },
      } as FrontendOrderEvent;

    case 'ORDER_MOVED':
      return {
        ...baseEvent,
        type: 'ORDER_MOVED' as OrderEventType,
        data: {
          sourceTableId: payload.source_table_id,
          sourceTableName: payload.source_table_name,
          targetTableId: payload.target_table_id,
          targetTableName: payload.target_table_name,
          items: payload.items.map(snapshotToEventItem),
        },
      } as FrontendOrderEvent;

    case 'ORDER_MOVED_OUT':
      return {
        ...baseEvent,
        type: 'ORDER_MOVED_OUT' as OrderEventType,
        data: {
          targetTableId: payload.target_table_id,
          targetTableName: payload.target_table_name,
          reason: payload.reason ?? undefined,
        },
      } as FrontendOrderEvent;

    case 'ORDER_MERGED':
      return {
        ...baseEvent,
        type: 'ORDER_MERGED' as OrderEventType,
        data: {
          sourceTableId: payload.source_table_id,
          sourceTableName: payload.source_table_name,
          items: payload.items.map(snapshotToEventItem),
        },
      } as FrontendOrderEvent;

    case 'ORDER_MERGED_OUT':
      return {
        ...baseEvent,
        type: 'ORDER_MERGED_OUT' as OrderEventType,
        data: {
          targetTableId: payload.target_table_id,
          targetTableName: payload.target_table_name,
          reason: payload.reason ?? undefined,
        },
      } as FrontendOrderEvent;

    case 'TABLE_REASSIGNED':
      return {
        ...baseEvent,
        type: 'TABLE_REASSIGNED' as OrderEventType,
        data: {
          sourceTableId: payload.source_table_id,
          sourceTableName: payload.source_table_name,
          targetTableId: payload.target_table_id,
          targetTableName: payload.target_table_name,
          targetZoneName: payload.target_zone_name ?? undefined,
          originalStartTime: payload.original_start_time,
          items: payload.items.map(snapshotToEventItem),
        },
      } as FrontendOrderEvent;

    case 'ORDER_INFO_UPDATED':
      return {
        ...baseEvent,
        type: 'ORDER_INFO_UPDATED' as OrderEventType,
        data: {
          receiptNumber: payload.receipt_number ?? undefined,
          guestCount: payload.guest_count ?? undefined,
          tableName: payload.table_name ?? undefined,
          isPrePayment: payload.is_pre_payment ?? undefined,
        },
      } as FrontendOrderEvent;

    default:
      // Fallback for unknown event types
      return {
        ...baseEvent,
        type: eventType,
        data: payload as Record<string, unknown>,
      } as unknown as FrontendOrderEvent;
  }
}

/**
 * Convert array of ES events to frontend events
 */
export function toFrontendEvents(esEvents: ESOrderEvent[]): FrontendOrderEvent[] {
  return esEvents.map(toFrontendEvent);
}
