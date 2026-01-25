/**
 * Timeline Event Renderers
 *
 * 职责分离设计：
 * - 每个 OrderEvent Payload 有独立的 Renderer
 * - Renderer 负责将事件数据转换为 UI 展示数据
 * - 通过注册表映射，无需 switch case
 *
 * 类似 Rust trait 的实现方式
 */

import type { OrderEvent, OrderEventType } from '@/core/domain/types/orderEvent';
import type {
  TableOpenedPayload,
  ItemsAddedPayload,
  ItemModifiedPayload,
  ItemRemovedPayload,
  ItemRestoredPayload,
  PaymentAddedPayload,
  PaymentCancelledPayload,
  OrderCompletedPayload,
  OrderVoidedPayload,
  OrderRestoredPayload,
  OrderSplitPayload,
  OrderMergedPayload,
  OrderMovedPayload,
  OrderMovedOutPayload,
  OrderMergedOutPayload,
  TableReassignedPayload,
  OrderInfoUpdatedPayload,
  RuleSkipToggledPayload,
} from '@/core/domain/types/orderEvent';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import {
  Utensils, ShoppingBag, Coins, CheckCircle,
  Edit3, Trash2, Ban, Tag, ArrowRight, ArrowLeft, Split
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

// ============================================================================
// Types
// ============================================================================

type TranslateFn = (key: string, params?: Record<string, string | number>) => string;

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

/**
 * Event Renderer Interface (类似 Rust trait)
 *
 * 每个 Payload 类型实现这个接口，定义如何渲染
 */
interface EventRenderer<T> {
  render(event: OrderEvent, payload: T, t: TranslateFn): TimelineDisplayData;
}

// ============================================================================
// Renderer Implementations
// ============================================================================

const TableOpenedRenderer: EventRenderer<TableOpenedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.table_order'),
      summary: t('timeline.guests_count', { n: payload.guest_count }),
      details: [],
      icon: Utensils,
      colorClass: 'bg-blue-500',
      timestamp: event.timestamp,
    };
  }
};

const ItemsAddedRenderer: EventRenderer<ItemsAddedPayload> = {
  render(event, payload, t) {
    const items = payload.items;
    const totalQty = items.reduce((sum, item) => sum + item.quantity, 0);
    const details = items.map((item) => {
      const spec = item.selected_specification ? `(${item.selected_specification.name})` : '';
      const modifiers: string[] = [];
      if (item.manual_discount_percent) modifiers.push(`-${item.manual_discount_percent}%`);
      if (item.surcharge) modifiers.push(`+${formatCurrency(item.surcharge)}`);
      return `${item.name} ${spec} x${item.quantity}${modifiers.length ? ` (${modifiers.join(', ')})` : ''}`;
    });

    return {
      title: t('timeline.add_items'),
      summary: t('timeline.added_items', { n: totalQty }),
      details,
      icon: ShoppingBag,
      colorClass: 'bg-orange-500',
      timestamp: event.timestamp,
    };
  }
};

const ItemModifiedRenderer: EventRenderer<ItemModifiedPayload> = {
  render(event, payload, t) {
    const changes = payload.changes;
    const previousValues = payload.previous_values || {};
    const details: string[] = [];

    const formatChange = (
      key: keyof typeof changes,
      labelKey: string,
      formatter: (v: any) => string = String
    ) => {
      if (changes[key] !== undefined && changes[key] !== null) {
        const oldVal = previousValues[key];
        const newVal = changes[key];
        if (oldVal !== newVal) {
          details.push(
            `${t(labelKey)}: ${oldVal !== undefined ? formatter(oldVal) : '0'} -> ${formatter(newVal)}`
          );
        }
      }
    };

    formatChange('price', 'timeline.labels.price', v => formatCurrency(v || 0));
    formatChange('quantity', 'timeline.labels.quantity');
    formatChange('manual_discount_percent', 'timeline.labels.discount', v => `${v}%`);
    formatChange('surcharge', 'timeline.labels.surcharge', v => formatCurrency(v || 0));

    return {
      title: t('timeline.item_modified'),
      summary: payload.source.name || '',
      details,
      icon: Edit3,
      colorClass: 'bg-yellow-500',
      timestamp: event.timestamp,
      tags: payload.source.instance_id ? [`#${payload.source.instance_id.slice(-5)}`] : [],
    };
  }
};

