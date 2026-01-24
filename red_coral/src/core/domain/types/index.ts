/**
 * Domain Types
 *
 * Unified type definitions for the application.
 * All types are organized into subdirectories by domain.
 *
 * Import from specific submodules when needed:
 * - '@/core/domain/types/api' - Backend API types (matches Rust server)
 * - '@/core/domain/types/orderEvent' - Event sourcing types (commands, events, snapshots)
 * - '@/core/domain/types/print' - Print and label types
 */

// API types (models, requests, responses) - primary source
export * from './api';

// Print and label types
export * from './print';

// Order Event Sourcing types (new server-side state architecture)
// Import with namespace to avoid conflicts: import { OrderEvent } from '@/core/domain/types/orderEvent'
// Or use: import type { OrderEvent as ServerOrderEvent } from '@/core/domain/types/orderEvent'
export type {
  // Event types
  OrderEventType,
  OrderEvent,
  EventPayload,
  TableOpenedPayload,
  OrderCompletedPayload,
  OrderVoidedPayload,
  OrderRestoredPayload,
  ItemsAddedPayload,
  ItemModifiedPayload,
  ItemModificationResult,
  ItemRemovedPayload,
  ItemRestoredPayload,
  PaymentAddedPayload,
  PaymentCancelledPayload,
  OrderSplitPayload,
  OrderMovedPayload,
  OrderMovedOutPayload,
  OrderMergedPayload,
  OrderMergedOutPayload,
  TableReassignedPayload,
  OrderInfoUpdatedPayload,
  RuleSkipToggledPayload,
  // Command types
  OrderCommand,
  OrderCommandPayload,
  OpenTableCommand,
  CompleteOrderCommand,
  VoidOrderCommand,
  RestoreOrderCommand,
  AddItemsCommand,
  ModifyItemCommand,
  RemoveItemCommand,
  RestoreItemCommand,
  AddPaymentCommand,
  CancelPaymentCommand,
  SplitOrderCommand,
  MoveOrderCommand,
  MergeOrdersCommand,
  UpdateOrderInfoCommand,
  ToggleRuleSkipCommand,
  // Response types
  CommandResponse,
  CommandError,
  CommandErrorCode,
  // Sync types
  SyncRequest,
  SyncResponse,
  // Snapshot types
  OrderSnapshot,
  // Shared types (snake_case for Rust compatibility)
  CartItemSnapshot,
  CartItemInput,
  ItemOption,
  SpecificationInfo,
  ItemChanges as ServerItemChanges,
  SplitItem,
  PaymentSummaryItem,
  PaymentRecord as ServerPaymentRecord,
  PaymentMethod,
  OrderConnectionState,
  AppliedRule,
  OrderStatus,  // Event sourcing status: ACTIVE | COMPLETED | VOID | MOVED | MERGED
} from './orderEvent';

// ============================================================================
// Type Aliases - Unified to Backend Standard
// ============================================================================

/**
 * CartItem is an alias for CartItemSnapshot
 */
export type CartItem = import('./orderEvent').CartItemSnapshot;

/**
 * PaymentRecord is now an alias for backend PaymentRecord
 * Use backend type directly for consistency
 */
export type PaymentRecord = import('./orderEvent').PaymentRecord;

/**
 * HeldOrder is OrderSnapshot plus optional timeline
 *
 * Timeline is optional and stores OrderEvent[] (服务端权威类型).
 * UI 层使用 Renderer 按需格式化，不存储转换后的数据。
 */
export type HeldOrder = import('./orderEvent').OrderSnapshot & {
  // Timeline: 存储原始 OrderEvent[]，UI 层按需格式化
  timeline?: import('./orderEvent').OrderEvent[];
};

// ============================================================================
// TimelineEvent 已删除
// ============================================================================
// Timeline 现在直接使用 OrderEvent[]（服务端权威类型）
// UI 层通过 Renderer 按需格式化，不再存储转换后的数据

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
  today_revenue: number;
  today_orders: number;
  today_customers: number;
  average_order_value: number;
  cash_revenue: number;
  card_revenue: number;
  other_revenue: number;
  voided_orders: number;
  voided_amount: number;
  total_discount: number;
  avg_guest_spend: number;
  avg_dining_time?: number;
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
  order_id: number;
  receipt_number: string | null;
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
