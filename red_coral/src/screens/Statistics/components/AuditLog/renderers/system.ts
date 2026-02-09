import type { AuditDetailsRenderer, AuditDetailLine } from './types';
import { formatTimestamp } from './helpers';

export const SystemStartupRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.source) {
      const sourceKey = `audit.detail.value.${details.source}`;
      lines.push({
        label: t('audit.detail.field.source'),
        value: t(sourceKey),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

export const SystemAbnormalShutdownRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.kind) {
      const kindKey = `audit.detail.value.${details.kind}`;
      lines.push({
        label: t('audit.detail.field.kind'),
        value: t(kindKey),
        valueClass: 'text-red-600',
      });
    }

    if (details.last_activity_timestamp != null) {
      lines.push({
        label: t('audit.detail.field.last_activity_timestamp'),
        value: formatTimestamp(details.last_activity_timestamp as number),
      });
    }

    if (details.note) {
      const noteKey = `audit.detail.value.${details.note}`;
      const translated = t(noteKey);
      lines.push({
        label: t('audit.detail.field.note'),
        value: translated.startsWith('audit.detail.value.') ? String(details.note) : translated,
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

export const ResolveSystemIssueRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.response) {
      const responseKey = `audit.detail.value.${details.response}`;
      lines.push({
        label: t('audit.detail.field.response'),
        value: t(responseKey),
      });
    }

    if (details.note) {
      lines.push({
        label: t('audit.detail.field.note'),
        value: String(details.note),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};
