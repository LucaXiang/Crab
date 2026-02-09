import type {
  ItemsAddedPayload,
  ItemModifiedPayload,
  ItemRemovedPayload,
  ItemCompedPayload,
  ItemUncompedPayload,
  SpecificationInfo,
  ItemOption,
} from '@/core/domain/types/orderEvent';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { ShoppingBag, Edit3, Trash2, Tag } from 'lucide-react';
import type { EventRenderer, TimelineTag } from './types';

export const ItemsAddedRenderer: EventRenderer<ItemsAddedPayload> = {
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

export const ItemModifiedRenderer: EventRenderer<ItemModifiedPayload> = {
  render(event, payload, t) {
    const changes = payload.changes || {};
    const previousValues = payload.previous_values || {};
    const details: string[] = [];

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

    if (changes.selected_specification) {
      const oldSpec = previousValues.selected_specification;
      const oldName = oldSpec && typeof oldSpec === 'object' && 'name' in oldSpec ? (oldSpec as SpecificationInfo).name : '-';
      details.push(`${t('pos.cart.spec')}: ${oldName} → ${changes.selected_specification.name}`);
    }

    if (changes.selected_options) {
      const oldOpts = previousValues.selected_options ?? [];
      const newOpts = changes.selected_options;

      const groupByAttr = (opts: ItemOption[]) => {
        const map = new Map<string, Map<string, number>>();
        for (const o of opts) {
          const attr = o.attribute_name || '';
          if (!map.has(attr)) map.set(attr, new Map());
          map.get(attr)!.set(o.option_name, o.quantity ?? 1);
        }
        return map;
      };

      const oldByAttr = groupByAttr(oldOpts);
      const newByAttr = groupByAttr(newOpts);
      const allAttrs = new Set([...oldByAttr.keys(), ...newByAttr.keys()]);

      for (const attr of allAttrs) {
        const oldMap = oldByAttr.get(attr) || new Map();
        const newMap = newByAttr.get(attr) || new Map();
        const allOptions = new Set([...oldMap.keys(), ...newMap.keys()]);

        for (const name of allOptions) {
          const oldQty = oldMap.get(name);
          const newQty = newMap.get(name);

          if (oldQty === undefined && newQty !== undefined) {
            const qtyStr = newQty > 1 ? ` ×${newQty}` : '';
            details.push(`${attr} ${t('timeline.option_added')} ${name}${qtyStr}`);
          } else if (oldQty !== undefined && newQty === undefined) {
            details.push(`${attr} ${t('timeline.option_removed')} ${name}`);
          } else if (oldQty !== newQty) {
            details.push(`${attr} ${name}: ×${oldQty} → ×${newQty}`);
          }
        }
      }
    }

    if (payload.authorizer_name && payload.authorizer_name !== event.operator_name) {
      details.push(`${t('timeline.labels.authorizer')}: ${payload.authorizer_name}`);
    }

    const source = payload.source;

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

export const ItemRemovedRenderer: EventRenderer<ItemRemovedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    if (payload.quantity != null && payload.quantity > 0) {
      details.push(`${t('timeline.labels.quantity')}: ${payload.quantity}`);
    }
    if (payload.reason) {
      details.push(`${t('timeline.labels.reason')}: ${payload.reason}`);
    }
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

export const ItemCompedRenderer: EventRenderer<ItemCompedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    if (payload.quantity != null && payload.quantity > 0) {
      details.push(`${t('timeline.labels.quantity')}: ${payload.quantity}`);
    }
    if (payload.reason) {
      const reasonKey = `checkout.comp.preset.${payload.reason}`;
      const resolved = t(reasonKey);
      const displayReason = resolved !== reasonKey ? resolved : payload.reason;
      details.push(`${t('timeline.labels.reason')}: ${displayReason}`);
    }
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

export const ItemUncompedRenderer: EventRenderer<ItemUncompedPayload> = {
  render(event, payload, t) {
    const details: string[] = [];

    if (payload.restored_price != null) {
      details.push(`${t('timeline.labels.price')}: ${formatCurrency(payload.restored_price)}`);
    }
    if (payload.merged_into) {
      details.push(`${t('timeline.merged_back')}: #${payload.merged_into.slice(-5)}`);
    }
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
