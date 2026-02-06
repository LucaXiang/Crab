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
// 简化权限系统：12 个可配置权限 + 1 个管理员专属权限
export type Permission = string;

export const Permission = {
  // === 模块化权限 (6) ===
  MENU_MANAGE: 'menu:manage' as Permission,           // 菜单管理
  TABLES_MANAGE: 'tables:manage' as Permission,       // 桌台管理
  SHIFTS_MANAGE: 'shifts:manage' as Permission,       // 班次管理
  REPORTS_VIEW: 'reports:view' as Permission,         // 报表查看
  PRICE_RULES_MANAGE: 'price_rules:manage' as Permission, // 价格规则
  SETTINGS_MANAGE: 'settings:manage' as Permission,   // 系统设置

  // === 敏感操作 (6) ===
  ORDERS_VOID: 'orders:void' as Permission,           // 作废订单
  ORDERS_DISCOUNT: 'orders:discount' as Permission,   // 应用折扣
  ORDERS_COMP: 'orders:comp' as Permission,           // 赠送菜品
  ORDERS_REFUND: 'orders:refund' as Permission,       // 退款
  ORDERS_MODIFY_PRICE: 'orders:modify_price' as Permission, // 修改价格
  CASH_DRAWER_OPEN: 'cash_drawer:open' as Permission, // 打开钱箱

  // === 管理员专属 ===
  USERS_MANAGE: 'users:manage' as Permission,         // 用户管理 (仅 admin)

  // === 兼容性别名 (deprecated, 用于过渡) ===
  /** @deprecated Use MENU_MANAGE */
  PRODUCTS_WRITE: 'menu:manage' as Permission,
  /** @deprecated Use MENU_MANAGE */
  PRODUCTS_DELETE: 'menu:manage' as Permission,
  /** @deprecated Use MENU_MANAGE */
  CATEGORIES_MANAGE: 'menu:manage' as Permission,
  /** @deprecated Use MENU_MANAGE */
  ATTRIBUTES_MANAGE: 'menu:manage' as Permission,
  /** @deprecated Use TABLES_MANAGE */
  ZONES_MANAGE: 'tables:manage' as Permission,
  /** @deprecated Use REPORTS_VIEW */
  STATISTICS_READ: 'reports:view' as Permission,
  /** @deprecated Use CASH_DRAWER_OPEN */
  POS_CASH_DRAWER: 'cash_drawer:open' as Permission,
  /** @deprecated Use SETTINGS_MANAGE */
  SYSTEM_WRITE: 'settings:manage' as Permission,
  /** @deprecated Use SETTINGS_MANAGE */
  PRINTERS_MANAGE: 'settings:manage' as Permission,
  /** @deprecated Use SETTINGS_MANAGE */
  RECEIPTS_REPRINT: 'settings:manage' as Permission,
  /** @deprecated 基础操作，无需权限检查 */
  ORDERS_CANCEL_ITEM: 'orders:void' as Permission,
  /** @deprecated 基础操作，无需权限检查 */
  TABLES_MERGE_BILL: 'tables:manage' as Permission,
  /** @deprecated 基础操作，无需权限检查 */
  TABLES_TRANSFER: 'tables:manage' as Permission,
} as const;

// Statistics types
export type TimeRange = 'today' | 'week' | 'month' | 'year' | 'custom';
export type ActiveTab = 'overview' | 'sales' | 'daily_report' | 'audit_log';

export interface OverviewStats {
  revenue: number;
  orders: number;
  customers: number;
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
