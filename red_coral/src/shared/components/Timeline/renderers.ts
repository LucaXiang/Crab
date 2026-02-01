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
  ItemCompedPayload,
  ItemUncompedPayload,
  PaymentAddedPayload,
  PaymentCancelledPayload,
  OrderCompletedPayload,
  OrderVoidedPayload,
  ItemSplitPayload,
  AmountSplitPayload,
  AaSplitStartedPayload,
  AaSplitPaidPayload,
  AaSplitCancelledPayload,
  OrderMergedPayload,
  OrderMovedPayload,
  OrderMovedOutPayload,
  OrderMergedOutPayload,
  TableReassignedPayload,
  OrderInfoUpdatedPayload,
  RuleSkipToggledPayload,
  OrderDiscountAppliedPayload,
  OrderSurchargeAppliedPayload,
  OrderNoteAddedPayload,
} from '@/core/domain/types/orderEvent';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import {
  Utensils, ShoppingBag, Coins, CheckCircle,
  Edit3, Trash2, Ban, Tag, ArrowRight, ArrowLeft, Split, Users, XCircle
} from 'lucide-react';
import type { LucideIcon } from 'lucide-react';

// ============================================================================
// Types
// ============================================================================

type TranslateFn = (key: string, params?: Record<string, string | number>) => string;

export interface TimelineTag {
  text: string;
  type: 'item' | 'payment';
}

export interface TimelineDisplayData {
  title: string;
  summary?: string;
  details: string[];
  icon: LucideIcon | React.FC<any>;
  colorClass: string;
  customColor?: string;
  timestamp: number;
  isHidden?: boolean;
  tags?: TimelineTag[];
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
      return `${instanceId} ${item.name} x${item.quantity}${modifiers.length ? ` (${modifiers.join(', ')})` : ''}`;
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

    // Only show key field changes (old → new)
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

    // Show specification change
    if (changes.selected_specification) {
      const oldSpec = previousValues.selected_specification;
      const oldName = oldSpec && typeof oldSpec === 'object' && 'name' in oldSpec ? (oldSpec as any).name : '-';
      details.push(`${t('pos.cart.spec')}: ${oldName} → ${changes.selected_specification.name}`);
    }

    // Show options change (diff by attribute: added/removed)
    if (changes.selected_options) {
      const oldOpts = (previousValues.selected_options as any[] | undefined) || [];
      const newOpts = changes.selected_options;

      const groupByAttr = (opts: any[]) => {
        const map = new Map<string, Set<string>>();
        for (const o of opts) {
          const attr = o.attribute_name || '';
          if (!map.has(attr)) map.set(attr, new Set());
          map.get(attr)!.add(o.option_name);
        }
        return map;
      };

      const oldByAttr = groupByAttr(oldOpts);
      const newByAttr = groupByAttr(newOpts);
      const allAttrs = new Set([...oldByAttr.keys(), ...newByAttr.keys()]);

      for (const attr of allAttrs) {
        const oldSet = oldByAttr.get(attr) || new Set();
        const newSet = newByAttr.get(attr) || new Set();
        for (const name of newSet) {
          if (!oldSet.has(name)) {
            details.push(`${attr} ${t('timeline.option_added')} ${name}`);
          }
        }
        for (const name of oldSet) {
          if (!newSet.has(name)) {
            details.push(`${attr} ${t('timeline.option_removed')} ${name}`);
          }
        }
      }
    }

    // Show authorizer only if different from operator
    if (payload.authorizer_name && payload.authorizer_name !== event.operator_name) {
      details.push(`${t('timeline.labels.authorizer')}: ${payload.authorizer_name}`);
    }

    const source = payload.source;

    // Build tags: old instance_id → new instance_id (if changed)
    const tags: TimelineTag[] = [];
    if (source?.instance_id) {
      tags.push({ text: `#${source.instance_id.slice(-5)}`, type: 'item' as const });
    }
    const updatedResult = payload.results?.find(r => r.action === 'UPDATED' || r.action === 'CREATED');
    if (updatedResult && updatedResult.instance_id !== source?.instance_id) {
      tags.push({ text: `→ #${updatedResult.instance_id.slice(-5)}`, type: 'item' as const });
    }

    const summary = source?.name || '';

