import type {
  TableOpenedPayload,
  OrderCompletedPayload,
  OrderVoidedPayload,
} from '@/core/domain/types/orderEvent';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { Utensils, CheckCircle, Ban } from 'lucide-react';
import type { EventRenderer } from './types';

export const TableOpenedRenderer: EventRenderer<TableOpenedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    if (payload.table_name) {
      details.push(`${t('timeline.labels.table')}: ${payload.table_name}`);
    }
    if (payload.zone_name) {
      details.push(`${t('timeline.labels.zone')}: ${payload.zone_name}`);
    }
    if (payload.receipt_number) {
      details.push(`${t('timeline.labels.receipt')}: ${payload.receipt_number}`);
    }
    if (payload.is_retail) {
      details.push(`${t('timeline.labels.type')}: ${t('timeline.retail_mode')}`);
    }

    return {
      title: t('timeline.table_order'),
      summary: payload.guest_count != null ? t('timeline.guests_count', { n: payload.guest_count }) : '',
      details,
      icon: Utensils,
      colorClass: 'bg-blue-500',
      timestamp: event.timestamp,
    };
  }
};

export const OrderCompletedRenderer: EventRenderer<OrderCompletedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    if (payload.final_total != null) {
      details.push(`${t('timeline.labels.total')}: ${formatCurrency(payload.final_total)}`);
    }

    if (payload.payment_summary && payload.payment_summary.length > 0) {
      payload.payment_summary.forEach((item) => {
        const methodKey = `checkout.method.${item.method?.toLowerCase() || 'other'}`;
        const methodDisplay = t(methodKey);
        details.push(`${methodDisplay}: ${formatCurrency(item.amount)}`);
      });
    }

    return {
      title: t('timeline.order_completed'),
      summary: payload.receipt_number
        ? t('timeline.receipt_no', { n: payload.receipt_number })
        : '',
      details,
      icon: CheckCircle,
      colorClass: 'bg-green-600',
      timestamp: event.timestamp,
    };
  }
};

export const OrderVoidedRenderer: EventRenderer<OrderVoidedPayload> = {
  render(event, payload, t) {
    const isLossSettled = payload.void_type === 'LOSS_SETTLED';
    const summary = isLossSettled
      ? t('checkout.void.type.loss_settled')
      : t('checkout.void.type.cancelled');
    const details: string[] = [];

    if (payload.loss_reason) {
      const reasonKey = `checkout.void.loss_reason.${payload.loss_reason.toLowerCase()}`;
      details.push(`${t('timeline.labels.reason')}: ${t(reasonKey)}`);
    }

    if (payload.loss_amount != null) {
      details.push(`${t('timeline.labels.loss_amount')}: ${formatCurrency(payload.loss_amount)}`);
    }

    if (payload.note) {
      const parts = payload.note.split(' - ');
      const cancelKey = `checkout.void.cancel_reason.${parts[0]}`;
      const resolved = t(cancelKey);
      if (resolved !== cancelKey) {
        parts[0] = resolved;
      }
      details.push(parts.join(' - '));
    }

    if (payload.authorizer_name) {
      details.push(`${t('timeline.labels.authorizer')}: ${payload.authorizer_name}`);
    }

    return {
      title: t('timeline.order_voided'),
      summary,
      details,
      icon: Ban,
      colorClass: isLossSettled ? 'bg-orange-600' : 'bg-red-700',
      timestamp: event.timestamp,
    };
  }
};
