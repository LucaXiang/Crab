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
    const details: string[] = [];

    // Show table and zone info
    if (payload.table_name) {
      details.push(`${t('timeline.labels.table')}: ${payload.table_name}`);
    }
    if (payload.zone_name) {
      details.push(`${t('timeline.labels.zone')}: ${payload.zone_name}`);
    }
    // Show receipt number (server-generated)
    if (payload.receipt_number) {
      details.push(`${t('timeline.labels.receipt')}: ${payload.receipt_number}`);
    }
    // Show retail mode
    if (payload.is_retail) {
      details.push(t('timeline.retail_mode'));
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

const ItemsAddedRenderer: EventRenderer<ItemsAddedPayload> = {
  render(event, payload, t) {
    const items = payload.items || [];
    const totalQty = items.reduce((sum, item) => sum + item.quantity, 0);
    const details = items.map((item) => {
      const instanceId = item.instance_id ? `#${item.instance_id.slice(-5)}` : '';
      const modifiers: string[] = [];
      if (item.manual_discount_percent) modifiers.push(`-${item.manual_discount_percent}%`);
      if (item.surcharge) modifiers.push(`+${formatCurrency(item.surcharge)}`);
      return `${item.name} ${instanceId} x${item.quantity}${modifiers.length ? ` (${modifiers.join(', ')})` : ''}`;
    });

    return {
      title: t('timeline.add_items'),
      summary: items.length > 0 ? t('timeline.added_items', { n: totalQty }) : '',
      details,
      icon: ShoppingBag,
      colorClass: 'bg-orange-500',
      timestamp: event.timestamp,
    };
  }
};

const ItemModifiedRenderer: EventRenderer<ItemModifiedPayload> = {
  render(event, payload, t) {
    const changes = payload.changes || {};
    const previousValues = payload.previous_values || {};
    const details: string[] = [];

    // Show operation description
    if (payload.operation) {
      details.push(payload.operation);
    }

    // Show affected quantity
    if (payload.affected_quantity != null && payload.affected_quantity > 0) {
      details.push(`${t('timeline.labels.affected_quantity')}: ${payload.affected_quantity}`);
    }

    const formatChange = (
      key: keyof typeof changes,
      labelKey: string,
      formatter: (v: number) => string = String
    ) => {
      const newVal = changes[key];
      if (typeof newVal === 'number') {
        const oldVal = previousValues[key];
        if (oldVal !== newVal) {
          details.push(
            `${t(labelKey)}: ${typeof oldVal === 'number' ? formatter(oldVal) : '0'} → ${formatter(newVal)}`
          );
        }
      }
    };

    formatChange('price', 'timeline.labels.price', v => formatCurrency(v || 0));
    formatChange('quantity', 'timeline.labels.quantity');
    formatChange('manual_discount_percent', 'timeline.labels.discount', v => `${v}%`);
    formatChange('surcharge', 'timeline.labels.surcharge', v => formatCurrency(v || 0));

    // Show authorizer
    if (payload.authorizer_name) {
      details.push(`${t('timeline.labels.authorizer')}: ${payload.authorizer_name}`);
    }

    const source = payload.source;
    return {
      title: t('timeline.item_modified'),
      summary: source?.name || '',
      details,
      icon: Edit3,
      colorClass: 'bg-yellow-500',
      timestamp: event.timestamp,
      tags: source?.instance_id ? [`#${source.instance_id.slice(-5)}`] : [],
    };
  }
};

const ItemRemovedRenderer: EventRenderer<ItemRemovedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    // Show quantity if available
    if (payload.quantity != null && payload.quantity > 0) {
      details.push(`${t('timeline.labels.quantity')}: ${payload.quantity}`);
    }

    // Show reason if available
    if (payload.reason) {
      details.push(`${t('timeline.labels.reason')}: ${payload.reason}`);
    }

    // Show authorizer
    if (payload.authorizer_name) {
      details.push(`${t('timeline.labels.authorizer')}: ${payload.authorizer_name}`);
    }

    return {
      title: t('timeline.item_removed'),
      summary: payload.item_name || '',
      details,
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
      summary: payload.item_name || '',
      details: [],
      icon: Utensils,
      colorClass: 'bg-green-400',
      timestamp: event.timestamp,
      tags: payload.instance_id ? [`#${payload.instance_id.slice(-5)}`] : [],
    };
  }
};

