import React, { useState, useEffect, useCallback, useMemo } from 'react';
import type { ArchivedOrderDetail, ArchivedOrderItem, ArchivedPayment, ArchivedEvent } from '@/core/domain/types';
import type { OrderEvent, OrderEventType, EventPayload } from '@/core/domain/types/orderEvent';
import { useI18n } from '@/hooks/useI18n';
import { useCategoryStore } from '@/core/stores/resources';
import { formatCurrency, Currency, computePriceBreakdown } from '@/utils/currency';
import { CATEGORY_ACCENT } from '@/utils/categoryColors';
import { Receipt, Calendar, Printer, CreditCard, Coins, Clock, ChevronDown, ChevronUp, ChevronsDown, ChevronsUp, Ban, Gift, Stamp, Tag, Hash, Undo2, FileUp } from 'lucide-react';
import { Permission } from '@/core/domain/types';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { TimelineList } from '@/shared/components/TimelineList';
import { calculateItemSink } from '@/utils/itemSorting';
import { KitchenReprintModal } from '@/screens/Checkout/KitchenReprintModal';
import { LabelReprintModal } from '@/screens/Checkout/LabelReprintModal';
import { CreditNoteSection } from './CreditNoteSection';
import { RefundModal } from './RefundModal';
import { AnulacionModal } from './AnulacionModal';
import { UpgradeInvoiceModal } from './UpgradeInvoiceModal';
import { InvoiceSection } from './InvoiceSection';

interface HistoryDetailProps {
  order?: ArchivedOrderDetail;
  onReprint: () => void;
  hashInfo?: { prev_hash: string; curr_hash: string };
  onRefundCreated?: () => void;
  onNavigateToCreditNote?: (creditNotePk: number) => void;
  onAnulacionCreated?: () => void;
  onUpgradeCreated?: () => void;
}

/**
 * Convert ArchivedEvent to OrderEvent format for TimelineList compatibility
 *
 * Backend stores:
 * - event_type: SCREAMING_SNAKE_CASE (e.g., "TABLE_OPENED")
 * - payload: JSON with `type` field from serde(tag = "type")
 */
function convertArchivedEventToOrderEvent(event: ArchivedEvent, index: number): OrderEvent {
  // Backend uses SCREAMING_SNAKE_CASE via serde(rename_all)
  const eventType = event.event_type as OrderEventType;

  // Backend payload already has `type` field from serde serialization
  // If payload is null/empty, create minimal payload with type
  const payload: EventPayload = (event.payload && Object.keys(event.payload).length > 0)
    ? event.payload
    : { type: eventType } as EventPayload;

  return {
    event_id: event.event_id,
    sequence: index,
    order_id: 0,
    timestamp: event.timestamp,
    operator_id: 0,
    operator_name: '',
    command_id: 0,
    event_type: eventType,
    payload,
  };
}

