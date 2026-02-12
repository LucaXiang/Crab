/**
 * Audit Details Renderers
 *
 * 职责分离设计：
 * - 每个 AuditAction 类型有独立的 Renderer
 * - Renderer 负责将审计详情转换为 UI 展示数据
 * - 通过注册表映射，无需 switch case
 */

import type { AuditEntry } from '@/core/domain/types/api';
import type { AuditDetailsRenderer, TranslateFn, AuditDisplayData } from './types';

// Re-export types for consumers
export type { TranslateFn, FieldChange, AuditDisplayData, AuditDetailLine, AuditChangeItem, AuditDetailsRenderer } from './types';

// Renderer imports
import { OrderCompletedRenderer, OrderVoidedRenderer, OrderMergedRenderer } from './orders';
import { ShiftOpenedRenderer, ShiftClosedRenderer } from './shifts';
import { SystemStartupRenderer, SystemAbnormalShutdownRenderer, ResolveSystemIssueRenderer } from './system';
import { LoginSuccessRenderer, EscalationSuccessRenderer, LoginFailedRenderer } from './auth';
import { createSnapshotRenderer, createDiffRenderer, createDeleteRenderer } from './generic';

// ============================================================================
// Renderer Registry
// ============================================================================

type AuditActionType =
  | 'system_startup'
  | 'system_shutdown'
  | 'system_abnormal_shutdown'
  | 'system_long_downtime'
  | 'resolve_system_issue'
  | 'login_success'
  | 'login_failed'
  | 'logout'
  | 'escalation_success'
  | 'order_completed'
  | 'order_voided'
  | 'order_merged'
  | 'employee_created'
  | 'employee_updated'
  | 'employee_deleted'
  | 'role_created'
  | 'role_updated'
  | 'role_deleted'
  | 'shift_opened'
  | 'shift_updated'
  | 'shift_closed'
  | 'product_created'
  | 'product_updated'
  | 'product_deleted'
  | 'category_created'
  | 'category_updated'
  | 'category_deleted'
  | 'tag_created'
  | 'tag_updated'
  | 'tag_deleted'
  | 'attribute_created'
  | 'attribute_updated'
  | 'attribute_deleted'
  | 'price_rule_created'
  | 'price_rule_updated'
  | 'price_rule_deleted'
  | 'zone_created'
  | 'zone_updated'
  | 'zone_deleted'
  | 'table_created'
  | 'table_updated'
  | 'table_deleted'
  | 'label_template_created'
  | 'label_template_updated'
  | 'label_template_deleted'
  | 'print_destination_created'
  | 'print_destination_updated'
  | 'print_destination_deleted'
  | 'member_created'
  | 'member_updated'
  | 'member_deleted'
  | 'marketing_group_created'
  | 'marketing_group_updated'
  | 'marketing_group_deleted'
  | 'daily_report_generated'
  | 'print_config_changed'
  | 'store_info_changed';

/**
 * 审计详情渲染器注册表
 */
export const AUDIT_RENDERERS: Partial<Record<AuditActionType, AuditDetailsRenderer>> = {
  // 系统
  system_startup: SystemStartupRenderer,
  system_shutdown: createSnapshotRenderer(),
  system_abnormal_shutdown: SystemAbnormalShutdownRenderer,
  system_long_downtime: SystemAbnormalShutdownRenderer,
  resolve_system_issue: ResolveSystemIssueRenderer,

  // 认证
  login_success: LoginSuccessRenderer,
  login_failed: LoginFailedRenderer,
  logout: LoginSuccessRenderer,
  escalation_success: EscalationSuccessRenderer,

  // 订单
  order_completed: OrderCompletedRenderer,
  order_voided: OrderVoidedRenderer,
  order_merged: OrderMergedRenderer,

  // 班次
  shift_opened: ShiftOpenedRenderer,
  shift_updated: createDiffRenderer(),
  shift_closed: ShiftClosedRenderer,

  // 员工
  employee_created: createSnapshotRenderer(['hash_pass', 'is_system']),
  employee_updated: createDiffRenderer(),
  employee_deleted: createDeleteRenderer(),

  // 角色
  role_created: createSnapshotRenderer(['is_system']),
  role_updated: createDiffRenderer(),
  role_deleted: createDeleteRenderer(),

  // 商品
  product_created: createSnapshotRenderer(),
  product_updated: createDiffRenderer(),
  product_deleted: createDeleteRenderer(),

  // 分类
  category_created: createSnapshotRenderer(),
  category_updated: createDiffRenderer(),
  category_deleted: createDeleteRenderer(),

  // 标签
  tag_created: createSnapshotRenderer(['is_system']),
  tag_updated: createDiffRenderer(),
  tag_deleted: createDeleteRenderer(),

  // 属性
  attribute_created: createSnapshotRenderer(),
  attribute_updated: createDiffRenderer(),
  attribute_deleted: createDeleteRenderer(),

  // 价格规则
  price_rule_created: createSnapshotRenderer(['created_by', 'created_at']),
  price_rule_updated: createDiffRenderer(),
  price_rule_deleted: createDeleteRenderer(),

  // 区域
  zone_created: createSnapshotRenderer(),
  zone_updated: createDiffRenderer(),
  zone_deleted: createDeleteRenderer(),

  // 桌台
  table_created: createSnapshotRenderer(),
  table_updated: createDiffRenderer(),
  table_deleted: createDeleteRenderer(),

  // 标签模板
  label_template_created: createSnapshotRenderer(),
  label_template_updated: createDiffRenderer(),
  label_template_deleted: createDeleteRenderer(),

  // 打印目的地
  print_destination_created: createSnapshotRenderer(),
  print_destination_updated: createDiffRenderer(),
  print_destination_deleted: createDeleteRenderer(),

  // 会员
  member_created: createSnapshotRenderer(),
  member_updated: createDiffRenderer(),
  member_deleted: createDeleteRenderer(),

  // 营销组
  marketing_group_created: createSnapshotRenderer(),
  marketing_group_updated: createDiffRenderer(),
  marketing_group_deleted: createDeleteRenderer(),

  // 日结
  daily_report_generated: createSnapshotRenderer(),

  // 配置
  print_config_changed: createSnapshotRenderer(),
  store_info_changed: createDiffRenderer(),
};

/**
 * 渲染审计详情
 */
export function renderAuditDetails(
  entry: AuditEntry,
  t: TranslateFn
): AuditDisplayData {
  const details = entry.details as Record<string, unknown> | null;

  if (!details || typeof details !== 'object') {
    return { lines: [], isEmpty: true };
  }

  const renderer = AUDIT_RENDERERS[entry.action as AuditActionType];

  if (!renderer) {
    // Fallback: 使用通用快照渲染器
    return createSnapshotRenderer().render(entry, details, t);
  }

  return renderer.render(entry, details, t);
}