const PaymentAddedRenderer: EventRenderer<PaymentAddedPayload> = {
  render(event, payload, t) {
    const method = payload.method || 'unknown';
    const methodLower = method.toLowerCase();
    let methodDisplay = method;
    if (methodLower === 'cash') {
      methodDisplay = t('checkout.method.cash');
    } else if (methodLower === 'card') {
      methodDisplay = t('checkout.method.card');
    }

    const details: string[] = [];

    // Show tendered/change for cash payments
    if (payload.tendered !== undefined && payload.tendered !== null) {
      details.push(`${t('checkout.amount.tendered')}: ${formatCurrency(payload.tendered)}`);
      details.push(`${t('checkout.amount.change')}: ${formatCurrency(payload.change || 0)}`);
    }

    // Show note if available
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
      tags: payload.payment_id ? [`#${payload.payment_id.slice(-6)}`] : [],
    };
  }
};

const PaymentCancelledRenderer: EventRenderer<PaymentCancelledPayload> = {
  render(event, payload, t) {
    // Format payment method display
    const methodKey = `checkout.method.${payload.method?.toLowerCase() || 'other'}`;
    const methodDisplay = t(methodKey);

    // Build details array
    const details: string[] = [];

    // Show cancelled amount
    if (payload.amount != null) {
      details.push(`${t('checkout.amount.paid')}: ${formatCurrency(payload.amount)}`);
    }

    // Show reason if available
    if (payload.reason) {
      details.push(`${t('timeline.labels.reason')}: ${payload.reason}`);
    }

    // Show authorizer
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
      tags: payload.payment_id ? [`#${payload.payment_id.slice(-6)}`] : [],
    };
  }
};

const OrderSplitRenderer: EventRenderer<OrderSplitPayload> = {
  render(event, payload, t) {
    const items = payload.items || [];
    const details = items.map(item => {
      const instanceId = item.instance_id ? `#${item.instance_id.slice(-5)}` : '';
      return `${item.name} ${instanceId} x${item.quantity}`;
    });

    let methodDisplay = payload.payment_method || '';
    const methodLower = methodDisplay.toLowerCase();
    if (methodLower === 'cash') {
      methodDisplay = t('checkout.method.cash');
    } else if (methodLower === 'card') {
      methodDisplay = t('checkout.method.card');
    }

    return {
      title: t('timeline.split_bill'),
      summary: payload.split_amount != null
        ? `${formatCurrency(payload.split_amount)} (${methodDisplay})`
        : '',
      details,
      icon: Split,
      colorClass: 'bg-teal-500',
      timestamp: event.timestamp,
    };
  }
};

const OrderCompletedRenderer: EventRenderer<OrderCompletedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    // Show final total
    if (payload.final_total != null) {
      details.push(`${t('timeline.labels.total')}: ${formatCurrency(payload.final_total)}`);
    }

    // Show payment summary
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

const OrderVoidedRenderer: EventRenderer<OrderVoidedPayload> = {
  render(event, payload, t) {
    const isLossSettled = payload.void_type === 'LOSS_SETTLED';
    const summary = isLossSettled
      ? t('checkout.void.type.loss_settled')
      : t('checkout.void.type.cancelled');
    const details: string[] = [];

    // Show loss reason
    if (payload.loss_reason) {
      const reasonKey = `checkout.void.loss_reason.${payload.loss_reason.toLowerCase()}`;
      details.push(`${t('timeline.labels.reason')}: ${t(reasonKey)}`);
    }

    // Show loss amount (for tax reporting)
    if (payload.loss_amount != null) {
      details.push(`${t('timeline.labels.loss_amount')}: ${formatCurrency(payload.loss_amount)}`);
    }

    // Show note
    if (payload.note) {
      details.push(payload.note);
    }

    // Show authorizer
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
    const details: string[] = [];

    // Show merged items
    if (payload.items && payload.items.length > 0) {
      const itemCount = payload.items.reduce((sum, item) => sum + item.quantity, 0);
      details.push(`${t('timeline.labels.items')}: ${itemCount}`);
    }

    return {
      title: t('timeline.order_merged'),
      summary: payload.source_table_name ? `${t('timeline.from')} ${payload.source_table_name}` : '',
      details,
      icon: ArrowLeft,
      colorClass: 'bg-purple-500',
      timestamp: event.timestamp,
    };
  }
};