export const HistoryDetail: React.FC<HistoryDetailProps> = ({ order, onReprint, hashInfo, onRefundCreated, onNavigateToCreditNote, onAnulacionCreated, onUpgradeCreated }) => {
  const { t } = useI18n();
  const [expandedItems, setExpandedItems] = useState<Set<number>>(new Set());
  const [showKitchenReprint, setShowKitchenReprint] = useState(false);
  const [showLabelReprint, setShowLabelReprint] = useState(false);
  const [showRefundModal, setShowRefundModal] = useState(false);
  const [showAnulacionModal, setShowAnulacionModal] = useState(false);
  const [showUpgradeModal, setShowUpgradeModal] = useState(false);
  const [printMenuOpen, setPrintMenuOpen] = useState(false);
  const [actionMenuOpen, setActionMenuOpen] = useState(false);
  const categories = useCategoryStore((s) => s.items);
  const categoriesLoaded = useCategoryStore((s) => s.isLoaded);

  useEffect(() => {
    if (!categoriesLoaded) useCategoryStore.getState().fetchAll();
  }, [categoriesLoaded]);

  // Close dropdowns on outside click
  useEffect(() => {
    if (!printMenuOpen && !actionMenuOpen) return;
    const handler = () => { setPrintMenuOpen(false); setActionMenuOpen(false); };
    document.addEventListener('click', handler);
    return () => document.removeEventListener('click', handler);
  }, [printMenuOpen, actionMenuOpen]);

  // Convert archived events to OrderEvent format for TimelineList
  const timelineEvents = useMemo(() => {
    if (!order?.timeline) return [];
    return order.timeline.map((event, index) => convertArchivedEventToOrderEvent(event, index));
  }, [order?.timeline]);

  const breakdown = useMemo(
    () => order ? computePriceBreakdown(order.items, order) : null, [order],
  );

  // Sort items: category sort_order → paid/comped sink → name
  const sortedItems = useMemo(() => {
    if (!order) return [];
    const categoryMap = new Map(categories.map(c => [c.id, c]));

    return [...order.items].sort((a, b) => {
      const sortA = a.category_id != null ? (categoryMap.get(a.category_id)?.sort_order ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;
      const sortB = b.category_id != null ? (categoryMap.get(b.category_id)?.sort_order ?? Number.MAX_SAFE_INTEGER) : Number.MAX_SAFE_INTEGER;
      if (sortA !== sortB) return sortA - sortB;

      const sinkA = calculateItemSink(a);
      const sinkB = calculateItemSink(b);
      if (sinkA !== sinkB) return sinkA - sinkB;

      return a.name.localeCompare(b.name);
    });
  }, [order, categories]);

  // 按 category_id 出现顺序分配颜色（不依赖当前分类表）
  const itemColorMap = useMemo(() => {
    if (!order) return new Map<string, number>();
    const map = new Map<string, number>();
    const seen: (number | null)[] = [];
    for (const item of order.items) {
      const catId = item.category_id;
      let idx = seen.indexOf(catId);
      if (idx === -1) { seen.push(catId); idx = seen.length - 1; }
      map.set(item.instance_id, idx % CATEGORY_ACCENT.length);
    }
    return map;
  }, [order]);

  useEffect(() => {
    setExpandedItems(new Set());
  }, [order?.order_id]);

  const toggleItem = useCallback((idx: number) => {
    setExpandedItems((prev) => {
      const next = new Set(prev);
      if (next.has(idx)) {
        next.delete(idx);
      } else {
        next.add(idx);
      }
      return next;
    });
  }, []);

  // 只有有详情（options）的 item 才算可展开
  const expandableIndices = useMemo(
    () => sortedItems.reduce<number[]>((acc, item, i) => {
      if (item.selected_options && item.selected_options.length > 0) acc.push(i);
      return acc;
    }, []),
    [sortedItems],
  );
  const allExpanded = expandableIndices.length > 0 && expandableIndices.every((i) => expandedItems.has(i));

  const toggleAll = () => {
    if (!order) return;
    if (allExpanded) {
      setExpandedItems(new Set());
    } else {
      setExpandedItems(new Set(expandableIndices));
    }
  };

  if (!order) {
    return (
      <div className="h-full flex flex-col items-center justify-center text-gray-300">
        <Receipt size={64} className="mb-4 opacity-50" />
        <p>{t('history.info.select_order')}</p>
      </div>
    );
  }

  const isVoid = order.status === 'VOID';
  const isMerged = order.status === 'MERGED';
  return (
    <div className="max-w-5xl mx-auto space-y-4">
      {/* Header */}
      <div className="bg-white rounded-2xl p-5 shadow-sm border border-gray-200 flex justify-between items-start">
        <div>
          <div className="flex items-center gap-3 mb-2">
            <h1 className={`text-2xl font-bold ${isVoid || isMerged ? 'text-gray-500 line-through' : 'text-gray-900'}`}>
              {order.receipt_number || (order.is_retail ? t('common.label.retail') : order.table_name)}
            </h1>
            {isVoid && (
              <span className="px-2 py-1 bg-red-100 text-red-700 text-xs font-bold rounded uppercase">
                {t('history.status.voided')}
              </span>
            )}
            {isMerged && (
              <span className="px-2 py-1 bg-blue-100 text-blue-700 text-xs font-bold rounded uppercase">
                {t('history.status.merged')}
              </span>
            )}
            {order.is_voided && (
              <span className="px-2 py-1 bg-gray-200 text-gray-700 text-xs font-bold rounded uppercase">
                {t('anulacion.status.anulada')}
              </span>
            )}
            {order.is_upgraded && (
              <span className="px-2 py-1 bg-blue-100 text-blue-700 text-xs font-bold rounded uppercase">
                {t('upgrade.status.upgraded')}
              </span>
            )}
            {/* Print dropdown */}
            <div className="relative">
              <button
                onClick={(e) => { e.stopPropagation(); setPrintMenuOpen(!printMenuOpen); setActionMenuOpen(false); }}
                className="flex items-center gap-1.5 px-3 py-1 bg-white border border-gray-300 rounded-lg shadow-sm text-sm font-medium text-gray-700 hover:bg-gray-50 transition-colors"
              >
                <Printer size={16} />
                <span>{t('history.action.print_group')}</span>
                <ChevronDown size={14} />
              </button>
              {printMenuOpen && (
                <div className="absolute top-full left-0 mt-1 bg-white border border-gray-200 rounded-lg shadow-lg z-50 min-w-[160px] py-1">
                  <EscalatableGate permission={Permission.SETTINGS_MANAGE}>
                    <button
                      onClick={() => { onReprint(); setPrintMenuOpen(false); }}
                      className="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50"
                    >
                      <Printer size={15} />
                      <span>{t('history.action.reprint')}</span>
                    </button>
                  </EscalatableGate>
                  <button
                    onClick={() => { setShowKitchenReprint(true); setPrintMenuOpen(false); }}
                    className="w-full flex items-center gap-2 px-3 py-2 text-sm text-amber-700 hover:bg-amber-50"
                  >
                    <Printer size={15} />
                    <span>{t('checkout.kitchen_reprint.tab_kitchen')}</span>
                  </button>
                  <button
                    onClick={() => { setShowLabelReprint(true); setPrintMenuOpen(false); }}
                    className="w-full flex items-center gap-2 px-3 py-2 text-sm text-amber-700 hover:bg-amber-50"
                  >
                    <Tag size={15} />
                    <span>{t('checkout.label_reprint.tab_label')}</span>
                  </button>
                </div>
              )}
            </div>
            {/* Action dropdown */}
            {!isVoid && !isMerged && (
              <div className="relative">
                <button
                  onClick={(e) => { e.stopPropagation(); setActionMenuOpen(!actionMenuOpen); setPrintMenuOpen(false); }}
                  className="flex items-center gap-1.5 px-3 py-1 bg-white border border-red-300 rounded-lg shadow-sm text-sm font-medium text-red-600 hover:bg-red-50 transition-colors"
                >
                  <Undo2 size={16} />
                  <span>{t('history.action.correction_group')}</span>
                  <ChevronDown size={14} />
                </button>
                {actionMenuOpen && (
                  <div className="absolute top-full left-0 mt-1 bg-white border border-gray-200 rounded-lg shadow-lg z-50 min-w-[160px] py-1">
                    <EscalatableGate permission={Permission.ORDERS_REFUND}>
                      <button
                        onClick={() => { setShowRefundModal(true); setActionMenuOpen(false); }}
                        className="w-full flex items-center gap-2 px-3 py-2 text-sm text-red-600 hover:bg-red-50"
                      >
                        <Undo2 size={15} />
                        <span>{t('credit_note.action.create')}</span>
                      </button>
                    </EscalatableGate>
                    {!order.is_voided && (
                      <EscalatableGate permission={Permission.ORDERS_VOID}>
                        <button
                          onClick={() => { setShowAnulacionModal(true); setActionMenuOpen(false); }}
                          className="w-full flex items-center gap-2 px-3 py-2 text-sm text-gray-700 hover:bg-gray-50"
                        >
                          <Ban size={15} />
                          <span>{t('anulacion.action.void')}</span>
                        </button>
                      </EscalatableGate>
                    )}
                    {!order.is_upgraded && (
                      <EscalatableGate
                        permission={Permission.SETTINGS_MANAGE}
                        mode="intercept"
                        description={t('upgrade.action.upgrade')}
                        onAuthorized={() => { setShowUpgradeModal(true); setActionMenuOpen(false); }}
                      >
                        <button
                          className="w-full flex items-center gap-2 px-3 py-2 text-sm text-blue-600 hover:bg-blue-50"
                        >
                          <FileUp size={15} />
                          <span>{t('upgrade.action.upgrade')}</span>
                        </button>
                      </EscalatableGate>
                    )}
                  </div>
                )}
              </div>
            )}
          </div>
          <div className="flex gap-4 text-sm text-gray-500 flex-wrap">
            {order.table_name && order.table_name !== 'RETAIL' && (
              <div className="flex items-center gap-1.5 font-medium text-gray-700">
                <span>{t('history.info.table')}: {order.table_name}</span>
              </div>
            )}
            {order.operator_name && (
              <div className="flex items-center gap-1.5">
                <span>{t('history.info.operator')}: {order.operator_name}</span>
              </div>
            )}
            <div className="flex items-center gap-1.5">
              <Calendar size={16} />
              <span>{new Date(order.start_time).toLocaleDateString()}</span>
            </div>
            <div className="flex items-center gap-1.5">
              <Clock size={16} />
              <span>
                {new Date(order.start_time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}
                {' - '}
                {order.end_time ? new Date(order.end_time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false }) : t('common.label.none')}
              </span>
            </div>
            {hashInfo && (
              <div className="flex items-center gap-1.5 font-mono text-xs text-gray-400">
                <Hash size={14} />
                <span title={hashInfo.prev_hash}>{hashInfo.prev_hash ? hashInfo.prev_hash.slice(0, 8) + '…' : 'genesis'}</span>
                <span className="text-gray-300">→</span>
                <span title={hashInfo.curr_hash}>{hashInfo.curr_hash ? hashInfo.curr_hash.slice(0, 8) + '…' : '\u2014'}</span>
              </div>
            )}
          </div>

          {/* Void Information */}
          {isVoid && order.void_type && (
            <div className="mt-4 pt-3 border-t border-red-100 flex flex-wrap gap-6 text-sm">
              <div className="flex flex-col gap-0.5">
                <span className="text-xs text-red-400 font-medium uppercase">{t('common.status.void')}</span>
                <span className="text-red-700 font-medium">{t(`history.void_type.${order.void_type}`)}</span>
              </div>
              {order.void_type === 'LOSS_SETTLED' && order.loss_reason && (
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs text-gray-400 font-medium uppercase">{t('common.label.description')}</span>
                  <span className="text-gray-700 font-medium">{t(`history.loss_reason.${order.loss_reason}`)}</span>
                </div>
              )}
              {order.void_type === 'LOSS_SETTLED' && order.loss_amount !== null && order.loss_amount !== undefined && (
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs text-orange-400 font-medium uppercase">{t('common.label.loss_amount')}</span>
                  <span className="text-orange-600 font-bold">{formatCurrency(order.loss_amount)}</span>
                </div>
              )}
            </div>
          )}
        </div>
        <div className="text-right">
          <div className="text-sm text-gray-500 uppercase font-bold tracking-wider mb-1">{t('history.info.total_amount')}</div>
          <div className={`text-3xl font-bold ${isVoid || isMerged ? 'text-gray-400 line-through' : 'text-primary-500'}`}>
            {formatCurrency(order.total)}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div className="lg:col-span-2 space-y-4">
          {/* Items */}
          <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
            <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center justify-between font-bold text-gray-700">
              <div className="flex items-center gap-2">
                <Receipt size={18} />
                <span>{t('history.info.order_items')}</span>
              </div>
              <button
                onClick={toggleAll}
                title={allExpanded ? t('common.action.collapse_all') : t('common.action.expand_all')}
                className="p-1.5 text-gray-500 hover:text-gray-700 transition-colors rounded hover:bg-gray-200"
              >
                {allExpanded ? <ChevronsUp size={18} /> : <ChevronsDown size={18} />}
              </button>
            </div>
            <div className="divide-y divide-gray-100">
              {sortedItems.map((item, idx) => (
                <OrderItemRow
                  key={item.id || idx}
                  item={item}
                  index={idx}
                  isExpanded={expandedItems.has(idx)}
                  onToggle={toggleItem}
                  accentColor={CATEGORY_ACCENT[itemColorMap.get(item.instance_id) ?? 0]}
                  t={t}
                />
              ))}
            </div>
            <div className="p-4 bg-gray-50 border-t border-gray-200 space-y-2">
              {order.comp_total_amount > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-emerald-600">{t('checkout.breakdown.comp')}</span>
                  <span className="text-emerald-600">-{formatCurrency(order.comp_total_amount)}</span>
                </div>
              )}
              {breakdown && breakdown.manualItemDiscount > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-orange-500">{t('checkout.breakdown.manual_discount')}</span>
                  <span className="text-orange-500">-{formatCurrency(breakdown.manualItemDiscount)}</span>
                </div>
              )}
              {breakdown && breakdown.totalRuleDiscount > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-amber-600">{t('checkout.breakdown.rule_discount')}</span>
                  <span className="text-amber-600">-{formatCurrency(breakdown.totalRuleDiscount)}</span>
                </div>
              )}
              {breakdown && breakdown.totalRuleSurcharge > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-purple-500">{t('checkout.breakdown.rule_surcharge')}</span>
                  <span className="text-purple-500">+{formatCurrency(breakdown.totalRuleSurcharge)}</span>
                </div>
              )}
              {order.order_manual_discount_amount > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-orange-500">{t('checkout.breakdown.order_discount')}</span>
                  <span className="text-orange-500">-{formatCurrency(order.order_manual_discount_amount)}</span>
                </div>
              )}
              {order.order_manual_surcharge_amount > 0 && (
                <div className="flex justify-between text-sm">
                  <span className="text-purple-500">{t('checkout.breakdown.order_surcharge')}</span>
                  <span className="text-purple-500">+{formatCurrency(order.order_manual_surcharge_amount)}</span>
                </div>
              )}
              <div className="flex justify-between items-end pt-3 mt-1 border-t border-gray-200">
                <span className="text-gray-800 font-bold">{t('checkout.amount.total')}</span>
                <span className="text-xl font-bold text-primary-500">{formatCurrency(order.total)}</span>
              </div>
            </div>
          </div>

          {/* Credit Notes (退款记录) */}
          <CreditNoteSection order={order} onRefundCreated={onRefundCreated} onNavigateToCreditNote={onNavigateToCreditNote} />

          {/* Payments */}
          <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
            <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
              <CreditCard size={18} />
              <span>{t('history.payment.details')}</span>
            </div>
            <div className="divide-y divide-gray-100">
              {order.payments.length === 0 ? (
                <div className="p-4 text-center text-gray-400 text-sm">{t('history.payment.no_payments')}</div>
              ) : (
                order.payments.map((payment, idx) => (
                  <PaymentRow key={idx} payment={payment} t={t} />
                ))
              )}
            </div>
          </div>

          {/* Invoices (Verifactu 发票) */}
          <InvoiceSection order={order} />
        </div>

        {/* Timeline */}
        <div className="lg:col-span-1 bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden flex flex-col h-fit">
          <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
            <Clock size={18} />
            <span>{t('checkout.timeline.label')}</span>
          </div>
          <div className="p-4">
            {timelineEvents.length > 0 ? (
              <TimelineList events={timelineEvents} showNoteTags={true} />
            ) : (
              <div className="text-center text-gray-400 text-sm py-4">
                {t('timeline.empty')}
              </div>
            )}
          </div>
        </div>

      </div>

      {order && (
        <>
          <KitchenReprintModal
            isOpen={showKitchenReprint}
            orderId={order.order_id}
            onClose={() => setShowKitchenReprint(false)}
          />
          <LabelReprintModal
            isOpen={showLabelReprint}
            orderId={order.order_id}
            onClose={() => setShowLabelReprint(false)}
          />
          {showRefundModal && (
            <RefundModal
              order={order}
              onClose={() => setShowRefundModal(false)}
              onCreated={() => {
                setShowRefundModal(false);
                onRefundCreated?.();
              }}
            />
          )}
          {showAnulacionModal && (
            <AnulacionModal
              order={order}
              onClose={() => setShowAnulacionModal(false)}
              onCreated={() => {
                setShowAnulacionModal(false);
                onAnulacionCreated?.();
              }}
            />
          )}
          {showUpgradeModal && (
            <UpgradeInvoiceModal
              order={order}
              onClose={() => setShowUpgradeModal(false)}
              onCreated={() => {
                setShowUpgradeModal(false);
                onUpgradeCreated?.();
              }}
            />
          )}
        </>
      )}
    </div>
  );
};

