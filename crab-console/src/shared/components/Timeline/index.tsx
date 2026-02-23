import React, { useMemo } from 'react';
import {
  Clock, Utensils, CheckCircle, ShoppingBag, Pencil, Trash2, Tag,
  Gift, Ban, Coins, Split, Users, XCircle, ArrowRight, ArrowLeft,
  UserPlus, UserMinus, Award,
  type LucideIcon,
} from 'lucide-react';
import { formatCurrency } from '@/utils/format';

/* ═══════════════════════════════════════════════════════════════════════
   Types
   ═══════════════════════════════════════════════════════════════════════ */

export interface TimelineTag {
  text: string;
  type: 'item' | 'payment';
}

export interface DetailTag {
  label: string;
  value: string;
  colorClass: string;
}

export interface EventDisplay {
  title: string;
  summary?: string;
  details: string[];
  detailTags?: DetailTag[];
  tags?: TimelineTag[];
  icon: LucideIcon;
  colorClass: string;
  timestamp: number;
  operatorName?: string | null;
}

/** Normalized event input — works for both archived (data: string) and live (payload: object) */
export interface TimelineEvent {
  event_type: string;
  timestamp: number;
  operator_name?: string | null;
  /** Already-parsed payload object */
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  payload: Record<string, any>;
}

/* ═══════════════════════════════════════════════════════════════════════
   Event Config (icon + color + i18n key for each event type)
   ═══════════════════════════════════════════════════════════════════════ */

const EVENT_CONFIG: Record<string, { icon: LucideIcon; color: string; titleKey: string }> = {
  TABLE_OPENED:               { icon: Utensils,    color: 'bg-blue-500',    titleKey: 'timeline.table_opened' },
  ITEMS_ADDED:                { icon: ShoppingBag,  color: 'bg-orange-500',  titleKey: 'timeline.items_added' },
  ITEM_MODIFIED:              { icon: Pencil,       color: 'bg-yellow-500',  titleKey: 'timeline.item_modified' },
  ITEM_REMOVED:               { icon: Trash2,       color: 'bg-red-500',     titleKey: 'timeline.item_removed' },
  ITEM_COMPED:                { icon: Gift,         color: 'bg-emerald-500', titleKey: 'timeline.item_comped' },
  ITEM_UNCOMPED:              { icon: Tag,          color: 'bg-amber-500',   titleKey: 'timeline.item_uncomped' },
  PAYMENT_ADDED:              { icon: Coins,        color: 'bg-green-500',   titleKey: 'timeline.payment_added' },
  PAYMENT_CANCELLED:          { icon: Ban,          color: 'bg-red-400',     titleKey: 'timeline.payment_cancelled' },
  ORDER_COMPLETED:            { icon: CheckCircle,  color: 'bg-green-600',   titleKey: 'timeline.order_completed' },
  ORDER_VOIDED:               { icon: Ban,          color: 'bg-red-700',     titleKey: 'timeline.order_voided' },
  ORDER_MERGED:               { icon: ArrowLeft,    color: 'bg-purple-500',  titleKey: 'timeline.order_merged' },
  ORDER_MOVED:                { icon: ArrowRight,   color: 'bg-indigo-500',  titleKey: 'timeline.order_moved' },
  ORDER_MOVED_OUT:            { icon: ArrowRight,   color: 'bg-indigo-600',  titleKey: 'timeline.order_moved_out' },
  ORDER_MERGED_OUT:           { icon: ArrowRight,   color: 'bg-purple-600',  titleKey: 'timeline.order_merged_out' },
  TABLE_REASSIGNED:           { icon: ArrowRight,   color: 'bg-blue-600',    titleKey: 'timeline.table_reassigned' },
  ORDER_INFO_UPDATED:         { icon: Pencil,       color: 'bg-blue-400',    titleKey: 'timeline.order_info_updated' },
  RULE_SKIP_TOGGLED:          { icon: Tag,          color: 'bg-orange-400',  titleKey: 'timeline.rule_toggled' },
  ORDER_DISCOUNT_APPLIED:     { icon: Tag,          color: 'bg-orange-400',  titleKey: 'timeline.discount_applied' },
  ORDER_SURCHARGE_APPLIED:    { icon: Tag,          color: 'bg-purple-400',  titleKey: 'timeline.surcharge_applied' },
  ORDER_NOTE_ADDED:           { icon: Pencil,       color: 'bg-blue-400',    titleKey: 'timeline.note_added' },
  MEMBER_LINKED:              { icon: UserPlus,     color: 'bg-red-400',     titleKey: 'timeline.member_linked' },
  MEMBER_UNLINKED:            { icon: UserMinus,    color: 'bg-red-400',     titleKey: 'timeline.member_unlinked' },
  ITEM_SPLIT:                 { icon: Split,        color: 'bg-teal-500',    titleKey: 'timeline.item_split' },
  AMOUNT_SPLIT:               { icon: Split,        color: 'bg-teal-500',    titleKey: 'timeline.amount_split' },
  AA_SPLIT_STARTED:           { icon: Users,        color: 'bg-cyan-500',    titleKey: 'timeline.aa_split_started' },
  AA_SPLIT_PAID:              { icon: Users,        color: 'bg-cyan-600',    titleKey: 'timeline.aa_split_paid' },
  AA_SPLIT_CANCELLED:         { icon: XCircle,      color: 'bg-red-400',     titleKey: 'timeline.aa_split_cancelled' },
  STAMP_REDEEMED:             { icon: Award,        color: 'bg-amber-400',   titleKey: 'timeline.stamp_redeemed' },
  STAMP_REDEMPTION_CANCELLED: { icon: Award,        color: 'bg-slate-400',   titleKey: 'timeline.stamp_cancelled' },
};

