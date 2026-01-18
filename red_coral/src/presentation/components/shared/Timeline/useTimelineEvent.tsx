/* eslint-disable @typescript-eslint/no-explicit-any */
import React, { useMemo } from 'react';
import { TimelineEvent } from '@/core/domain/types';
import { OrderEventType } from '@/core/domain/events';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/formatCurrency';
import {
  Utensils, ShoppingBag, Coins, CheckCircle,
  Edit3, Trash2, Ban, Tag, LucideIcon, ArrowRight, ArrowLeft, Printer, Split
} from 'lucide-react';

// ============ Type Definitions ============

export interface TimelineDisplayData {
  title: string;
  summary?: string;
  details: string[];
  icon: LucideIcon | React.FC<any>;
  colorClass: string;
  customColor?: string;
  timestamp: number;
  isHidden?: boolean;
  tags?: string[];
}

// ============ Type-Safe Event Adapter ============

type TranslateFn = (key: string, params?: Record<string, string | number>) => string;

// ============ Helper Functions ============

function getNumber(value: unknown, defaultValue = 0): number {
  return typeof value === 'number' ? value : defaultValue;
}

function getString(value: unknown, defaultValue = ''): string {
  return typeof value === 'string' ? value : defaultValue;
}

function getArray<T>(value: unknown): T[] {
  return Array.isArray(value) ? (value as T[]) : [];
}

// ============ Adapter Registry ============

