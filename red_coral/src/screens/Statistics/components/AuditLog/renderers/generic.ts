/**
 * Generic Audit Detail Renderers
 *
 * 通用渲染器工厂，用于 CREATE/UPDATE/DELETE 操作
 */

import type { AuditDetailsRenderer, AuditDetailLine, AuditChangeItem, FieldChange } from './types';
import { formatFieldValue, getFieldLabel } from './helpers';

/**
 * 通用 CREATE 渲染器
 *
 * 处理快照格式：完整的 JSON 对象
 */
export function createSnapshotRenderer(excludeFields: string[] = []): AuditDetailsRenderer {
  return {
    render(_entry, details, t) {
      const lines: AuditDetailLine[] = [];
      const exclude = new Set(['error', ...excludeFields]);

      for (const [field, value] of Object.entries(details)) {
        if (exclude.has(field)) continue;
        lines.push({
          label: getFieldLabel(field, t),
          value: formatFieldValue(field, value, t),
        });
      }

      return {
        lines,
        isEmpty: lines.length === 0,
      };
    },
  };
}

/**
 * 通用 UPDATE 渲染器
 *
 * 处理 diff 格式：{"changes": [{"field": "...", "from": ..., "to": ...}]}
 */
export function createDiffRenderer(): AuditDetailsRenderer {
  return {
    render(_entry, details, t) {
      const changesRaw = details.changes as FieldChange[] | undefined;

      if (!changesRaw || changesRaw.length === 0) {
        // 可能是旧格式或无变更
        const note = details.note as string | undefined;
        if (note === 'no_changes_detected') {
          return {
            lines: [{ label: t('audit.detail.note'), value: t('audit.detail.no_changes') }],
            isEmpty: false,
          };
        }
        // 回退到快照渲染
        return createSnapshotRenderer().render(_entry, details, t);
      }

      const changes: AuditChangeItem[] = changesRaw.map((change) => ({
        field: change.field,
        fieldLabel: getFieldLabel(change.field, t),
        from: formatFieldValue(change.field, change.from, t),
        to: formatFieldValue(change.field, change.to, t),
      }));

      return {
        lines: [],
        changes,
        isEmpty: changes.length === 0,
      };
    },
  };
}

/**
 * 通用 DELETE 渲染器
 *
 * 处理格式：{"name": "..."}
 */
export function createDeleteRenderer(): AuditDetailsRenderer {
  return {
    render(_entry, details, t) {
      const name = details.name as string | undefined;
      if (!name) {
        return { lines: [], isEmpty: true };
      }

      return {
        lines: [{
          label: t('audit.detail.field.name'),
          value: name,
        }],
        isEmpty: false,
      };
    },
  };
}