const OrderMovedRenderer: EventRenderer<OrderMovedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    // Show moved items count
    if (payload.items && payload.items.length > 0) {
      const itemCount = payload.items.reduce((sum, item) => sum + item.quantity, 0);
      details.push(`${t('timeline.labels.items')}: ${itemCount}`);
    }

    return {
      title: t('timeline.table_moved'),
      summary: (payload.source_table_name && payload.target_table_name)
        ? `${payload.source_table_name} → ${payload.target_table_name}`
        : '',
      details,
      icon: ArrowRight,
      colorClass: 'bg-indigo-500',
      timestamp: event.timestamp,
    };
  }
};

const OrderMovedOutRenderer: EventRenderer<OrderMovedOutPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    // Show reason if available
    if (payload.reason) {
      details.push(`${t('timeline.labels.reason')}: ${payload.reason}`);
    }

    return {
      title: t('timeline.moved_out'),
      summary: payload.target_table_name ? `${t('timeline.to')} ${payload.target_table_name}` : '',
      details,
      icon: ArrowRight,
      colorClass: 'bg-indigo-600',
      timestamp: event.timestamp,
    };
  }
};

const OrderMergedOutRenderer: EventRenderer<OrderMergedOutPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    // Show reason if available
    if (payload.reason) {
      details.push(`${t('timeline.labels.reason')}: ${payload.reason}`);
    }

    return {
      title: t('timeline.merged_out'),
      summary: payload.target_table_name ? `${t('timeline.to')} ${payload.target_table_name}` : '',
      details,
      icon: ArrowRight,
      colorClass: 'bg-purple-600',
      timestamp: event.timestamp,
    };
  }
};

const TableReassignedRenderer: EventRenderer<TableReassignedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    // Show target zone if available
    if (payload.target_zone_name) {
      details.push(`${t('timeline.labels.zone')}: ${payload.target_zone_name}`);
    }

    // Show items count
    if (payload.items && payload.items.length > 0) {
      const itemCount = payload.items.reduce((sum, item) => sum + item.quantity, 0);
      details.push(`${t('timeline.labels.items')}: ${itemCount}`);
    }

    return {
      title: t('timeline.table_reassigned'),
      summary: (payload.source_table_name && payload.target_table_name)
        ? `${payload.source_table_name} → ${payload.target_table_name}`
        : '',
      details,
      icon: ArrowRight,
      colorClass: 'bg-blue-600',
      timestamp: event.timestamp,
    };
  }
};

const OrderInfoUpdatedRenderer: EventRenderer<OrderInfoUpdatedPayload> = {
  render(event, payload, t) {
    // Note: receipt_number is immutable (set at OpenTable), not updatable
    const details: string[] = [];
    // Use != null to check for both null and undefined, allowing 0 values
    if (payload.guest_count != null) {
      details.push(`${t('timeline.labels.guests')}: ${payload.guest_count}`);
    }
    if (payload.table_name != null) {
      details.push(`${t('timeline.labels.table')}: ${payload.table_name}`);
    }
    if (payload.is_pre_payment != null) {
      details.push(`${t('timeline.labels.pre_payment')}: ${payload.is_pre_payment ? t('common.yes') : t('common.no')}`);
    }

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
    const actionLabel = payload.skipped
      ? t('timeline.rule_skipped')
      : t('timeline.rule_applied');

    const details: string[] = [];

    // Show recalculated amounts
    if (payload.subtotal != null) {
      details.push(`${t('timeline.labels.subtotal')}: ${formatCurrency(payload.subtotal)}`);
    }
    if (payload.discount != null && payload.discount !== 0) {
      details.push(`${t('timeline.labels.discount')}: -${formatCurrency(payload.discount)}`);
    }
    if (payload.surcharge != null && payload.surcharge !== 0) {
      details.push(`${t('timeline.labels.surcharge')}: +${formatCurrency(payload.surcharge)}`);
    }
    if (payload.total != null) {
      details.push(`${t('timeline.labels.total')}: ${formatCurrency(payload.total)}`);
    }

    return {
      title: t('timeline.rule_toggled'),
      summary: `${actionLabel}: ${payload.rule_id}`,
      details,
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