/* ═══════════════════════════════════════════════════════════════════════
   Render single event → EventDisplay
   ═══════════════════════════════════════════════════════════════════════ */

export function renderEventDisplay(event: TimelineEvent, t: (k: string) => string): EventDisplay {
  const config = EVENT_CONFIG[event.event_type];
  const p = event.payload;

  if (!config) {
    return {
      title: event.event_type,
      details: [],
      icon: Tag,
      colorClass: 'bg-slate-400',
      timestamp: event.timestamp,
      operatorName: event.operator_name,
    };
  }

  let title = t(config.titleKey);
  const details: string[] = [];
  const detailTags: DetailTag[] = [];
  const tags: TimelineTag[] = [];
  let summary: string | undefined;

  const addItemTag = (id: string | undefined) => {
    if (id) tags.push({ text: `#${id.slice(-5)}`, type: 'item' });
  };
  const addPaymentTag = (id: string | undefined) => {
    if (id) tags.push({ text: `#${id.slice(-5)}`, type: 'payment' });
  };

  switch (event.event_type) {
    case 'TABLE_OPENED': {
      if (p.table_name) details.push(`${t('orders.table')}: ${p.table_name}`);
      if (p.zone_name) details.push(`${t('orders.zone')}: ${p.zone_name}`);
      if (p.receipt_number) details.push(`${t('orders.receipt')}: ${p.receipt_number}`);
      if (p.guest_count) summary = `${p.guest_count} ${t('orders.guests')}`;
      break;
    }
    case 'ITEMS_ADDED': {
      const items = p.items || [];
      const totalQty = items.reduce((s: number, i: { quantity: number }) => s + i.quantity, 0);
      if (totalQty > 0) summary = `${totalQty} items`;
      for (const item of items) {
        const id = item.instance_id ? `#${item.instance_id.slice(-5)} ` : '';
        const mods: string[] = [];
        if (item.manual_discount_percent) mods.push(`-${item.manual_discount_percent}%`);
        details.push(`${id}${item.name} x${item.quantity}${mods.length ? ` (${mods.join(', ')})` : ''}`);
      }
      break;
    }
    case 'ITEM_MODIFIED': {
      if (p.source?.name) summary = p.source.name;
      addItemTag(p.source?.instance_id);
      const updatedResult = (p.results || []).find((r: { action: string; instance_id: string }) => r.action === 'UPDATED' || r.action === 'CREATED');
      if (updatedResult && updatedResult.instance_id !== p.source?.instance_id) {
        tags.push({ text: `→ #${updatedResult.instance_id.slice(-5)}`, type: 'item' });
      }
      const changes = p.changes || {};
      const prev = p.previous_values || {};
      if (typeof changes.quantity === 'number' && changes.quantity !== prev.quantity) {
        details.push(`${t('timeline.quantity')}: ${prev.quantity ?? '?'} → ${changes.quantity}`);
      }
      if (typeof changes.price === 'number' && changes.price !== prev.price) {
        details.push(`${t('orders.subtotal')}: ${formatCurrency(prev.price ?? 0)} → ${formatCurrency(changes.price)}`);
      }
      if (typeof changes.manual_discount_percent === 'number') {
        details.push(`${t('orders.discount')}: ${prev.manual_discount_percent ?? 0}% → ${changes.manual_discount_percent}%`);
      }
      if (changes.selected_specification) {
        const oldName = prev.selected_specification?.name ?? '-';
        details.push(`Spec: ${oldName} → ${changes.selected_specification.name}`);
      }
      if (changes.selected_options) {
        const groupByAttr = (opts: { attribute_name: string; option_name: string; quantity?: number }[]) => {
          const map = new Map<string, Map<string, number>>();
          for (const o of opts) {
            const attr = o.attribute_name || '';
            if (!map.has(attr)) map.set(attr, new Map());
            map.get(attr)!.set(o.option_name, o.quantity ?? 1);
          }
          return map;
        };
        const oldByAttr = groupByAttr(prev.selected_options ?? []);
        const newByAttr = groupByAttr(changes.selected_options);
        const allAttrs = new Set([...oldByAttr.keys(), ...newByAttr.keys()]);
        for (const attr of allAttrs) {
          const oldMap = oldByAttr.get(attr) || new Map();
          const newMap = newByAttr.get(attr) || new Map();
          const allOpts = new Set([...oldMap.keys(), ...newMap.keys()]);
          for (const name of allOpts) {
            const oldQty = oldMap.get(name);
            const newQty = newMap.get(name);
            if (oldQty === undefined && newQty !== undefined) {
              details.push(`${attr} + ${name}${newQty > 1 ? ` ×${newQty}` : ''}`);
            } else if (oldQty !== undefined && newQty === undefined) {
              details.push(`${attr} - ${name}`);
            } else if (oldQty !== newQty) {
              details.push(`${attr} ${name}: ×${oldQty} → ×${newQty}`);
            }
          }
        }
      }
      if (p.authorizer_name && p.authorizer_name !== event.operator_name) {
        details.push(`${t('timeline.authorizer')}: ${p.authorizer_name}`);
      }
      break;
    }
    case 'ITEM_REMOVED': {
      if (p.item_name) summary = p.item_name;
      addItemTag(p.instance_id);
      if (p.quantity) details.push(`${t('timeline.quantity')}: ${p.quantity}`);
      if (p.reason) details.push(`${t('timeline.reason')}: ${p.reason}`);
      if (p.authorizer_name) details.push(`${t('timeline.authorizer')}: ${p.authorizer_name}`);
      break;
    }
    case 'ITEM_COMPED': {
      if (p.item_name) summary = p.item_name;
      addItemTag(p.instance_id);
      if (p.quantity) details.push(`${t('timeline.quantity')}: ${p.quantity}`);
      if (p.reason) details.push(`${t('timeline.reason')}: ${p.reason}`);
      if (p.authorizer_name) details.push(`${t('timeline.authorizer')}: ${p.authorizer_name}`);
      break;
    }
    case 'ITEM_UNCOMPED': {
      if (p.item_name) summary = p.item_name;
      addItemTag(p.instance_id);
      if (p.restored_price != null) details.push(`${t('orders.subtotal')}: ${formatCurrency(p.restored_price)}`);
      if (p.merged_into) details.push(`Merged → #${p.merged_into.slice(-5)}`);
      if (p.authorizer_name) details.push(`${t('timeline.authorizer')}: ${p.authorizer_name}`);
      break;
    }
    case 'PAYMENT_ADDED': {
      const method = p.method || 'unknown';
      title = `${t(config.titleKey)}: ${method}`;
      addPaymentTag(p.payment_id);
      if (p.amount != null) summary = `+${formatCurrency(p.amount)}`;
      if (p.tendered != null) {
        details.push(`${t('orders.paid')}: ${formatCurrency(p.tendered)}`);
        details.push(`Change: ${formatCurrency(p.change || 0)}`);
      }
      if (p.note) details.push(p.note);
      break;
    }
    case 'PAYMENT_CANCELLED': {
      const method = p.method || 'unknown';
      title = `${t(config.titleKey)}: ${method}`;
      addPaymentTag(p.payment_id);
      if (p.amount != null) summary = `-${formatCurrency(p.amount)}`;
      if (p.reason) details.push(`${t('timeline.reason')}: ${p.reason}`);
      if (p.authorizer_name) details.push(`${t('timeline.authorizer')}: ${p.authorizer_name}`);
      break;
    }
    case 'ORDER_COMPLETED': {
      if (p.final_total != null) details.push(`${t('orders.total')}: ${formatCurrency(p.final_total)}`);
      if (p.receipt_number) summary = `#${p.receipt_number}`;
      if (p.payment_summary) {
        for (const ps of p.payment_summary) {
          details.push(`${ps.method}: ${formatCurrency(ps.amount)}`);
        }
      }
      break;
    }
    case 'ORDER_VOIDED': {
      summary = p.void_type === 'LOSS_SETTLED' ? 'Loss Settled' : 'Cancelled';
      if (p.loss_reason) details.push(`${t('timeline.reason')}: ${p.loss_reason}`);
      if (p.loss_amount != null) details.push(`${t('timeline.loss_amount')}: ${formatCurrency(p.loss_amount)}`);
      if (p.note) details.push(p.note);
      if (p.authorizer_name) details.push(`${t('timeline.authorizer')}: ${p.authorizer_name}`);
      break;
    }
    case 'ORDER_MERGED': {
      if (p.source_table_name) summary = `← ${p.source_table_name}`;
      if (p.items?.length) details.push(`${t('orders.items')}: ${p.items.reduce((s: number, i: { quantity: number }) => s + i.quantity, 0)}`);
      break;
    }
    case 'ORDER_MOVED': {
      if (p.source_table_name && p.target_table_name) summary = `${p.source_table_name} → ${p.target_table_name}`;
      if (p.items?.length) details.push(`${t('orders.items')}: ${p.items.reduce((s: number, i: { quantity: number }) => s + i.quantity, 0)}`);
      break;
    }
    case 'ORDER_MOVED_OUT': {
      if (p.target_table_name) summary = `→ ${p.target_table_name}`;
      if (p.reason) details.push(`${t('timeline.reason')}: ${p.reason}`);
      break;
    }
    case 'ORDER_MERGED_OUT': {
      if (p.target_table_name) summary = `→ ${p.target_table_name}`;
      if (p.reason) details.push(`${t('timeline.reason')}: ${p.reason}`);
      break;
    }
    case 'TABLE_REASSIGNED': {
      if (p.source_table_name && p.target_table_name) summary = `${p.source_table_name} → ${p.target_table_name}`;
      if (p.target_zone_name) details.push(`${t('orders.zone')}: ${p.target_zone_name}`);
      if (p.items?.length) details.push(`${t('orders.items')}: ${p.items.reduce((s: number, i: { quantity: number }) => s + i.quantity, 0)}`);
      break;
    }
    case 'ORDER_INFO_UPDATED': {
      if (p.guest_count != null) details.push(`${t('orders.guests')}: ${p.guest_count}`);
      if (p.table_name != null) details.push(`${t('orders.table')}: ${p.table_name}`);
      break;
    }
    case 'RULE_SKIP_TOGGLED': {
      summary = `${p.skipped ? 'Skipped' : 'Applied'}: ${p.rule_name || ''}`;
      break;
    }
    case 'ORDER_DISCOUNT_APPLIED': {
      if (p.discount_percent) {
        const computed = p.discount !== 0 ? ` (-${formatCurrency(p.discount)})` : '';
        detailTags.push({ label: t('orders.discount'), value: `${p.discount_percent}%${computed}`, colorClass: 'bg-orange-100 text-orange-700 border-orange-200' });
      } else if (p.discount_fixed) {
        detailTags.push({ label: t('orders.discount'), value: `-${formatCurrency(p.discount_fixed)}`, colorClass: 'bg-orange-100 text-orange-700 border-orange-200' });
      }
      if (!p.discount_percent && !p.discount_fixed) title = t('timeline.discount_cleared');
      if (p.subtotal != null) details.push(`${t('orders.subtotal')}: ${formatCurrency(p.subtotal)}`);
      if (p.discount != null) details.push(`${t('orders.discount')}: -${formatCurrency(p.discount)}`);
      if (p.total != null) details.push(`${t('orders.total')}: ${formatCurrency(p.total)}`);
      break;
    }
    case 'ORDER_SURCHARGE_APPLIED': {
      if (p.surcharge_percent) {
        const computed = p.surcharge !== 0 ? ` (+${formatCurrency(p.surcharge)})` : '';
        detailTags.push({ label: t('orders.surcharge'), value: `${p.surcharge_percent}%${computed}`, colorClass: 'bg-purple-100 text-purple-700 border-purple-200' });
      } else if (p.surcharge_amount) {
        detailTags.push({ label: t('orders.surcharge'), value: `+${formatCurrency(p.surcharge_amount)}`, colorClass: 'bg-purple-100 text-purple-700 border-purple-200' });
      }
      if (!p.surcharge_percent && !p.surcharge_amount) title = t('timeline.surcharge_cleared');
      if (p.subtotal != null) details.push(`${t('orders.subtotal')}: ${formatCurrency(p.subtotal)}`);
      if (p.surcharge != null) details.push(`${t('orders.surcharge')}: +${formatCurrency(p.surcharge)}`);
      if (p.total != null) details.push(`${t('orders.total')}: ${formatCurrency(p.total)}`);
      break;
    }
    case 'ORDER_NOTE_ADDED': {
      if (p.note === '') title = t('timeline.note_cleared');
      else if (p.note) details.push(p.note);
      if (p.previous_note) details.push(`← ${p.previous_note}`);
      break;
    }
    case 'MEMBER_LINKED': {
      if (p.member_name) summary = p.member_name;
      if (p.marketing_group_name) details.push(p.marketing_group_name);
      break;
    }
    case 'MEMBER_UNLINKED': {
      if (p.previous_member_name) summary = p.previous_member_name;
      break;
    }
    case 'ITEM_SPLIT': {
      if (p.split_amount != null) summary = `${formatCurrency(p.split_amount)} · ${p.payment_method || ''}`;
      addPaymentTag(p.payment_id);
      if (p.items) {
        for (const item of p.items) {
          const id = item.instance_id ? `#${item.instance_id.slice(-5)} ` : '';
          details.push(`${id}${item.name} x${item.quantity}`);
        }
      }
      break;
    }
    case 'AMOUNT_SPLIT': {
      if (p.split_amount != null) summary = `${formatCurrency(p.split_amount)} · ${p.payment_method || ''}`;
      addPaymentTag(p.payment_id);
      break;
    }
    case 'AA_SPLIT_STARTED': {
      if (p.total_shares && p.per_share_amount != null) {
        summary = `${formatCurrency(p.order_total ?? 0)} / ${p.total_shares} = ${formatCurrency(p.per_share_amount)}`;
      }
      break;
    }
    case 'AA_SPLIT_PAID': {
      if (p.amount != null) summary = `${formatCurrency(p.amount)} · ${p.payment_method || ''}`;
      addPaymentTag(p.payment_id);
      if (p.progress_paid != null && p.progress_total != null) {
        details.push(`${p.progress_paid}/${p.progress_total}`);
      }
      break;
    }
    case 'AA_SPLIT_CANCELLED': {
      if (p.total_shares) summary = `${p.total_shares} shares`;
      break;
    }
    case 'STAMP_REDEEMED': {
      if (p.stamp_activity_name) summary = p.stamp_activity_name;
      if (p.reward_strategy) details.push(p.reward_strategy);
      break;
    }
    case 'STAMP_REDEMPTION_CANCELLED': {
      if (p.stamp_activity_name) summary = p.stamp_activity_name;
      break;
    }
  }

  return {
    title,
    summary,
    details,
    detailTags: detailTags.length > 0 ? detailTags : undefined,
    tags: tags.length > 0 ? tags : undefined,
    icon: config.icon,
    colorClass: config.color,
    timestamp: event.timestamp,
    operatorName: event.operator_name,
  };
}

