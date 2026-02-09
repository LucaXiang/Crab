import { formatCurrency } from '@/utils/currency/formatCurrency';
import type { AuditDetailsRenderer, AuditDetailLine } from './types';

export const OrderCompletedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.receipt_number) {
      lines.push({
        label: t('audit.detail.field.receipt_number'),
        value: String(details.receipt_number),
      });
    }

    if (details.total != null) {
      lines.push({
        label: t('audit.detail.field.total'),
        value: formatCurrency(details.total as number),
      });
    }

    if (details.item_count != null) {
      lines.push({
        label: t('audit.detail.field.item_count'),
        value: String(details.item_count),
      });
    }

    const paymentSummary = details.payment_summary as Array<{ method: string; amount: number }> | undefined;
    if (paymentSummary && paymentSummary.length > 0) {
      paymentSummary.forEach((p) => {
        const methodKey = `checkout.method.${p.method.toLowerCase()}`;
        const methodDisplay = t(methodKey);
        lines.push({
          label: methodDisplay.startsWith('checkout.method.') ? p.method : methodDisplay,
          value: formatCurrency(p.amount),
        });
      });
    }

    if (details.table_name) {
      lines.push({
        label: t('audit.detail.field.table_name'),
        value: String(details.table_name),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

export const OrderVoidedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.receipt_number) {
      lines.push({
        label: t('audit.detail.field.receipt_number'),
        value: String(details.receipt_number),
      });
    }

    if (details.void_type) {
      const voidTypeKey = `checkout.void.type.${(details.void_type as string).toLowerCase()}`;
      lines.push({
        label: t('audit.detail.field.void_type'),
        value: t(voidTypeKey),
        valueClass: details.void_type === 'LOSS_SETTLED' ? 'text-orange-600' : 'text-red-600',
      });
    }

    if (details.total != null) {
      lines.push({
        label: t('audit.detail.field.total'),
        value: formatCurrency(details.total as number),
      });
    }

    if (details.loss_amount != null) {
      lines.push({
        label: t('audit.detail.field.loss_amount'),
        value: formatCurrency(details.loss_amount as number),
        valueClass: 'text-red-600',
      });
    }

    if (details.loss_reason) {
      const reasonKey = `checkout.void.loss_reason.${(details.loss_reason as string).toLowerCase()}`;
      lines.push({
        label: t('audit.detail.field.loss_reason'),
        value: t(reasonKey),
      });
    }

    if (details.void_note) {
      lines.push({
        label: t('audit.detail.field.note'),
        value: String(details.void_note),
      });
    }

    if (details.authorizer_name) {
      lines.push({
        label: t('audit.detail.field.authorizer_name'),
        value: String(details.authorizer_name),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};

export const OrderMergedRenderer: AuditDetailsRenderer = {
  render(_entry, details, t) {
    const lines: AuditDetailLine[] = [];

    if (details.receipt_number) {
      lines.push({
        label: t('audit.detail.field.receipt_number'),
        value: String(details.receipt_number),
      });
    }

    if (details.source_table) {
      lines.push({
        label: t('audit.detail.field.source_table'),
        value: String(details.source_table),
      });
    }

    if (details.merged_item_count != null) {
      lines.push({
        label: t('audit.detail.field.merged_item_count'),
        value: String(details.merged_item_count),
      });
    }

    if (details.merged_paid_amount != null) {
      lines.push({
        label: t('audit.detail.field.merged_paid_amount'),
        value: formatCurrency(details.merged_paid_amount as number),
      });
    }

    if (details.total != null) {
      lines.push({
        label: t('audit.detail.field.total'),
        value: formatCurrency(details.total as number),
      });
    }

    if (details.authorizer_name) {
      lines.push({
        label: t('audit.detail.field.authorizer_name'),
        value: String(details.authorizer_name),
      });
    }

    return { lines, isEmpty: lines.length === 0 };
  },
};