    return {
      title: t('timeline.item_modified'),
      summary,
      details,
      icon: Edit3,
      colorClass: 'bg-yellow-500',
      timestamp: event.timestamp,
      tags,
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
      tags: payload.instance_id ? [{ text: `#${payload.instance_id.slice(-5)}`, type: 'item' as const }] : [],
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
      tags: payload.instance_id ? [{ text: `#${payload.instance_id.slice(-5)}`, type: 'item' as const }] : [],
    };
  }
};

const ItemCompedRenderer: EventRenderer<ItemCompedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    // Show quantity
    if (payload.quantity != null && payload.quantity > 0) {
      details.push(`${t('timeline.labels.quantity')}: ${payload.quantity}`);
    }

    // Show reason
    if (payload.reason) {
      details.push(`${t('timeline.labels.reason')}: ${payload.reason}`);
    }

    // Show authorizer
    if (payload.authorizer_name) {
      details.push(`${t('timeline.labels.authorizer')}: ${payload.authorizer_name}`);
    }

    return {
      title: t('timeline.item_comped'),
      summary: payload.item_name || '',
      details,
      icon: Tag,
      colorClass: 'bg-emerald-500',
      timestamp: event.timestamp,
      tags: payload.instance_id ? [{ text: `#${payload.instance_id.slice(-5)}`, type: 'item' as const }] : [],
    };
  }
};

const ItemUncompedRenderer: EventRenderer<ItemUncompedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    // Show restored price
    if (payload.restored_price != null) {
      details.push(`${t('timeline.labels.price')}: ${formatCurrency(payload.restored_price)}`);
    }

    // Show merge info
    if (payload.merged_into) {
      details.push(`${t('timeline.merged_back')}: #${payload.merged_into.slice(-5)}`);
    }

    // Show authorizer
    if (payload.authorizer_name) {
      details.push(`${t('timeline.labels.authorizer')}: ${payload.authorizer_name}`);
    }

    return {
      title: t('timeline.item_uncomped'),
      summary: payload.item_name || '',
      details,
      icon: Tag,
      colorClass: 'bg-amber-500',
      timestamp: event.timestamp,
      tags: payload.instance_id ? [{ text: `#${payload.instance_id.slice(-5)}`, type: 'item' as const }] : [],
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
      tags: payload.payment_id ? [{ text: `#${payload.payment_id.slice(-5)}`, type: 'payment' as const }] : [],
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
      tags: payload.payment_id ? [{ text: `#${payload.payment_id.slice(-5)}`, type: 'payment' as const }] : [],
    };
  }
};

// ---- Split Renderers ----

function formatPaymentMethod(method: string, t: TranslateFn): string {
  const lower = method.toLowerCase();
  if (lower === 'cash') return t('checkout.method.cash');
  if (lower === 'card') return t('checkout.method.card');
  return method;
}

const ItemSplitRenderer: EventRenderer<ItemSplitPayload> = {
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
        ? `${formatCurrency(payload.split_amount)} (${methodDisplay})`
        : '',
      details,
      icon: Split,
      colorClass: 'bg-teal-500',
      timestamp: event.timestamp,
      tags: payload.payment_id ? [{ text: `#${payload.payment_id.slice(-5)}`, type: 'payment' as const }] : [],
    };
  }
};

const AmountSplitRenderer: EventRenderer<AmountSplitPayload> = {
  render(event, payload, t) {
    const methodDisplay = formatPaymentMethod(payload.payment_method || '', t);

    return {
      title: t('timeline.amount_split'),
      summary: payload.split_amount != null
        ? `${formatCurrency(payload.split_amount)} (${methodDisplay})`
        : '',
      details: [],
      icon: Split,
      colorClass: 'bg-teal-500',
      timestamp: event.timestamp,
      tags: payload.payment_id ? [{ text: `#${payload.payment_id.slice(-5)}`, type: 'payment' as const }] : [],
    };
  }
};