const ItemRemovedRenderer: EventRenderer<ItemRemovedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.item_removed'),
      summary: payload.item_name || payload.reason || '',
      details: [],
      icon: Trash2,
      colorClass: 'bg-red-500',
      timestamp: event.timestamp,
      tags: payload.instance_id ? [`#${payload.instance_id.slice(-5)}`] : [],
    };
  }
};

const ItemRestoredRenderer: EventRenderer<ItemRestoredPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.item_restored'),
      summary: payload.instance_id,
      details: [],
      icon: Utensils,
      colorClass: 'bg-green-400',
      timestamp: event.timestamp,
    };
  }
};

const PaymentAddedRenderer: EventRenderer<PaymentAddedPayload> = {
  render(event, payload, t) {
    const methodKey = `payment.${payload.method}`;
    let methodDisplay = t(methodKey) !== methodKey ? t(methodKey) : payload.method;

    if (methodDisplay.toLowerCase() === 'cash') methodDisplay = t('checkout.method.cash');
    else if (methodDisplay.toLowerCase() === 'card') methodDisplay = t('checkout.method.card');

    const details = payload.tendered !== undefined && payload.tendered !== null
      ? [
          `${t('checkout.amount.tendered')}: ${formatCurrency(payload.tendered)}`,
          `${t('checkout.amount.change')}: ${formatCurrency(payload.change || 0)}`
        ]
      : payload.note ? [payload.note] : [];

    return {
      title: `${t('timeline.payment')}: ${methodDisplay}`,
      summary: formatCurrency(payload.amount),
      details,
      icon: Coins,
      colorClass: 'bg-green-500',
      timestamp: event.timestamp,
    };
  }
};

const PaymentCancelledRenderer: EventRenderer<PaymentCancelledPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.payment_cancelled'),
      summary: payload.reason || '',
      details: [],
      icon: Ban,
      colorClass: 'bg-gray-500',
      timestamp: event.timestamp,
    };
  }
};

const OrderSplitRenderer: EventRenderer<OrderSplitPayload> = {
  render(event, payload, t) {
    const details = payload.items.map(item =>
      `${item.name} x${item.quantity}`
    );

    return {
      title: t('timeline.split_bill'),
      summary: `${formatCurrency(payload.split_amount)} (${payload.payment_method})`,
      details,
      icon: Split,
      colorClass: 'bg-teal-500',
      timestamp: event.timestamp,
    };
  }
};

const OrderCompletedRenderer: EventRenderer<OrderCompletedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.order_completed'),
      summary: payload.receipt_number
        ? t('timeline.receipt_no', { n: payload.receipt_number })
        : formatCurrency(payload.final_total),
      details: [],
      icon: CheckCircle,
      colorClass: 'bg-green-600',
      timestamp: event.timestamp,
    };
  }
};

const OrderVoidedRenderer: EventRenderer<OrderVoidedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.order_voided'),
      summary: payload.reason || '',
      details: [],
      icon: Ban,
      colorClass: 'bg-red-700',
      timestamp: event.timestamp,
    };
  }
};

const OrderRestoredRenderer: EventRenderer<OrderRestoredPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.order_restored'),
      details: [],
      icon: CheckCircle,
      colorClass: 'bg-blue-400',
      timestamp: event.timestamp,
    };
  }
};

const OrderMergedRenderer: EventRenderer<OrderMergedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.order_merged'),
      summary: `${t('timeline.from')} ${payload.source_table_name}`,
      details: [],
      icon: ArrowLeft,
      colorClass: 'bg-purple-500',
      timestamp: event.timestamp,
    };
  }
};

const OrderMovedRenderer: EventRenderer<OrderMovedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.table_moved'),
      summary: `${payload.source_table_name} → ${payload.target_table_name}`,
      details: [],
      icon: ArrowRight,
      colorClass: 'bg-indigo-500',
      timestamp: event.timestamp,
    };
  }
};

const OrderMovedOutRenderer: EventRenderer<OrderMovedOutPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.moved_out'),
      summary: `${t('timeline.to')} ${payload.target_table_name}`,
      details: [],
      icon: ArrowRight,
      colorClass: 'bg-indigo-600',
      timestamp: event.timestamp,
    };
  }
};

