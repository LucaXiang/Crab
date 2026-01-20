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
  SurchargeConfig,
  ItemOption,
} from '@/core/domain/types/orderEvent';
import type { HeldOrder, CartItem, PaymentRecord, SurchargeInfo, ItemAttributeSelection } from '@/core/domain/types';

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
 * Convert SurchargeConfig (snake_case) to SurchargeInfo (camelCase)
 */
export function toSurchargeInfo(config: SurchargeConfig | null): SurchargeInfo | undefined {
  if (!config) return undefined;
  return {
    type: config.type,
    value: config.value,
    amount: config.amount,
    total: config.total ?? undefined,
    name: config.name ?? undefined,
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
    surcharge: toSurchargeInfo(snapshot.surcharge),
    surchargeExempt: snapshot.surcharge_exempt,
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
