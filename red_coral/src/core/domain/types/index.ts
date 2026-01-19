/**
 * Domain Types
 *
 * Unified type definitions for the application.
 * All types are organized into subdirectories by domain.
 *
 * Note: Some types with conflicting names exist in different submodules.
 * Import from specific submodules when needed:
 * - '@/core/domain/types/api' - Backend API types (matches Rust server)
 * - '@/core/domain/types/events' - Client-side event sourcing types
 * - '@/core/domain/types/print' - Print and label types
 * - '@/core/domain/types/pricing' - Frontend pricing adjustment types
 */

// API types (models, requests, responses) - primary source
export * from './api';

// Print and label types
export * from './print';

// Frontend-specific types for cart and orders
export interface CartItem {
  id: string;
  instanceId?: string;
  originalInstanceId?: string;
  productId: string;  // SurrealDB string ID
  specificationId?: string;  // SurrealDB string ID
  name: string;
  price: number;
  originalPrice?: number;
  quantity: number;
  note?: string;
  attributes?: ItemAttributeSelection[];
  selectedOptions?: ItemAttributeSelection[];
  selectedSpecification?: {
    id: string;
    name: string;
    receipt_name?: string;
    price?: number;
  };
  _removed?: boolean;
  discountPercent?: number;
  surcharge?: number;
  externalId?: string;
  authorizerId?: string;
  authorizerName?: string;
}

export interface ItemAttributeSelection {
  attribute_id: string;  // SurrealDB string ID (attr_id)
  option_idx: number;    // Index into attribute.options array
  name: string;
  value: string;
  price_modifier?: number;
  // Frontend display fields
  attribute_name?: string;
  attribute_receipt_name?: string | null;
  kitchen_printer?: string | null;  // SurrealDB string ID
  receipt_name?: string | null;
  option_name?: string;  // Added for option name display
}

export interface PaymentRecord {
  id: string;
  amount: number;
  method: string;
  timestamp: number;
  note?: string;
  tendered?: number;
  change?: number;
}

export interface SurchargeInfo {
  type: 'percentage' | 'fixed';
  amount: number;
  total: number;
  value?: number;
  name?: string;
}

export type OrderStatus = 'ACTIVE' | 'COMPLETED' | 'VOID' | 'MOVED' | 'MERGED';

export interface HeldOrder {
  id?: string;
  key?: string;
  tableKey?: string;
  tableId?: number;
  tableName?: string;
  zoneId?: number;
  zoneName?: string;
  guestCount?: number;
  items: CartItem[];
  subtotal: number;
  tax: number;
  discount: number;
  surcharge?: SurchargeInfo;
  surchargeExempt?: boolean;
  total: number;
  paidAmount?: number;
  paidItemQuantities?: Record<string, number>;
  payments: PaymentRecord[];
  note?: string;
  receiptNumber?: string;
  isPrePayment?: boolean;
  isRetail?: boolean;
  status?: OrderStatus;
  startTime?: number;
  endTime?: number;
  timeline: TimelineEvent[];
  createdAt: number;
  updatedAt: number;
}

export interface TimelineEvent {
  id?: string;
  type: 'ITEM_ADDED' | 'ITEM_REMOVED' | 'QUANTITY_CHANGED' | 'NOTE_ADDED' | 'ORDER_CREATED' | 'PAYMENT_ADDED' | 'STATUS_CHANGED' | 'PAYMENT' | 'ORDER_SPLIT' | 'TABLE_OPENED' | 'ITEMS_ADDED' | 'ITEM_MODIFIED' | 'ITEM_RESTORED' | 'PAYMENT_CANCELLED' | 'ORDER_COMPLETED' | 'ORDER_VOIDED' | 'ORDER_RESTORED' | 'ORDER_SURCHARGE_EXEMPT_SET' | 'ORDER_MERGED' | 'ORDER_MOVED' | 'ORDER_MOVED_OUT' | 'ORDER_MERGED_OUT' | 'TABLE_REASSIGNED' | 'ORDER_INFO_UPDATED';
  timestamp: number;
  data: Record<string, unknown>;
  userId?: number;
  title?: string;
  summary?: string;
}

export type CheckoutMode = 'retail' | 'dine-in' | 'takeout' | 'SELECT';
export type DetailTab = 'items' | 'payments' | 'timeline';
export interface PendingCashTx {
  id: string;
  amount: number;
  timestamp: number;
}

export type DraftOrder = HeldOrder;
export type CompletedOrder = HeldOrder;

// Permission type and constants
export type Permission = string;

export const Permission = {
  MANAGE_USERS: 'manage_users' as Permission,
  VOID_ORDER: 'void_order' as Permission,
  RESTORE_ORDER: 'restore_order' as Permission,
  MANAGE_PRODUCTS: 'manage_products' as Permission,
  CREATE_PRODUCT: 'create_product' as Permission,
  UPDATE_PRODUCT: 'update_product' as Permission,
  DELETE_PRODUCT: 'delete_product' as Permission,
  MANAGE_CATEGORIES: 'manage_categories' as Permission,
  MANAGE_ZONES: 'manage_zones' as Permission,
  MANAGE_TABLES: 'manage_tables' as Permission,
  MODIFY_PRICE: 'modify_price' as Permission,
  APPLY_DISCOUNT: 'apply_discount' as Permission,
  VIEW_STATISTICS: 'view_statistics' as Permission,
  MANAGE_PRINTERS: 'manage_printers' as Permission,
  MANAGE_ATTRIBUTES: 'manage_attributes' as Permission,
  MANAGE_SETTINGS: 'manage_settings' as Permission,
  SYSTEM_SETTINGS: 'system_settings' as Permission,
  PRINT_RECEIPTS: 'print_receipts' as Permission,
  REPRINT_RECEIPT: 'reprint_receipt' as Permission,
  REFUND: 'refund' as Permission,
  DISCOUNT: 'discount' as Permission,
  CANCEL_ITEM: 'cancel_item' as Permission,
  OPEN_CASH_DRAWER: 'open_cash_drawer' as Permission,
  MERGE_BILL: 'merge_bill' as Permission,
  TRANSFER_TABLE: 'transfer_table' as Permission,
} as const;

// Statistics types
export type TimeRange = 'today' | 'week' | 'month' | 'year' | 'custom';
export type ActiveTab = 'overview' | 'sales' | 'products' | 'categories';

export interface OverviewStats {
  todayRevenue: number;
  todayOrders: number;
  todayCustomers: number;
  averageOrderValue: number;
  cashRevenue: number;
  cardRevenue: number;
  otherRevenue: number;
  voidedOrders: number;
  voidedAmount: number;
  totalDiscount: number;
  avgGuestSpend: number;
  avgDiningTime?: number;
}

// Matches Rust RevenueTrendPoint
export interface RevenueTrendPoint {
  time: string;
  value: number;
}

// Matches Rust CategorySale
export interface CategorySale {
  name: string;
  value: number;
  color: string;
}

// Matches Rust TopProduct
export interface TopProduct {
  name: string;
  sales: number;
}

export interface SalesReportItem {
  orderId: number;
  receiptNumber: string | null;
  date: string;
  total: number;
  status: string;
}

export interface StatisticsResponse {
  overview: OverviewStats;
  revenueTrend: RevenueTrendPoint[];
  categorySales: CategorySale[];
  topProducts: TopProduct[];
}