/* ═══════════════════════════════════════════════════════════════════════
   Visual Components
   ═══════════════════════════════════════════════════════════════════════ */

const TAG_TYPE_STYLES: Record<TimelineTag['type'], string> = {
  item: 'bg-blue-100 text-blue-600 border-blue-200',
  payment: 'bg-emerald-100 text-emerald-600 border-emerald-200',
};

const NOTE_TAG_STYLES = [
  'bg-blue-100 text-blue-700 border-blue-200',
  'bg-green-100 text-green-700 border-green-200',
  'bg-purple-100 text-purple-700 border-purple-200',
  'bg-orange-100 text-orange-700 border-orange-200',
  'bg-pink-100 text-pink-700 border-pink-200',
  'bg-cyan-100 text-cyan-700 border-cyan-200',
  'bg-indigo-100 text-indigo-700 border-indigo-200',
  'bg-rose-100 text-rose-700 border-rose-200',
];

function getNoteTagStyle(text: string) {
  let hash = 0;
  for (let i = 0; i < text.length; i++) hash = text.charCodeAt(i) + ((hash << 5) - hash);
  return NOTE_TAG_STYLES[Math.abs(hash) % NOTE_TAG_STYLES.length];
}

const HashText: React.FC<{ text: string }> = ({ text }) => {
  const parts = text.split(/(#[a-f0-9]{5})/gi);
  return (
    <span>
      {parts.map((part, i) =>
        /^#[a-f0-9]{5}$/i.test(part)
          ? <span key={i} className="mx-1 px-1.5 py-0.5 rounded text-[0.625rem] font-bold font-mono bg-blue-100 text-blue-600 border border-blue-200">{part}</span>
          : <span key={i}>{part}</span>
      )}
    </span>
  );
};

const NoteTag: React.FC<{ text: string }> = ({ text }) => {
  const parts = text.split(/[:：]/);
  const name = parts[0].trim();
  const detail = parts.slice(1).join(':').trim();
  return (
    <div className="flex items-center gap-2 text-sm">
      <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-bold border shadow-sm ${getNoteTagStyle(name)}`}>
        {name}
      </span>
      {detail && <span className="text-slate-500 text-xs"><HashText text={detail} /></span>}
    </div>
  );
};

/* ═══════════════════════════════════════════════════════════════════════
   EventTimeline Component
   ═══════════════════════════════════════════════════════════════════════ */

export const EventTimeline: React.FC<{
  events: TimelineEvent[];
  t: (k: string) => string;
}> = ({ events, t }) => {
  const displays = useMemo(() => events.map(e => renderEventDisplay(e, t)), [events, t]);

  return (
    <div className="relative pl-3 border-l-2 border-slate-200 space-y-5">
      {displays.map((d, i) => {
        const Icon = d.icon;
        return (
          <div key={i} className="relative pl-6">
            <div className={`absolute -left-[calc(0.75rem+1px)] top-0 w-5 h-5 rounded-full border-2 border-white flex items-center justify-center text-white ${d.colorClass}`}>
              <Icon size={12} strokeWidth={2.5} />
            </div>
            <div className="flex items-center justify-between gap-2">
              <div className="text-sm font-bold text-slate-800">{d.title}</div>
              {d.tags && d.tags.length > 0 && (
                <div className="flex items-center gap-1">
                  {d.tags.map((tag, ti) => (
                    <span key={ti} className={`px-1.5 py-0.5 rounded text-[0.625rem] font-bold font-mono border ${TAG_TYPE_STYLES[tag.type]}`}>
                      {tag.text}
                    </span>
                  ))}
                </div>
              )}
            </div>
            <div className="text-xs text-slate-400 font-mono">
              {new Date(d.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false })}
              {d.operatorName && <span className="ml-2 text-slate-500">{d.operatorName}</span>}
            </div>
            {d.summary && <div className="text-sm leading-6 font-medium text-slate-700 mt-0.5">{d.summary}</div>}
            {((d.details.length > 0) || (d.detailTags && d.detailTags.length > 0)) && (
              <div className="mt-1 bg-slate-50 p-2 rounded text-xs text-slate-600 space-y-0.5">
                {d.detailTags?.map((tag, ti) => (
                  <div key={`tag-${ti}`} className="flex items-center gap-2 text-sm">
                    <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-bold border shadow-sm ${tag.colorClass}`}>
                      {tag.label}
                    </span>
                    <span className="text-slate-500 text-xs">{tag.value}</span>
                  </div>
                ))}
                {d.details.map((line, j) => (
                  <div key={j}>
                    {line.includes(':') && !line.includes('€') && !line.includes('→')
                      ? <NoteTag text={line} />
                      : <HashText text={line} />
                    }
                  </div>
                ))}
              </div>
            )}
          </div>
        );
      })}
    </div>
  );
};

/** Timeline card with header — ready to embed in any layout */
export const TimelineCard: React.FC<{
  events: TimelineEvent[];
  t: (k: string) => string;
  className?: string;
}> = ({ events, t, className = '' }) => (
  <div className={`bg-white rounded-2xl shadow-sm border border-slate-200 overflow-hidden flex flex-col h-fit ${className}`}>
    <div className="p-4 border-b border-slate-100 bg-slate-50 flex items-center gap-2 font-bold text-slate-700">
      <Clock className="w-[18px] h-[18px]" />
      <span>{t('timeline.title')}</span>
    </div>
    <div className="p-4">
      {events.length > 0 ? (
        <EventTimeline events={events} t={t} />
      ) : (
        <div className="text-center text-slate-400 text-sm py-4">
          {t('timeline.empty')}
        </div>
      )}
    </div>
  </div>
);