// =============================================================================
// Order Item Row
// =============================================================================

interface OrderItemRowProps {
  item: ArchivedOrderItem;
  index: number;
  isExpanded: boolean;
  onToggle: (index: number) => void;
  accentColor?: string;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const OrderItemRow: React.FC<OrderItemRowProps> = React.memo(({ item, index, isExpanded, onToggle, accentColor, t }) => {
  const hasOptions = item.selected_options && item.selected_options.length > 0;
  const totalRuleDiscount = item.rule_discount_amount;
  const totalRuleSurcharge = item.rule_surcharge_amount;
  const manualDiscount = Currency.sub(item.discount_amount, item.rule_discount_amount).toNumber();
  const isFullyPaid = item.unpaid_quantity === 0;
  const isPartiallyPaid = !isFullyPaid && item.unpaid_quantity < item.quantity;

  return (
    <div>
      <div
        className="px-4 py-3 flex justify-between items-center cursor-pointer transition-colors select-none hover:bg-gray-50/50"
        onClick={() => onToggle(index)}
      >
        <div className="flex items-center gap-3 flex-1">
          <span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: accentColor || '#d1d5db' }} />
          <div className={`w-8 h-8 rounded flex items-center justify-center font-bold text-sm shrink-0
            ${item.is_comped ? 'bg-emerald-100 text-emerald-600' : isFullyPaid ? 'bg-green-100 text-green-600' : isPartiallyPaid ? 'bg-amber-100 text-amber-600' : 'bg-gray-100 text-gray-500'}
          `}>
            x{item.quantity}
          </div>
          <div className="flex-1 min-w-0">
            <div className="font-medium text-gray-800 flex items-center gap-2 flex-wrap">
              <span className="text-[0.625rem] text-blue-600 bg-blue-100 font-bold font-mono px-1.5 py-0.5 rounded border border-blue-200 shrink-0">
                #{item.instance_id.slice(-5)}
              </span>
              <span className="shrink-0">{item.name}</span>
              {item.spec_name && item.spec_name !== 'default' && (
                <span className="text-xs text-gray-500">({item.spec_name})</span>
              )}
              {item.is_comped && (
                item.instance_id.startsWith('stamp_reward::') ? (
                  <span className="text-[0.625rem] font-bold bg-amber-100 text-amber-700 px-1.5 py-0.5 rounded flex items-center gap-0.5">
                    <Stamp size={10} />
                    {t('checkout.stamp_reward')}
                  </span>
                ) : (
                  <span className="text-[0.625rem] font-bold bg-emerald-100 text-emerald-700 px-1.5 py-0.5 rounded flex items-center gap-0.5">
                    <Gift size={10} />
                    {t('checkout.comp.badge')}
                  </span>
                )
              )}
              {manualDiscount > 0 && (
                <span className="text-[0.625rem] font-bold bg-orange-100 text-orange-700 px-1.5 py-0.5 rounded-full">
                  -{formatCurrency(manualDiscount)}
                </span>
              )}
              {totalRuleDiscount > 0 && (
                <span className="text-[0.625rem] font-bold bg-amber-100 text-amber-700 px-1.5 py-0.5 rounded-full">
                  -{formatCurrency(totalRuleDiscount)}
                </span>
              )}
              {totalRuleSurcharge > 0 && (
                <span className="text-[0.625rem] font-bold bg-purple-100 text-purple-700 px-1.5 py-0.5 rounded-full">
                  +{formatCurrency(totalRuleSurcharge)}
                </span>
              )}
            </div>
            <div className="text-xs text-gray-400 flex items-center gap-2">
              <span>{formatCurrency(item.unit_price)}</span>
              <span>/ {t('checkout.amount.unit_price')}</span>
              {hasOptions && (
                <span className="flex items-center gap-1 ml-2 text-gray-400 bg-gray-100 px-1.5 py-0.5 rounded-md">
                  {isExpanded ? <ChevronUp size={10} /> : <ChevronDown size={10} />}
                  {t('common.label.details')}
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="font-bold text-gray-800 pl-4">{formatCurrency(item.line_total)}</div>
      </div>

      {isExpanded && hasOptions && (() => {
        const grouped = new Map<string, typeof item.selected_options>();
        for (const opt of item.selected_options) {
          const key = opt.attribute_name;
          if (!grouped.has(key)) grouped.set(key, []);
          grouped.get(key)!.push(opt);
        }
        return (
          <div className="px-16 pb-4 pt-0 animate-in slide-in-from-top-2 duration-200">
            <div className="p-3 bg-white rounded-lg border border-gray-100 space-y-1 shadow-sm">
              {[...grouped.entries()].map(([attrName, opts]) => (
                <div key={attrName} className="text-sm">
                  <span className="text-gray-500 font-medium">{attrName}: </span>
                  <span className="text-gray-800">
                    {opts!.map((opt, i) => (
                      <React.Fragment key={i}>
                        {i > 0 && ', '}
                        {opt.option_name}
                        {opt.price_modifier != null && opt.price_modifier !== 0 && (
                          <span className={`text-xs font-bold ml-0.5 ${opt.price_modifier > 0 ? 'text-orange-600' : 'text-green-600'}`}>
                            {opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)}
                          </span>
                        )}
                      </React.Fragment>
                    ))}
                  </span>
                </div>
              ))}
            </div>
          </div>
        );
      })()}
    </div>
  );
});

