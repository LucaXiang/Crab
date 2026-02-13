import type {
  PaymentAddedPayload,
  PaymentCancelledPayload,
} from '@/core/domain/types/orderEvent';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { Coins, Ban } from 'lucide-react';
import type { EventRenderer } from './types';

export const PaymentAddedRenderer: EventRenderer<PaymentAddedPayload> = {
  render(event, payload, t) {
    const method = payload.method || 'unknown';
    const methodLower = method.toLowerCase();
    let methodDisplay: string = method;
    if (methodLower === 'cash') {
      methodDisplay = t('checkout.method.cash');
    } else if (methodLower === 'card') {
      methodDisplay = t('checkout.method.card');
    }

    const details: string[] = [];

    if (payload.tendered !== undefined && payload.tendered !== null) {
      details.push(`${t('checkout.amount.tendered')}: ${formatCurrency(payload.tendered)}`);
      details.push(`${t('checkout.amount.change')}: ${formatCurrency(payload.change || 0)}`);
    }

    if (payload.note) {
      details.push(payload.note);
    }

    return {
      title: `${t('timeline.payment')}: ${methodDisplay}`,
      summary: payload.amount != null ? `+${formatCurrency(payload.amount)}` : '',
      details,
      icon: Coins,
      colorClass: 'bg-green-500',
      timestamp: event.timestamp,
      tags: payload.payment_id ? [{ text: `#${payload.payment_id.slice(-5)}`, type: 'payment' as const }] : [],
    };
  }
};

export const PaymentCancelledRenderer: EventRenderer<PaymentCancelledPayload> = {
  render(event, payload, t) {
    const methodKey = `checkout.method.${payload.method?.toLowerCase() || 'other'}`;
    const methodDisplay = t(methodKey);

    const details: string[] = [];

    if (payload.amount != null) {
      details.push(`${t('checkout.amount.paid')}: ${formatCurrency(payload.amount)}`);
    }
    if (payload.reason) {
      details.push(`${t('timeline.labels.reason')}: ${payload.reason}`);
    }
    if (payload.authorizer_name) {
      details.push(`${t('timeline.labels.authorizer')}: ${payload.authorizer_name}`);
    }

    return {
      title: `${t('timeline.payment_cancelled')}: ${methodDisplay}`,
      summary: payload.amount != null ? `-${formatCurrency(payload.amount)}` : '',
      details,
      icon: Ban,
      colorClass: 'bg-red-400',
      timestamp: event.timestamp,
      tags: payload.payment_id ? [{ text: `#${payload.payment_id.slice(-5)}`, type: 'payment' as const }] : [],
    };
  }
};
