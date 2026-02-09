import type {
  ItemSplitPayload,
  AmountSplitPayload,
  AaSplitStartedPayload,
  AaSplitPaidPayload,
  AaSplitCancelledPayload,
} from '@/core/domain/types/orderEvent';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { Split, Users, XCircle } from 'lucide-react';
import type { EventRenderer, TranslateFn } from './types';

export function formatPaymentMethod(method: string, t: TranslateFn): string {
  const lower = method.toLowerCase();
  if (lower === 'cash') return t('checkout.method.cash');
  if (lower === 'card') return t('checkout.method.card');
  return method;
}

export const ItemSplitRenderer: EventRenderer<ItemSplitPayload> = {
  render(event, payload, t) {
    const items = payload.items || [];
    const details = items.map(item => {
      const instanceId = item.instance_id ? `#${item.instance_id.slice(-5)}` : '';
      return `${instanceId} ${item.name} x${item.quantity}`;
    });

    const methodDisplay = formatPaymentMethod(payload.payment_method || '', t);

    return {
      title: t('timeline.item_split'),
      summary: payload.split_amount != null
        ? `${formatCurrency(payload.split_amount)} · ${methodDisplay}`
        : '',
      details,
      icon: Split,
      colorClass: 'bg-teal-500',
      timestamp: event.timestamp,
      tags: payload.payment_id ? [{ text: `#${payload.payment_id.slice(-5)}`, type: 'payment' as const }] : [],
    };
  }
};

export const AmountSplitRenderer: EventRenderer<AmountSplitPayload> = {
  render(event, payload, t) {
    const methodDisplay = formatPaymentMethod(payload.payment_method || '', t);

    return {
      title: t('timeline.amount_split'),
      summary: payload.split_amount != null
        ? `${formatCurrency(payload.split_amount)} · ${methodDisplay}`
        : '',
      details: [],
      icon: Split,
      colorClass: 'bg-teal-500',
      timestamp: event.timestamp,
      tags: payload.payment_id ? [{ text: `#${payload.payment_id.slice(-5)}`, type: 'payment' as const }] : [],
    };
  }
};

export const AaSplitStartedRenderer: EventRenderer<AaSplitStartedPayload> = {
  render(event, payload, t) {
    const summary = (payload.order_total != null && payload.total_shares != null && payload.per_share_amount != null)
      ? `${formatCurrency(payload.order_total)} / ${payload.total_shares}${t('checkout.aa_split.shares_unit')} = ${formatCurrency(payload.per_share_amount)}/${t('checkout.aa_split.shares_unit')}`
      : '';

    return {
      title: t('timeline.aa_split_started'),
      summary,
      details: [],
      icon: Users,
      colorClass: 'bg-cyan-500',
      timestamp: event.timestamp,
    };
  }
};

export const AaSplitPaidRenderer: EventRenderer<AaSplitPaidPayload> = {
  render(event, payload, t) {
    const methodDisplay = formatPaymentMethod(payload.payment_method || '', t);
    const progress = (payload.progress_paid != null && payload.progress_total != null)
      ? ` ${payload.progress_paid}/${payload.progress_total}`
      : '';

    return {
      title: t('timeline.aa_split_paid'),
      summary: payload.amount != null
        ? `${formatCurrency(payload.amount)} · ${methodDisplay}${progress}`
        : '',
      details: [],
      icon: Users,
      colorClass: 'bg-cyan-600',
      timestamp: event.timestamp,
      tags: payload.payment_id ? [{ text: `#${payload.payment_id.slice(-5)}`, type: 'payment' as const }] : [],
    };
  }
};

export const AaSplitCancelledRenderer: EventRenderer<AaSplitCancelledPayload> = {
  render(event, payload, t) {
    const summary = payload.total_shares != null
      ? t('timeline.aa_split_cancelled_summary', { n: payload.total_shares })
      : '';

    return {
      title: t('timeline.aa_split_cancelled'),
      summary,
      details: [],
      icon: XCircle,
      colorClass: 'bg-red-400',
      timestamp: event.timestamp,
    };
  }
};