const OrderMergedOutRenderer: EventRenderer<OrderMergedOutPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.merged_out'),
      summary: `${t('timeline.to')} ${payload.target_table_name}`,
      details: [],
      icon: ArrowRight,
      colorClass: 'bg-purple-600',
      timestamp: event.timestamp,
    };
  }
};

const TableReassignedRenderer: EventRenderer<TableReassignedPayload> = {
  render(event, payload, t) {
    return {
      title: t('timeline.table_reassigned'),
      summary: `${payload.source_table_name} → ${payload.target_table_name}`,
      details: [],
      icon: ArrowRight,
      colorClass: 'bg-blue-600',
      timestamp: event.timestamp,
    };
  }
};

const OrderInfoUpdatedRenderer: EventRenderer<OrderInfoUpdatedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];
    if (payload.receipt_number) details.push(`${t('timeline.labels.receipt')}: ${payload.receipt_number}`);
    if (payload.guest_count) details.push(`${t('timeline.labels.guests')}: ${payload.guest_count}`);
    if (payload.table_name) details.push(`${t('timeline.labels.table')}: ${payload.table_name}`);

    return {
      title: t('timeline.order_info_updated'),
      details,
      icon: Edit3,
      colorClass: 'bg-blue-400',
      timestamp: event.timestamp,
    };
  }
};

const RuleSkipToggledRenderer: EventRenderer<RuleSkipToggledPayload> = {
  render(event, payload, t) {
    const action = payload.skipped ? 'Skipped' : 'Applied';

    return {
      title: 'Price Rule Toggled',
      summary: `${action}: ${payload.rule_id}`,
      details: [
        `Total: ${formatCurrency(payload.total)}`,
      ],
      icon: Tag,
      colorClass: payload.skipped ? 'bg-orange-400' : 'bg-green-400',
      timestamp: event.timestamp,
    };
  }
};

// ============================================================================
// Renderer Registry (类似 Rust trait object dispatch)
// ============================================================================

/**
 * 事件渲染器注册表
 *
 * 自动映射 OrderEventType → Renderer，无需 switch case
 * 新增事件类型只需在这里添加一行
 */
export const EVENT_RENDERERS: Record<OrderEventType, EventRenderer<any>> = {
  TABLE_OPENED: TableOpenedRenderer,
  ITEMS_ADDED: ItemsAddedRenderer,
  ITEM_MODIFIED: ItemModifiedRenderer,
  ITEM_REMOVED: ItemRemovedRenderer,
  ITEM_RESTORED: ItemRestoredRenderer,
  PAYMENT_ADDED: PaymentAddedRenderer,
  PAYMENT_CANCELLED: PaymentCancelledRenderer,
  ORDER_SPLIT: OrderSplitRenderer,
  ORDER_COMPLETED: OrderCompletedRenderer,
  ORDER_VOIDED: OrderVoidedRenderer,
  ORDER_RESTORED: OrderRestoredRenderer,
  ORDER_MERGED: OrderMergedRenderer,
  ORDER_MOVED: OrderMovedRenderer,
  ORDER_MOVED_OUT: OrderMovedOutRenderer,
  ORDER_MERGED_OUT: OrderMergedOutRenderer,
  TABLE_REASSIGNED: TableReassignedRenderer,
  ORDER_INFO_UPDATED: OrderInfoUpdatedRenderer,
  SURCHARGE_EXEMPT_SET: OrderInfoUpdatedRenderer, // Reuse for now
  RULE_SKIP_TOGGLED: RuleSkipToggledRenderer,
};

/**
 * 渲染单个事件
 *
 * @param event - OrderEvent（服务端权威类型）
 * @param t - 翻译函数
 * @returns TimelineDisplayData - UI 展示数据
 */
export function renderEvent(event: OrderEvent, t: TranslateFn): TimelineDisplayData {
  const renderer = EVENT_RENDERERS[event.event_type];

  if (!renderer) {
    // Fallback for unknown event types
    return {
      title: event.event_type,
      summary: '',
      details: [],
      icon: Tag,
      colorClass: 'bg-gray-400',
      timestamp: event.timestamp,
    };
  }

  return renderer.render(event, event.payload, t);
}