OrderItemRow.displayName = 'OrderItemRow';

// =============================================================================
// Payment Row
// =============================================================================

interface PaymentRowProps {
  payment: ArchivedPayment;
  t: (key: string, params?: Record<string, string | number>) => string;
}

const SPLIT_TYPE_CONFIG: Record<string, { label: string; bg: string; text: string }> = {
  ITEM_SPLIT: { label: 'history.payment.split_type.item', bg: 'bg-indigo-100', text: 'text-indigo-600' },
  AMOUNT_SPLIT: { label: 'history.payment.split_type.amount', bg: 'bg-cyan-100', text: 'text-cyan-600' },
  AA_SPLIT: { label: 'history.payment.split_type.aa', bg: 'bg-cyan-100', text: 'text-cyan-600' },
};

const PaymentRow: React.FC<PaymentRowProps> = React.memo(({ payment, t }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const isCash = /cash/i.test(payment.method);
  const hasItems = payment.split_items && payment.split_items.length > 0;
  // Infer split type: explicit field > fallback from split_items presence
  const effectiveSplitType = payment.split_type ?? (hasItems ? 'ITEM_SPLIT' : null);
  const splitConfig = effectiveSplitType ? SPLIT_TYPE_CONFIG[effectiveSplitType] ?? null : null;

  // Icon and color based on payment method
  const iconBg = isCash ? 'bg-green-100 text-green-600' : 'bg-indigo-100 text-indigo-600';
  const IconComponent = isCash ? Coins : CreditCard;

  return (
    <div className={`transition-colors ${isExpanded ? 'bg-gray-50/50' : ''}`}>
      <div
        className={`px-4 py-3 flex justify-between items-center ${hasItems ? 'cursor-pointer hover:bg-gray-50' : ''}`}
        onClick={() => hasItems && setIsExpanded(!isExpanded)}
      >
        <div className="flex items-center gap-3">
          <div className={`p-2 rounded-full ${iconBg}`}>
            <IconComponent size={16} />
          </div>
          <div>
            <div className="font-medium text-gray-800 flex items-center gap-2 flex-wrap">
              {isCash ? t('checkout.method.cash') : payment.method}
              {splitConfig && (
                <span className={`text-[0.625rem] font-bold px-1.5 py-0.5 rounded ${splitConfig.bg} ${splitConfig.text}`}>
                  {t(splitConfig.label)}
                </span>
              )}
              {payment.payment_id && (
                <span className="text-[0.625rem] text-emerald-600 bg-emerald-100 font-bold font-mono px-1.5 py-0.5 rounded">
                  #{String(payment.payment_id).slice(-5)}
                </span>
              )}
              {payment.cancelled && (
                <span className="text-xs bg-red-100 text-red-600 px-1.5 py-0.5 rounded font-bold flex items-center gap-1">
                  <Ban size={10} /> {t('common.status.cancelled')}
                </span>
              )}
              {hasItems && (
                <span className="text-gray-400">
                  {isExpanded ? <ChevronUp size={14} /> : <ChevronDown size={14} />}
                </span>
              )}
            </div>
            <div className="text-xs text-gray-400 flex items-center gap-2">
              <span>{new Date(payment.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}</span>
              {effectiveSplitType === 'AA_SPLIT' && payment.aa_shares && payment.aa_total_shares && (
                <span className="text-cyan-600 font-medium">
                  {payment.aa_shares}/{payment.aa_total_shares} {t('history.payment.aa_shares_unit')}
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="flex flex-col items-end">
          <div className={`font-bold ${payment.cancelled ? 'text-gray-400 line-through' : 'text-gray-800'}`}>
            {formatCurrency(payment.amount)}
          </div>
          {payment.cancel_reason && (
            <div className="text-xs text-red-500 mt-1">{payment.cancel_reason}</div>
          )}
        </div>
      </div>

      {isExpanded && hasItems && (
        <div className="px-14 pb-4 pt-0 animate-in slide-in-from-top-2 duration-200">
          <div className="p-3 bg-white rounded-lg border border-gray-100 space-y-2 shadow-sm">
            {payment.split_items.map((item, idx) => (
              <div key={idx} className="flex items-center gap-3 text-sm">
                <div className="w-7 h-7 rounded flex items-center justify-center font-bold text-xs shrink-0 bg-green-100 text-green-600">
                  x{item.quantity}
                </div>
                <div className="flex-1 min-w-0">
                  <div className="font-medium text-gray-800 flex items-center gap-2 flex-wrap">
                    <span className="text-[0.625rem] text-blue-600 bg-blue-100 font-bold font-mono px-1.5 py-0.5 rounded border border-blue-200">
                      #{item.instance_id.slice(-5)}
                    </span>
                    <span>{item.name}</span>
                  </div>
                  <div className="text-xs text-gray-400">
                    {formatCurrency(item.unit_price)} / {t('checkout.amount.unit_price')}
                  </div>
                </div>
                <div className="font-bold text-gray-800 pl-4 shrink-0">
                  {formatCurrency(Currency.mul(item.unit_price, item.quantity).toNumber())}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
});

PaymentRow.displayName = 'PaymentRow';
