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

// Archived order types (for history view)
export * from './archivedOrder';

// Credit note types (退款凭证)
export * from './creditNote';

// Chain entry types (统一 hash 链时间线)
export * from './chainEntry';

// Invoice types (Verifactu 发票)
export * from './invoice';

// Print and label types
export * from './print';

// Order Event Sourcing types - import directly from '@/core/domain/types/orderEvent' for full access
export type {
  CartItemSnapshot,
  ItemOption,
  AppliedRule,
  SpecificationInfo,
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
 * HeldOrder = OrderSnapshot (服务端权威快照)
 * Timeline 通过 useActiveOrdersStore.timelines 或 ArchivedOrderDetail.timeline 独立获取
 */
export type HeldOrder = import('./orderEvent').OrderSnapshot;

export type DraftOrder = HeldOrder;

// Permission type and constants
// 18 个可配置权限 + 1 个管理员专属权限
export type Permission = string;

export const Permission = {
  // === 模块化权限 (7) ===
  MENU_MANAGE: 'menu:manage' as Permission,           // 菜单管理
  TABLES_MANAGE: 'tables:manage' as Permission,       // 桌台管理
  SHIFTS_MANAGE: 'shifts:manage' as Permission,       // 班次管理
  REPORTS_VIEW: 'reports:view' as Permission,         // 报表查看
  PRICE_RULES_MANAGE: 'price_rules:manage' as Permission, // 价格规则
  SETTINGS_MANAGE: 'settings:manage' as Permission,   // 系统设置
  MARKETING_MANAGE: 'marketing:manage' as Permission,  // 营销组+会员管理
  ORDERS_LINK_MEMBER: 'orders:link_member' as Permission,   // 订单关联会员
  ORDERS_REDEEM_STAMP: 'orders:redeem_stamp' as Permission, // 订单兑换印花

  // === 敏感操作 (9) ===
  ORDERS_VOID: 'orders:void' as Permission,           // 作废订单
  ORDERS_DISCOUNT: 'orders:discount' as Permission,   // 应用折扣
  ORDERS_COMP: 'orders:comp' as Permission,           // 赠送菜品
  ORDERS_REFUND: 'orders:refund' as Permission,       // 退款
  ORDERS_MODIFY_PRICE: 'orders:modify_price' as Permission, // 修改价格
  ORDERS_CANCEL_ITEM: 'orders:cancel_item' as Permission,   // 删除商品
  TABLES_TRANSFER: 'tables:transfer' as Permission,         // 移台
  TABLES_MERGE_BILL: 'tables:merge_bill' as Permission,     // 合台
  CASH_DRAWER_OPEN: 'cash_drawer:open' as Permission, // 打开钱箱

  // === 管理员专属 ===
  USERS_MANAGE: 'users:manage' as Permission,         // 用户管理 (仅 admin)

} as const;

// Statistics types
export type TimeRange = 'today' | 'yesterday' | 'this_week' | 'this_month' | 'last_month' | 'custom';
export type ActiveTab = 'overview' | 'invoices' | 'reports_shifts' | 'audit_log';

// ── StoreOverview — flat response aligned with edge-server ──

export interface StoreOverview {
  revenue: number;
  net_revenue: number;
  orders: number;
  guests: number;
  average_order_value: number;
  per_guest_spend: number;
  average_dining_minutes: number;
  total_tax: number;
  total_discount: number;
  total_surcharge: number;
  avg_items_per_order: number;
  voided_orders: number;
  voided_amount: number;
  loss_orders: number;
  loss_amount: number;
  anulacion_count: number;
  anulacion_amount: number;
  refund_count: number;
  refund_amount: number;
  revenue_trend: RevenueTrendPoint[];
  daily_trend: DailyTrendPoint[];
  payment_breakdown: PaymentBreakdownEntry[];
  tax_breakdown: TaxBreakdownEntry[];
  category_sales: CategorySaleEntry[];
  top_products: TopProductEntry[];
  tag_sales: TagSaleEntry[];
  refund_method_breakdown: RefundMethodEntry[];
  service_type_breakdown: ServiceTypeEntry[];
  zone_sales: ZoneSaleEntry[];
  discount_breakdown: AdjustmentEntry[];
  surcharge_breakdown: AdjustmentEntry[];
}

export interface AdjustmentEntry {
  /** Rule name or source key (e.g. "item_manual", "mg", "order_manual") */
  name: string;
  /** "item_manual" | "item_rule" | "mg" | "order_manual" | "order_rule" */
  source: string;
  amount: number;
  order_count: number;
}

export interface RevenueTrendPoint {
  hour: number;
  revenue: number;
  orders: number;
}

export interface DailyTrendPoint {
  date: string;
  revenue: number;
  orders: number;
}

export interface PaymentBreakdownEntry {
  method: string;
  amount: number;
  count: number;
}

export interface TaxBreakdownEntry {
  tax_rate: number;
  base_amount: number;
  tax_amount: number;
}

export interface CategorySaleEntry {
  name: string;
  revenue: number;
}

export interface TopProductEntry {
  name: string;
  quantity: number;
  revenue: number;
}

export interface TagSaleEntry {
  name: string;
  color: string | null;
  revenue: number;
  quantity: number;
}

export interface RefundMethodEntry {
  method: string;
  amount: number;
  count: number;
}

export interface ServiceTypeEntry {
  service_type: string;
  revenue: number;
  orders: number;
}

export interface ZoneSaleEntry {
  zone_name: string;
  revenue: number;
  orders: number;
  guests: number;
}

export interface SalesReportItem {
  order_id: number;
  receipt_number: string | null;
  date: string;
  total: number;
  status: string;
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
