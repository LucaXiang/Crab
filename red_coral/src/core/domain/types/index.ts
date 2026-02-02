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

// Archived order types (for history view, from SurrealDB graph model)
export * from './archivedOrder';

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
  ItemsAddedPayload,
  ItemModifiedPayload,
  ItemModificationResult,
  ItemRemovedPayload,
  ItemRestoredPayload,
  PaymentAddedPayload,
  PaymentCancelledPayload,
  ItemSplitPayload,
  AmountSplitPayload,
  AaSplitStartedPayload,
  AaSplitPaidPayload,
  AaSplitCancelledPayload,
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
  AddItemsCommand,
  ModifyItemCommand,
  RemoveItemCommand,
  RestoreItemCommand,
  AddPaymentCommand,
  CancelPaymentCommand,
  SplitByItemsCommand,
  SplitByAmountCommand,
  StartAaSplitCommand,
  PayAaSplitCommand,
  SplitType,
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
  VoidType,
  LossReason,
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
  // User management
  USERS_READ: 'users:read' as Permission,
  USERS_MANAGE: 'users:manage' as Permission,
  // Product permissions
  PRODUCTS_READ: 'products:read' as Permission,
  PRODUCTS_WRITE: 'products:write' as Permission,
  PRODUCTS_DELETE: 'products:delete' as Permission,
  PRODUCTS_MANAGE: 'products:manage' as Permission,
  // Category permissions
  CATEGORIES_READ: 'categories:read' as Permission,
  CATEGORIES_MANAGE: 'categories:manage' as Permission,
  // Attribute permissions
  ATTRIBUTES_READ: 'attributes:read' as Permission,
  ATTRIBUTES_MANAGE: 'attributes:manage' as Permission,
  // Order permissions
  ORDERS_READ: 'orders:read' as Permission,
  ORDERS_WRITE: 'orders:write' as Permission,
  ORDERS_VOID: 'orders:void' as Permission,
  ORDERS_DISCOUNT: 'orders:discount' as Permission,
  ORDERS_COMP: 'orders:comp' as Permission,
  ORDERS_REFUND: 'orders:refund' as Permission,
  ORDERS_CANCEL_ITEM: 'orders:cancel_item' as Permission,
  // Zone & Table permissions
  ZONES_READ: 'zones:read' as Permission,
  ZONES_MANAGE: 'zones:manage' as Permission,
  TABLES_READ: 'tables:read' as Permission,
  TABLES_MANAGE: 'tables:manage' as Permission,
  TABLES_MERGE_BILL: 'tables:merge_bill' as Permission,
  TABLES_TRANSFER: 'tables:transfer' as Permission,
  // Pricing permissions
  PRICING_READ: 'pricing:read' as Permission,
  PRICING_WRITE: 'pricing:write' as Permission,
  // Statistics
  STATISTICS_READ: 'statistics:read' as Permission,
  // Printer permissions
  PRINTERS_READ: 'printers:read' as Permission,
  PRINTERS_MANAGE: 'printers:manage' as Permission,
  // Receipt permissions
  RECEIPTS_PRINT: 'receipts:print' as Permission,
  RECEIPTS_REPRINT: 'receipts:reprint' as Permission,
  // Settings & System
  SETTINGS_MANAGE: 'settings:manage' as Permission,
  SYSTEM_READ: 'system:read' as Permission,
  SYSTEM_WRITE: 'system:write' as Permission,
  // Role management
  ROLES_READ: 'roles:read' as Permission,
  ROLES_WRITE: 'roles:write' as Permission,
  // POS operations
  POS_CASH_DRAWER: 'pos:cash_drawer' as Permission,
} as const;

// Statistics types
export type TimeRange = 'today' | 'week' | 'month' | 'year' | 'custom';
export type ActiveTab = 'overview' | 'sales' | 'products' | 'categories' | 'daily_report' | 'audit_log';

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
  order_id: string;
  receipt_number: string | null;
  date: string;
  total: number;
  status: string;
}

export interface StatisticsResponse {
  overview: OverviewStats;
  revenue_trend: RevenueTrendPoint[];
  category_sales: CategorySale[];
  top_products: TopProduct[];
}

export interface SalesReportResponse {
  items: SalesReportItem[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
}

// App state types
export * from './appState';
