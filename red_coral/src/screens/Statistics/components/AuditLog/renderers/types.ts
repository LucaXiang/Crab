import type React from 'react';

export type TranslateFn = (key: string, params?: Record<string, string | number>) => string;

/** 字段变更记录（与后端 FieldChange 对齐） */
export interface FieldChange {
  field: string;
  from: unknown;
  to: unknown;
}

/** 审计详情展示数据 */
export interface AuditDisplayData {
  /** 主要内容（单行或多行） */
  lines: AuditDetailLine[];
  /** 变更列表（用于 UPDATE 操作） */
  changes?: AuditChangeItem[];
  /** 是否为空 */
  isEmpty: boolean;
}

/** 审计详情行 */
export interface AuditDetailLine {
  label: string;
  value: React.ReactNode;
  /** 值的颜色类名 */
  valueClass?: string;
}

/** 变更项 */
export interface AuditChangeItem {
  field: string;
  fieldLabel: string;
  from: React.ReactNode;
  to: React.ReactNode;
}

/**
 * Audit Details Renderer Interface (类似 Rust trait)
 *
 * 每个资源类型实现这个接口，定义如何渲染审计详情
 */
export interface AuditDetailsRenderer {
  render(entry: import('@/core/domain/types/api').AuditEntry, details: Record<string, unknown>, t: TranslateFn): AuditDisplayData;
}
