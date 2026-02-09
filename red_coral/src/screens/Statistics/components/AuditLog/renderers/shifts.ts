import { formatCurrency } from '@/utils/currency/formatCurrency';
import type { AuditDetailsRenderer, AuditDetailLine } from './types';
import { formatTimestamp } from './helpers';

export const ShiftOpenedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.starting_cash != null) {
      lines.push({
        label: t('audit.detail.field.starting_cash'),
        value: formatCurrency(details.starting_cash as number),
      });
    }

    if (details.opened_at != null) {
      lines.push({
        label: t('audit.detail.field.opened_at'),
        value: formatTimestamp(details.opened_at as number),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

export const ShiftClosedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.expected_cash != null) {
      lines.push({
        label: t('audit.detail.field.expected_cash'),
        value: formatCurrency(details.expected_cash as number),
      });
    }

    if (details.actual_cash != null) {
      lines.push({
        label: t('audit.detail.field.actual_cash'),
        value: formatCurrency(details.actual_cash as number),
      });
    }

    if (details.cash_variance != null) {
      const variance = details.cash_variance as number;
      lines.push({
        label: t('audit.detail.field.cash_variance'),
        value: formatCurrency(variance),
        valueClass: variance < 0 ? 'text-red-600' : variance > 0 ? 'text-green-600' : undefined,
      });
    }

    if (details.closed_at != null) {
      lines.push({
        label: t('audit.detail.field.closed_at'),
        value: formatTimestamp(details.closed_at as number),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};