const adapters: Record<string, (event: TimelineEvent, t: TranslateFn) => TimelineDisplayData> = {
  // ============ Order Lifecycle ============
  [OrderEventType.TABLE_OPENED]: (event, t) => ({
    title: t('timeline.event.tableOrder'),
    summary: t('timeline.guestsCount', { n: getNumber(event.data?.guestCount, 1) }),
    details: [],
    icon: Utensils,
    colorClass: 'bg-blue-500',
    timestamp: event.timestamp,
  }),

  // ============ Item Operations ============
  [OrderEventType.ITEMS_ADDED]: (event, t) => {
    const items = getArray<{ name?: string; quantity?: number; selectedSpecification?: { name?: string }; discountPercent?: number; surcharge?: number }>(event.data?.items);
    const totalQty = items.reduce((sum, item) => sum + getNumber(item.quantity, 0), 0);
    const details = items.map((item) => {
      const qty = getNumber(item.quantity, 1);
      const name = getString(item.name, 'Unknown Item');
      const spec = item.selectedSpecification ? `(${item.selectedSpecification.name})` : '';
      const modifiers: string[] = [];
      if (item.discountPercent) modifiers.push(`-${item.discountPercent}%`);
      if (item.surcharge) modifiers.push(`+${formatCurrency(item.surcharge)}`);
      return `${name} ${spec} x${qty}${modifiers.length ? ` (${modifiers.join(', ')})` : ''}`;
    });
    return {
      title: t('timeline.event.addItems'),
      summary: t('timeline.addedItems', { n: totalQty }),
      details,
      icon: ShoppingBag,
      colorClass: 'bg-orange-500',
      timestamp: event.timestamp,
    };
  },

  [OrderEventType.ITEM_MODIFIED]: (event, t) => {
    const changes = event.data?.changes || {};
    const previousValues = event.data?.previousValues || {};
    const details: string[] = [];
    const formatChange = (key: string, labelKey: string, formatter: (v: unknown) => string = String) => {
      if ((changes as Record<string, unknown>)[key] !== undefined) {
        const oldVal = (previousValues as Record<string, unknown>)[key];
        const newVal = (changes as Record<string, unknown>)[key];
        if (oldVal !== newVal) {
          details.push(`${t(labelKey)}: ${oldVal !== undefined ? formatter(oldVal) : '0'} -> ${formatter(newVal)}`);
        }
      }
    };
    formatChange('price', 'timeline.labels.price');
    formatChange('quantity', 'timeline.labels.quantity');
    formatChange('discountPercent', 'timeline.labels.discount', v => `${v}%`);
    formatChange('surcharge', 'timeline.labels.surcharge');
    return {
      title: t('timeline.event.itemModified'),
      summary: getString(event.data?.itemName),
      details,
      icon: Edit3,
      colorClass: 'bg-yellow-500',
      timestamp: event.timestamp,
      tags: event.data?.instanceId ? [`#${String(event.data.instanceId).slice(-5)}`] : [],
    };
  },

  [OrderEventType.ITEM_REMOVED]: (event, t) => ({
    title: t('timeline.event.itemRemoved'),
    summary: getString(event.data?.itemName || event.data?.reason),
    details: [],
    icon: Trash2,
    colorClass: 'bg-red-500',
    timestamp: event.timestamp,
    tags: event.data?.instanceId ? [`#${String(event.data.instanceId).slice(-5)}`] : [],
  }),

  [OrderEventType.ITEM_RESTORED]: (event, t) => ({
    title: t('timeline.event.itemRestored'),
    summary: getString(event.data?.instanceId),
    details: [],
    icon: Utensils,
    colorClass: 'bg-green-400',
    timestamp: event.timestamp,
  }),

  // ============ Payment Operations ============
  [OrderEventType.PAYMENT_ADDED]: (event, t) => {
    const payment = event.data?.payment || event.data || {};
    const methodKey = `payment.${getString((payment as { method?: string }).method)}`;
    let methodDisplay = t(methodKey) !== methodKey ? t(methodKey) : getString((payment as { method?: string }).method);
    if (String(methodDisplay).toLowerCase() === 'cash') methodDisplay = t('checkout.method.cash');
    else if (String(methodDisplay).toLowerCase() === 'card') methodDisplay = t('checkout.method.card');
    const paymentTyped = payment as { method?: string; amount?: number; tendered?: number; change?: number; note?: string };
    const details = paymentTyped.tendered !== undefined
      ? [`${t('checkout.amount.tendered')}: ${formatCurrency(getNumber(paymentTyped.tendered))}`, `${t('checkout.amount.change')}: ${formatCurrency(getNumber(paymentTyped.change))}`]
      : [paymentTyped.note].filter(Boolean) as string[];
    return {
      title: `${t('timeline.event.payment')}: ${methodDisplay}`,
      summary: formatCurrency(getNumber(paymentTyped.amount, 0)),
      details,
      icon: Coins,
      colorClass: 'bg-green-500',
      timestamp: event.timestamp,
    };
  },

  [OrderEventType.PAYMENT_CANCELLED]: (event, t) => ({
    title: t('timeline.event.paymentCancelled'),
    summary: getString(event.data?.reason),
    details: [],
    icon: Ban,
    colorClass: 'bg-gray-500',
    timestamp: event.timestamp,
  }),

  [OrderEventType.ORDER_SPLIT]: (event, t) => ({
    title: t('timeline.event.splitBill'),
    summary: `${formatCurrency(getNumber(event.data?.splitAmount, 0))} (${getString(event.data?.paymentMethod)})`,
    details: getArray<{ name?: string; quantity?: number }>(event.data?.items).map((item) => `${getString(item.name)} x${getNumber(item.quantity, 1)}`),
    icon: Split,
    colorClass: 'bg-teal-500',
    timestamp: event.timestamp,
  }),

  // ============ Order State Changes ============
  [OrderEventType.ORDER_COMPLETED]: (event, t) => ({
    title: t('timeline.event.orderCompleted'),
    summary: event.data?.receiptNumber ? t('timeline.receiptNo', { n: getString(event.data.receiptNumber) }) : formatCurrency(getNumber(event.data?.finalTotal, 0)),
    details: [],
    icon: CheckCircle,
    colorClass: 'bg-green-600',
    timestamp: event.timestamp,
  }),

  [OrderEventType.ORDER_VOIDED]: (event, t) => ({
    title: t('timeline.event.orderVoided'),
    summary: getString(event.data?.reason),
    details: [],
    icon: Ban,
    colorClass: 'bg-red-700',
    timestamp: event.timestamp,
  }),

  [OrderEventType.ORDER_RESTORED]: (event, t) => ({
    title: t('timeline.event.orderRestored'),
    details: [],
    icon: CheckCircle,
    colorClass: 'bg-blue-400',
    timestamp: event.timestamp,
  }),

  [OrderEventType.ORDER_SURCHARGE_EXEMPT_SET]: (event, t) => ({
    title: t('timeline.surchargeExempt'),
    summary: event.data?.exempt ? t('common.yes') : t('common.no'),
    details: [],
    icon: Tag,
    colorClass: 'bg-purple-500',
    timestamp: event.timestamp,
  }),

  // ============ Table Operations ============
  [OrderEventType.ORDER_MERGED]: (event, t) => {
    const itemsLength = getNumber((event.data?.items as unknown[])?.length, 0);
    return {
      title: t("timeline.event.orderMerged"),
      summary: event.data?.sourceTableName ? t('timeline.fromTable', { n: getString(event.data.sourceTableName) }) : undefined,
      details: itemsLength > 0 ? [t('timeline.itemsMerged', { n: itemsLength })] : [],
      icon: ArrowLeft,
      colorClass: 'bg-indigo-500',
      timestamp: event.timestamp,
    };
  },

  [OrderEventType.ORDER_MOVED]: (event, t) => {
    const itemsLength = getNumber((event.data?.items as unknown[])?.length, 0);
    return {
      title: t("timeline.event.orderMoved"),
      summary: event.data?.targetTableName ? t('timeline.toTable', { n: getString(event.data.targetTableName) }) : undefined,
      details: itemsLength > 0 ? [t('timeline.itemsMoved', { n: itemsLength })] : [],
      icon: ArrowRight,
      colorClass: 'bg-cyan-500',
      timestamp: event.timestamp,
    };
  },

  [OrderEventType.ORDER_MOVED_OUT]: (event, t) => ({
    title: t("timeline.event.orderMovedOut"),
    summary: event.data?.targetTableName ? t('timeline.toTable', { n: getString(event.data.targetTableName) }) : undefined,
    details: event.data?.reason ? [getString(event.data.reason)] : [],
    icon: ArrowRight,
    colorClass: 'bg-cyan-600',
    timestamp: event.timestamp,
  }),

  [OrderEventType.ORDER_MERGED_OUT]: (event, t) => ({
    title: t("timeline.event.orderMergedOut"),
    summary: event.data?.targetTableName ? t('timeline.toTable', { n: getString(event.data.targetTableName) }) : undefined,
    details: event.data?.reason ? [getString(event.data.reason)] : [],
    icon: ArrowLeft,
    colorClass: 'bg-indigo-600',
    timestamp: event.timestamp,
  }),

  [OrderEventType.ORDER_INFO_UPDATED]: (event, t) => {
    if (event.data?.isPrePayment) {
      return {
        title: t("timeline.event.prePaymentPrinted"),
        summary: event.data?.receiptNumber ? t('timeline.receiptNo', { n: getString(event.data.receiptNumber) }) : undefined,
        details: [t("timeline.event.prePaymentNote")],
        icon: Printer,
        colorClass: 'bg-blue-600',
        timestamp: event.timestamp,
      };
    }
    return { title: 'Order Updated', details: [], icon: Edit3, colorClass: 'bg-gray-400', timestamp: event.timestamp, isHidden: true };
  },
};

// ============ Main Hook ============

export const useTimelineEvent = (event: TimelineEvent, _showHidden?: boolean): TimelineDisplayData => {
  const { t } = useI18n();

  return useMemo(() => {
    const type = event.type as string;
    const adapter = adapters[type];

    if (adapter) {
      return adapter(event, t);
    }

    // Fallback for unknown/legacy events
    const typeStr = String(type).toLowerCase().replace(/_/g, ' ');
    return {
      title: event.title || typeStr.charAt(0).toUpperCase() + typeStr.slice(1),
      summary: event.summary,
      details: [],
      icon: Utensils,
      colorClass: 'bg-gray-500',
      timestamp: event.timestamp,
    };
  }, [event, t]);
};

export default useTimelineEvent;