const AaSplitStartedRenderer: EventRenderer<AaSplitStartedPayload> = {
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

const AaSplitPaidRenderer: EventRenderer<AaSplitPaidPayload> = {
  render(event, payload, t) {
    const methodDisplay = formatPaymentMethod(payload.payment_method || '', t);
    const progress = (payload.progress_paid != null && payload.progress_total != null)
      ? ` ${payload.progress_paid}/${payload.progress_total}`
      : '';

    return {
      title: t('timeline.aa_split_paid'),
      summary: payload.amount != null
        ? `${formatCurrency(payload.amount)} (${methodDisplay})${progress}`
        : '',
      details: [],
      icon: Users,
      colorClass: 'bg-cyan-600',
      timestamp: event.timestamp,
      tags: payload.payment_id ? [{ text: `#${payload.payment_id.slice(-5)}`, type: 'payment' as const }] : [],
    };
  }
};

const AaSplitCancelledRenderer: EventRenderer<AaSplitCancelledPayload> = {
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

const OrderDiscountAppliedRenderer: EventRenderer<OrderDiscountAppliedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];
    const isClearing = !payload.discount_percent && !payload.discount_fixed;

    if (payload.discount_percent != null) {
      details.push(`${t('timeline.labels.discount')}: ${payload.discount_percent}%`);
    }
    if (payload.discount_fixed != null) {
      details.push(`${t('timeline.labels.discount')}: ${formatCurrency(payload.discount_fixed)}`);
    }
    details.push(`${t('timeline.labels.subtotal')}: ${formatCurrency(payload.subtotal)}`);
    if (payload.discount !== 0) {
      details.push(`${t('timeline.labels.discount')}: -${formatCurrency(payload.discount)}`);
    }
    details.push(`${t('timeline.labels.total')}: ${formatCurrency(payload.total)}`);

    return {
      title: isClearing ? t('timeline.discount_cleared') : t('timeline.discount_applied'),
      summary: payload.reason ?? undefined,
      details,
      icon: Tag,
      colorClass: isClearing ? 'bg-gray-400' : 'bg-blue-400',
      timestamp: event.timestamp,
    };
  }
};

const OrderSurchargeAppliedRenderer: EventRenderer<OrderSurchargeAppliedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];
    const isClearing = payload.surcharge_amount == null;

    if (payload.surcharge_amount != null) {
      details.push(`${t('timeline.labels.surcharge')}: +${formatCurrency(payload.surcharge_amount)}`);
    }
    details.push(`${t('timeline.labels.subtotal')}: ${formatCurrency(payload.subtotal)}`);
    if (payload.surcharge !== 0) {
      details.push(`${t('timeline.labels.surcharge')}: +${formatCurrency(payload.surcharge)}`);
    }
    details.push(`${t('timeline.labels.total')}: ${formatCurrency(payload.total)}`);

    return {
      title: isClearing ? t('timeline.surcharge_cleared') : t('timeline.surcharge_applied'),
      summary: payload.reason ?? undefined,
      details,
      icon: Tag,
      colorClass: isClearing ? 'bg-gray-400' : 'bg-yellow-400',
      timestamp: event.timestamp,
    };
  }
};

const OrderNoteAddedRenderer: EventRenderer<OrderNoteAddedPayload> = {
  render(event, payload, t) {
    const isClearing = payload.note === '';
    const details: string[] = [];

    if (!isClearing) {
      details.push(payload.note);
    }
    if (payload.previous_note) {
      details.push(`${t('timeline.labels.previous')}: ${payload.previous_note}`);
    }

    return {
      title: isClearing ? t('timeline.note_cleared') : t('timeline.note_added'),
      details,
      icon: Edit3,
      colorClass: 'bg-blue-400',
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
  ITEM_COMPED: ItemCompedRenderer,
  ITEM_UNCOMPED: ItemUncompedRenderer,
  PAYMENT_ADDED: PaymentAddedRenderer,
  PAYMENT_CANCELLED: PaymentCancelledRenderer,
  ITEM_SPLIT: ItemSplitRenderer,
  AMOUNT_SPLIT: AmountSplitRenderer,
  AA_SPLIT_STARTED: AaSplitStartedRenderer,
  AA_SPLIT_PAID: AaSplitPaidRenderer,
  AA_SPLIT_CANCELLED: AaSplitCancelledRenderer,
  ORDER_COMPLETED: OrderCompletedRenderer,
  ORDER_VOIDED: OrderVoidedRenderer,
  ORDER_MERGED: OrderMergedRenderer,
  ORDER_MOVED: OrderMovedRenderer,
  ORDER_MOVED_OUT: OrderMovedOutRenderer,
  ORDER_MERGED_OUT: OrderMergedOutRenderer,
  TABLE_REASSIGNED: TableReassignedRenderer,
  ORDER_INFO_UPDATED: OrderInfoUpdatedRenderer,
  RULE_SKIP_TOGGLED: RuleSkipToggledRenderer,
  ORDER_DISCOUNT_APPLIED: OrderDiscountAppliedRenderer,
  ORDER_SURCHARGE_APPLIED: OrderSurchargeAppliedRenderer,
  ORDER_NOTE_ADDED: OrderNoteAddedRenderer,
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
