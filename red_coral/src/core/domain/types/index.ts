/**
 * Domain Types
 *
 * Unified type definitions for the application.
 * All types are organized into subdirectories by domain.
 *
 * Note: Some types with conflicting names exist in different submodules.
 * Import from specific submodules when needed:
 * - '@/core/domain/types/api' - Backend API types (matches Rust server)
 * - '@/core/domain/types/events' - Client-side event sourcing types (legacy)
 * - '@/core/domain/types/orderEvent' - Server-side event sourcing types (new)
 * - '@/core/domain/types/print' - Print and label types
 * - '@/core/domain/types/pricing' - Frontend pricing adjustment types
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
} from './orderEvent';

// ============================================================================
// Type Aliases - Unified to Backend Standard
// ============================================================================

/**
 * CartItem 临时扩展 (等待后端添加字段)
 *
 * TODO: 后端在 CartItemSnapshot 添加 product_id 和 external_id 后删除此扩展
 * - product_id: 应该是 id 字段的别名（兼容性）
 * - external_id: 应该从 Product 表关联查询
 *
 * 当前使用此扩展避免 TS2339 编译错误。
 */
export type CartItem = import('./orderEvent').CartItemSnapshot & {
  product_id?: string;  // TODO: 等待后端添加
  external_id?: string; // TODO: 等待后端添加
};

/**
 * PaymentRecord is now an alias for backend PaymentRecord
 * Use backend type directly for consistency
 */
export type PaymentRecord = import('./orderEvent').PaymentRecord;

export type OrderStatus = 'ACTIVE' | 'COMPLETED' | 'VOID' | 'MOVED' | 'MERGED';

/**
 * HeldOrder is OrderSnapshot plus optional timeline
 *
 * Timeline is optional and stores OrderEvent[] (服务端权威类型).
 * UI 层使用 Renderer 按需格式化，不存储转换后的数据。
 *
 * TODO: items 字段临时使用 CartItem[] (包含 product_id/external_id 扩展)
 */
export type HeldOrder = Omit<import('./orderEvent').OrderSnapshot, 'items'> & {
  // Override items to use CartItem (with temporary extensions)
  items: CartItem[];

  // Legacy aliases for backward compatibility
  key?: string;                   // Alias for order_id
  id?: string;                    // Alias for order_id

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
