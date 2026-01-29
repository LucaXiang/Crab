import React, { useState, useEffect, useCallback, useMemo } from 'react';
import type { ArchivedOrderDetail, ArchivedOrderItem, ArchivedPayment, ArchivedEvent } from '@/core/domain/types';
import type { OrderEvent, OrderEventType, EventPayload } from '@/core/domain/types/orderEvent';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency';
import { Receipt, Calendar, Printer, CreditCard, Coins, Clock, ChevronDown, ChevronUp, ChevronsDown, ChevronsUp, Ban } from 'lucide-react';
import { Permission } from '@/core/domain/types';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { TimelineList } from '@/shared/components/TimelineList';

interface HistoryDetailProps {
  order?: ArchivedOrderDetail;
  onReprint: () => void;
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
  const rawPayload = event.payload as Record<string, unknown> | null;
  const payload: EventPayload = (rawPayload && Object.keys(rawPayload).length > 0)
    ? rawPayload as unknown as EventPayload
    : { type: eventType } as unknown as EventPayload;

  return {
    event_id: event.event_id,
    sequence: index,
    order_id: '',
    timestamp: event.timestamp,
    operator_id: '',
    operator_name: '',
    command_id: '',
    event_type: eventType,
    payload,
  };
}

export const HistoryDetail: React.FC<HistoryDetailProps> = ({ order, onReprint }) => {
  const { t } = useI18n();
  const [expandedItems, setExpandedItems] = useState<Set<number>>(new Set());

  // Convert archived events to OrderEvent format for TimelineList
  const timelineEvents = useMemo(() => {
    if (!order?.timeline) return [];
    return order.timeline.map((event, index) => convertArchivedEventToOrderEvent(event, index));
  }, [order?.timeline]);

  useEffect(() => {
    setExpandedItems(new Set());
  }, [order]);

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

  const toggleAll = () => {
    if (!order) return;
    if (expandedItems.size === order.items.length) {
      setExpandedItems(new Set());
    } else {
      setExpandedItems(new Set(order.items.map((_, i) => i)));
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
  const isMoved = order.status === 'MOVED';

  return (
    <div className="max-w-4xl mx-auto space-y-6">
      {/* Header */}
      <div className="bg-white rounded-2xl p-6 shadow-sm border border-gray-200 flex justify-between items-start">
        <div>
          <div className="flex items-center gap-3 mb-2">
            <h1 className={`text-2xl font-bold ${isVoid || isMerged || isMoved ? 'text-gray-500 line-through' : 'text-gray-900'}`}>
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
            {isMoved && (
              <span className="px-2 py-1 bg-blue-100 text-blue-700 text-xs font-bold rounded uppercase">
                {t('history.status.moved')}
              </span>
            )}
            <ProtectedGate permission={Permission.RECEIPTS_REPRINT}>
              <button
                onClick={onReprint}
                className="flex items-center gap-1.5 px-3 py-1 bg-white border border-gray-300 rounded-lg shadow-sm text-sm font-medium text-gray-700 hover:bg-gray-50 hover:text-gray-900 transition-colors"
              >
                <Printer size={16} />
                <span>{t('history.action.reprint')}</span>
              </button>
            </ProtectedGate>
          </div>
          <div className="flex gap-4 text-sm text-gray-500">
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
          </div>

          {/* Void Information */}
          {isVoid && (order.void_type || order.loss_reason || order.loss_amount !== null) && (
            <div className="mt-4 pt-3 border-t border-red-100 flex flex-wrap gap-6 text-sm">
              {order.void_type && (
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs text-red-400 font-medium uppercase">{t('common.status.void')}</span>
                  <span className="text-red-700 font-medium">{t(`history.void_type.${order.void_type}`)}</span>
                </div>
              )}
              {order.loss_reason && (
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs text-gray-400 font-medium uppercase">{t('common.label.description')}</span>
                  <span className="text-gray-700 font-medium">{t(`history.loss_reason.${order.loss_reason}`)}</span>
                </div>
              )}
              {order.loss_amount !== null && order.loss_amount !== undefined && (
                <div className="flex flex-col gap-0.5">
                  <span className="text-xs text-red-400 font-medium uppercase">{t('common.label.loss_amount')}</span>
                  <span className="text-red-700 font-bold">{formatCurrency(order.loss_amount)}</span>
                </div>
              )}
            </div>
          )}
        </div>
        <div className="text-right">
          <div className="text-sm text-gray-500 uppercase font-bold tracking-wider mb-1">{t('history.info.total_amount')}</div>
          <div className={`text-3xl font-bold ${isVoid || isMerged || isMoved ? 'text-gray-400 line-through' : 'text-primary-500'}`}>
            {formatCurrency(order.total)}
          </div>
        </div>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-6">
          {/* Items */}
          <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
            <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center justify-between font-bold text-gray-700">
              <div className="flex items-center gap-2">
                <Receipt size={18} />
                <span>{t('history.info.order_items')}</span>
              </div>
              <button
                onClick={toggleAll}
                title={expandedItems.size === order.items.length ? t('common.action.collapse_all') : t('common.action.expand_all')}
                className="p-1.5 text-gray-500 hover:text-gray-700 transition-colors rounded hover:bg-gray-200"
              >
                {expandedItems.size === order.items.length ? <ChevronsUp size={18} /> : <ChevronsDown size={18} />}
              </button>
            </div>
            <div className="divide-y divide-gray-100">
              {order.items.map((item, idx) => (
                <OrderItemRow
                  key={item.id || idx}
                  item={item}
                  index={idx}
                  isExpanded={expandedItems.has(idx)}
                  onToggle={toggleItem}
                  t={t}
                />
              ))}
            </div>
            <div className="p-5 bg-gray-50 border-t border-gray-200 space-y-2">
              {order.total_discount > 0 && (
                <div className="flex justify-between text-sm text-gray-600">
                  <span>{t('checkout.amount.discount')}</span>
                  <span className="text-red-600">-{formatCurrency(order.total_discount)}</span>
                </div>
              )}
              {order.total_surcharge > 0 && (
                <div className="flex justify-between text-sm text-gray-600">
                  <span>{t('checkout.amount.surcharge')}</span>
                  <span className="text-purple-600">+{formatCurrency(order.total_surcharge)}</span>
                </div>
              )}
              <div className="flex justify-between items-end pt-3 mt-1 border-t border-gray-200">
                <span className="text-gray-800 font-bold">{t('checkout.amount.total')}</span>
                <span className="text-xl font-bold text-primary-500">{formatCurrency(order.total)}</span>
              </div>
            </div>
          </div>

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
  t: (key: string, params?: Record<string, string | number>) => string;
}

const OrderItemRow: React.FC<OrderItemRowProps> = React.memo(({ item, index, isExpanded, onToggle, t }) => {
  const hasOptions = item.selected_options && item.selected_options.length > 0;
  const hasDiscount = item.discount_amount > 0;
  const hasSurcharge = item.surcharge_amount > 0;
  const isFullyPaid = item.unpaid_quantity === 0;

  return (
    <div className={`transition-colors ${isExpanded ? 'bg-gray-50/50' : ''}`}>
      <div
        className="p-4 flex justify-between items-center cursor-pointer hover:bg-gray-50 transition-colors select-none"
        onClick={() => onToggle(index)}
      >
        <div className="flex items-center gap-4 flex-1">
          <div className={`w-8 h-8 rounded flex items-center justify-center font-bold text-sm shrink-0
            ${isFullyPaid ? 'bg-green-100 text-green-600' : 'bg-gray-100 text-gray-500'}
          `}>
            x{item.quantity}
          </div>
          <div className="flex-1 min-w-0">
            <div className="font-medium text-gray-800 flex items-center gap-2 flex-wrap">
              <span>{item.name}</span>
              {item.spec_name && (
                <span className="text-xs text-gray-500">({item.spec_name})</span>
              )}
              <span className="text-[0.625rem] text-blue-600 bg-blue-100 font-bold font-mono px-1.5 py-0.5 rounded">
                #{item.instance_id.slice(-5)}
              </span>
              {hasDiscount && (
                <span className="text-[0.625rem] font-bold bg-red-100 text-red-600 px-1.5 py-0.5 rounded-full">
                  -{formatCurrency(item.discount_amount)}
                </span>
              )}
              {hasSurcharge && (
                <span className="text-[0.625rem] font-bold bg-purple-100 text-purple-600 px-1.5 py-0.5 rounded-full">
                  +{formatCurrency(item.surcharge_amount)}
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

      {isExpanded && hasOptions && (
        <div className="px-16 pb-4 pt-0 animate-in slide-in-from-top-2 duration-200">
          <div className="p-3 bg-white rounded-lg border border-gray-100 space-y-1 shadow-sm">
            {item.selected_options.map((opt, idx) => (
              <div key={idx} className="flex justify-between items-center text-sm">
                <div className="flex items-center gap-2">
                  <span className="text-gray-500 font-medium">{opt.attribute_name}:</span>
                  <span className="text-gray-800">{opt.option_name}</span>
                </div>
                {opt.price_modifier !== 0 && (
                  <span className={`text-xs font-bold ${opt.price_modifier > 0 ? 'text-orange-600' : 'text-green-600'}`}>
                    {opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)}
                  </span>
                )}
              </div>
            ))}
          </div>
        </div>
      )}
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

const PaymentRow: React.FC<PaymentRowProps> = React.memo(({ payment, t }) => {
  const [isExpanded, setIsExpanded] = useState(false);
  const isCash = /cash/i.test(payment.method);
  const hasItems = payment.split_items && payment.split_items.length > 0;

  return (
    <div className={`transition-colors ${isExpanded ? 'bg-gray-50/50' : ''}`}>
      <div
        className={`p-4 flex justify-between items-center ${hasItems ? 'cursor-pointer hover:bg-gray-50' : ''}`}
        onClick={() => hasItems && setIsExpanded(!isExpanded)}
      >
        <div className="flex items-center gap-3">
          <div className={`p-2 rounded-full ${isCash ? 'bg-green-100 text-green-600' : 'bg-indigo-100 text-indigo-600'}`}>
            {isCash ? <Coins size={16} /> : <CreditCard size={16} />}
          </div>
          <div>
            <div className="font-medium text-gray-800 flex items-center gap-2">
              {isCash ? t('checkout.method.cash') : payment.method}
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
            <div className="text-xs text-gray-400">
              {new Date(payment.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}
            </div>
          </div>
        </div>
        <div className="flex flex-col items-end">
          <div className={`font-bold ${payment.cancelled ? 'text-gray-400 line-through' : 'text-gray-800'}`}>
            {formatCurrency(payment.amount)}
          </div>
          {payment.note && (
            <div className="text-xs text-gray-500 mt-1">{payment.note}</div>
          )}
          {payment.cancel_reason && (
            <div className="text-xs text-red-500 mt-1">{payment.cancel_reason}</div>
          )}
        </div>
      </div>

      {isExpanded && hasItems && (
        <div className="px-14 pb-4 pt-0 animate-in slide-in-from-top-2 duration-200">
          <div className="p-3 bg-white rounded-lg border border-gray-100 space-y-2 shadow-sm">
            {payment.split_items.map((item, idx) => (
              <div key={idx} className="flex justify-between items-start text-sm">
                <span className="text-gray-800 font-medium">
                  {item.name} <span className="text-gray-500">x{item.quantity}</span>
                  <span className="ml-1.5 text-[0.625rem] text-blue-600 bg-blue-100 font-bold font-mono px-1.5 py-0.5 rounded">
                    #{item.instance_id.slice(-5)}
                  </span>
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
});

PaymentRow.displayName = 'PaymentRow';
