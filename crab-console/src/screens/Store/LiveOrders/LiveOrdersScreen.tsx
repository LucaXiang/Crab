import React, { useMemo, useState } from 'react';
import {
  Radio, Wifi, WifiOff, Users, Clock, Receipt,
  X, CreditCard, ChevronDown, ChevronUp,
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { useLiveOrders } from '@/core/stores/useLiveOrdersStore';
import { formatCurrency, formatDateTime, timeAgo } from '@/utils/format';
import { Spinner } from '@/presentation/components/ui/Spinner';
import { EventTimeline } from '@/shared/components/Timeline';
import type { TimelineEvent } from '@/shared/components/Timeline';
import type { LiveOrderSnapshot, OrderEvent } from '@/core/types/live';

function connectionColor(state: string): string {
  switch (state) {
    case 'connected': return 'text-green-500';
    case 'connecting': case 'reconnecting': return 'text-amber-500';
    default: return 'text-slate-400';
  }
}

function statusBadge(status: string): string {
  switch (status) {
    case 'ACTIVE': return 'bg-blue-100 text-blue-700';
    case 'COMPLETED': return 'bg-green-100 text-green-700';
    case 'VOID': return 'bg-red-100 text-red-700';
    default: return 'bg-slate-100 text-slate-600';
  }
}

function orderTitle(order: LiveOrderSnapshot): string {
  if (order.queue_number) return `#${order.queue_number}`;
  if (order.table_name) return order.table_name;
  return order.order_id.slice(0, 8);
}

function toTimelineEvents(events: OrderEvent[]): TimelineEvent[] {
  return events.map(e => ({
    event_type: e.event_type,
    timestamp: e.timestamp,
    operator_name: e.operator_name,
    payload: e.payload as Record<string, any>,
  }));
}

export const LiveOrdersScreen: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const { sortedOrders, edgeOnline, connectionState } = useLiveOrders(token, storeId);
  const [selectedOrderId, setSelectedOrderId] = useState<string | null>(null);

  const selectedOrder = selectedOrderId ? sortedOrders.find(o => o.order_id === selectedOrderId) ?? null : null;

  const selectOrder = (orderId: string) => {
    setSelectedOrderId(prev => prev === orderId ? null : orderId);
  };

  return (
    <div className="max-w-7xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
      <div className="flex items-center justify-end">
        <div className="flex items-center gap-2 text-sm">
          {!edgeOnline ? (
            <><WifiOff className="w-4 h-4 text-red-500" /><span className="text-red-600 font-medium">{t('live.edge_offline')}</span></>
          ) : (
            <><Wifi className={`w-4 h-4 ${connectionColor(connectionState)}`} /><span className="text-slate-500">{t(`live.ws_${connectionState}`)}</span></>
          )}
        </div>
      </div>

      <div className="flex items-center gap-3">
        <div className="w-10 h-10 bg-primary-100 rounded-xl flex items-center justify-center">
          <Radio className="w-5 h-5 text-primary-600" />
        </div>
        <div>
          <h1 className="text-xl font-bold text-slate-900">{t('live.title')}</h1>
          <p className="text-sm text-slate-500">{sortedOrders.length} {t('live.active_orders')}</p>
        </div>
      </div>

      {connectionState === 'connecting' ? (
        <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>
      ) : sortedOrders.length === 0 ? (
        <div className="bg-white rounded-2xl border border-slate-200 p-12 text-center">
          <Receipt className="w-12 h-12 text-slate-300 mx-auto mb-3" />
          <p className="text-slate-500">{t('live.empty')}</p>
        </div>
      ) : (
        <div className="flex gap-6">
          {/* Order cards */}
          <div className="flex-1 min-w-0">
            <div className={`grid grid-cols-1 md:grid-cols-2 ${selectedOrderId ? '' : 'xl:grid-cols-3'} gap-4`}>
              {sortedOrders.map(order => (
                <button
                  key={order.order_id}
                  type="button"
                  className={`bg-white rounded-xl border p-4 transition-all text-left w-full cursor-pointer ${
                    selectedOrderId === order.order_id
                      ? 'border-primary-400 ring-2 ring-primary-100 shadow-md'
                      : 'border-slate-200 hover:border-primary-200 hover:shadow-sm'
                  }`}
                  onClick={() => selectOrder(order.order_id)}
                >
                  <div className="flex items-center justify-between mb-3">
                    <div className="flex items-center gap-2">
                      <span className="text-lg font-bold text-slate-900">{orderTitle(order)}</span>
                      <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${statusBadge(order.status)}`}>{order.status}</span>
                    </div>
                    <span className="text-lg font-bold text-slate-900">{formatCurrency(order.total)}</span>
                  </div>

                  <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-slate-500 mb-3">
                    {order.zone_name && <span>{order.zone_name}</span>}
                    {order.guest_count > 0 && <span className="inline-flex items-center gap-1"><Users className="w-3 h-3" />{order.guest_count}</span>}
                    {order.operator_name && <span>{order.operator_name}</span>}
                    <span className="inline-flex items-center gap-1"><Clock className="w-3 h-3" />{timeAgo(order.created_at)}</span>
                    {order.is_retail && <span className="text-primary-600 font-medium">{t('live.retail')}</span>}
                  </div>

                  <div className="border-t border-slate-100 pt-2 space-y-1">
                    {order.items.slice(0, 4).map(item => (
                      <div key={item.instance_id} className="flex items-center justify-between text-sm">
                        <span className="text-slate-700 truncate flex-1">
                          <span className="font-medium">{item.quantity}x</span> {item.name}
                          {item.is_comped && <span className="text-emerald-600 text-xs ml-1">({t('live.comped')})</span>}
                        </span>
                        <span className="text-slate-500 ml-2 shrink-0">{formatCurrency(item.line_total)}</span>
                      </div>
                    ))}
                    {order.items.length > 4 && <p className="text-xs text-slate-400">+{order.items.length - 4} {t('live.more_items')}</p>}
                  </div>

                  {order.paid_amount > 0 && (
                    <div className="mt-2 pt-2 border-t border-slate-100 flex justify-between text-xs">
                      <span className="text-slate-500">{t('live.paid')}</span>
                      <span className="text-green-600 font-medium">{formatCurrency(order.paid_amount)}</span>
                    </div>
                  )}
                  {order.remaining_amount > 0 && (
                    <div className={`flex justify-between text-xs ${order.paid_amount > 0 ? '' : 'mt-2 pt-2 border-t border-slate-100'}`}>
                      <span className="text-slate-500">{t('live.remaining')}</span>
                      <span className="text-amber-600 font-medium">{formatCurrency(order.remaining_amount)}</span>
                    </div>
                  )}
                </button>
              ))}
            </div>
          </div>

          {/* Detail panel (desktop) */}
          {selectedOrder && (
            <div className="w-[420px] shrink-0 hidden lg:block">
              <div className="bg-white rounded-2xl border border-slate-200 shadow-lg sticky top-6 overflow-hidden">
                <OrderDetailPanel order={selectedOrder} onClose={() => setSelectedOrderId(null)} t={t} />
              </div>
            </div>
          )}
        </div>
      )}

      {/* Mobile detail modal */}
      {selectedOrder && (
        <div
          className="lg:hidden fixed inset-0 z-50 bg-slate-900/60 backdrop-blur-sm flex items-end justify-center"
          onClick={() => setSelectedOrderId(null)}
        >
          <div className="bg-white rounded-t-2xl w-full max-h-[85vh] overflow-y-auto shadow-2xl" onClick={e => e.stopPropagation()} style={{ animation: 'slideUp 0.3s cubic-bezier(0.16, 1, 0.3, 1)' }}>
            <div className="sticky top-0 z-10 flex items-center justify-between px-5 py-4 border-b border-slate-100 bg-white/95 backdrop-blur-md">
              <span className="font-bold text-lg text-slate-900">{t('live.order_detail')}</span>
              <button 
                onClick={() => setSelectedOrderId(null)}
                className="p-2 -mr-2 text-slate-400 hover:text-slate-600 rounded-full hover:bg-slate-100 transition-colors"
              >
                <X className="w-5 h-5" />
              </button>
            </div>
            <OrderDetailPanel order={selectedOrder} onClose={() => setSelectedOrderId(null)} t={t} />
          </div>
        </div>
      )}
    </div>
  );
};

// --- Order Detail Panel ---

const OrderDetailPanel: React.FC<{
  order: LiveOrderSnapshot;
  onClose: () => void;
  t: (key: string) => string;
}> = ({ order, onClose, t }) => {
  const [showEvents, setShowEvents] = useState(true);
  const timelineEvents = useMemo(
    () => toTimelineEvents([...(order.events || [])].reverse()),
    [order.events],
  );

  return (
    <>
      {/* Header */}
      <div className="px-5 py-4 border-b border-slate-100 flex items-center justify-between bg-slate-50">
        <div>
          <h2 className="text-lg font-bold text-slate-900">{orderTitle(order)}</h2>
          <div className="flex items-center gap-2 mt-0.5">
            <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${statusBadge(order.status)}`}>{order.status}</span>
            {order.receipt_number && <span className="text-xs text-slate-400">#{order.receipt_number}</span>}
          </div>
        </div>
        <button type="button" className="p-1.5 hover:bg-slate-200 rounded-lg transition-colors cursor-pointer" onClick={onClose}>
          <X className="w-4 h-4 text-slate-500" />
        </button>
      </div>

      <div className="max-h-[calc(100vh-200px)] overflow-y-auto">
        {/* Order info */}
        <div className="px-5 py-3 space-y-2 text-sm border-b border-slate-100">
          {(order.zone_name || order.table_name) && (
            <DetailRow label={`${t('orders.zone')} / ${t('orders.table')}`} value={[order.zone_name, order.table_name].filter(Boolean).join(' · ')} />
          )}
          {order.guest_count > 0 && <DetailRow label={t('orders.guests')} value={String(order.guest_count)} />}
          {order.operator_name && <DetailRow label={t('live.operator')} value={order.operator_name} />}
          {order.member_name && <DetailRow label={t('live.member')} value={order.member_name} />}
          <DetailRow label={t('live.opened_at')} value={formatDateTime(order.start_time)} small />
          {order.note && (
            <div className="mt-1 p-2 bg-amber-50 border border-amber-100 rounded-lg text-xs text-amber-800">
              <span className="font-medium">{t('live.note')}:</span> {order.note}
            </div>
          )}
        </div>

        {/* Items */}
        <div className="px-5 py-3 border-b border-slate-100">
          <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{t('live.items')}</h3>
          <div className="space-y-2">
            {order.items.map(item => (
              <div key={item.instance_id} className="flex items-start justify-between text-sm gap-2">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-1.5">
                    <span className="font-medium text-slate-900">{item.quantity}x</span>
                    <span className="text-slate-800 truncate">{item.name}</span>
                    {item.is_comped && <span className="px-1.5 py-0.5 text-[10px] font-medium bg-emerald-100 text-emerald-700 rounded">{t('live.comped')}</span>}
                  </div>
                  {item.selected_specification?.is_multi_spec && <span className="text-xs text-slate-500 ml-5">{item.selected_specification.name}</span>}
                  {item.selected_options && item.selected_options.length > 0 && (
                    <div className="ml-5 text-xs text-slate-500">
                      {item.selected_options.map((opt, j) => (
                        <span key={j} className="inline-block mr-2">
                          {opt.option_name}{opt.quantity && opt.quantity > 1 ? ` x${opt.quantity}` : ''}
                          {opt.price_modifier && opt.price_modifier !== 0 && <span className="text-slate-400"> ({opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)})</span>}
                        </span>
                      ))}
                    </div>
                  )}
                  {item.manual_discount_percent != null && item.manual_discount_percent > 0 && <span className="ml-5 text-xs text-orange-500">-{item.manual_discount_percent}%</span>}
                  {item.note && <p className="ml-5 text-xs text-amber-600 italic">{item.note}</p>}
                </div>
                <span className="text-slate-900 font-medium shrink-0">{formatCurrency(item.line_total)}</span>
              </div>
            ))}
          </div>
        </div>

        {/* Price breakdown */}
        <div className="px-5 py-3 border-b border-slate-100 space-y-1.5 text-sm">
          <DetailRow label={t('live.subtotal')} value={formatCurrency(order.subtotal)} />
          {order.total_discount > 0 && <div className="flex justify-between"><span className="text-orange-500">{t('live.discount')}</span><span className="text-orange-500">-{formatCurrency(order.total_discount)}</span></div>}
          {order.total_surcharge > 0 && <div className="flex justify-between"><span className="text-purple-500">{t('live.surcharge')}</span><span className="text-purple-500">+{formatCurrency(order.total_surcharge)}</span></div>}
          {order.comp_total_amount > 0 && <div className="flex justify-between"><span className="text-emerald-600">{t('live.comp')}</span><span className="text-emerald-600">-{formatCurrency(order.comp_total_amount)}</span></div>}
          {order.tax > 0 && <DetailRow label={t('live.tax')} value={formatCurrency(order.tax)} />}
          <div className="flex justify-between pt-1.5 border-t border-slate-100 font-bold text-base">
            <span className="text-slate-900">{t('live.total')}</span>
            <span className="text-slate-900">{formatCurrency(order.total)}</span>
          </div>
        </div>

        {/* Payments */}
        <div className="px-5 py-3 border-b border-slate-100">
          <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{t('live.payments')}</h3>
          {order.payments.length === 0 ? (
            <p className="text-sm text-slate-400 italic">{t('live.no_payments')}</p>
          ) : (
            <>
              <div className="space-y-2">
                {order.payments.map(payment => (
                  <div key={payment.payment_id} className={`flex items-center justify-between text-sm ${payment.cancelled ? 'opacity-50' : ''}`}>
                    <div className="flex items-center gap-2">
                      <CreditCard className="w-3.5 h-3.5 text-slate-400" />
                      <span className="text-slate-700 capitalize">{payment.method}</span>
                      {payment.cancelled && <span className="px-1.5 py-0.5 text-[10px] bg-red-100 text-red-600 rounded font-medium">{t('live.cancelled')}</span>}
                    </div>
                    <span className={`font-medium ${payment.cancelled ? 'text-slate-400 line-through' : 'text-green-600'}`}>{formatCurrency(payment.amount)}</span>
                  </div>
                ))}
              </div>
              <div className="mt-3 pt-2 border-t border-slate-100 space-y-1">
                <div className="flex justify-between text-sm">
                  <span className="text-slate-500">{t('live.paid')}</span>
                  <span className="text-green-600 font-medium">{formatCurrency(order.paid_amount)}</span>
                </div>
                {order.remaining_amount > 0 && (
                  <div className="flex justify-between text-sm">
                    <span className="text-slate-500">{t('live.remaining')}</span>
                    <span className="text-amber-600 font-medium">{formatCurrency(order.remaining_amount)}</span>
                  </div>
                )}
              </div>
            </>
          )}
        </div>

        {/* Events timeline — using shared component */}
        {timelineEvents.length > 0 && (
          <div className="px-5 py-3">
            <button
              type="button"
              className="flex items-center justify-between w-full text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2 cursor-pointer"
              onClick={() => setShowEvents(!showEvents)}
            >
              <span className="flex items-center gap-1.5">
                <Clock className="w-3.5 h-3.5" />
                {t('live.events')} ({timelineEvents.length})
              </span>
              {showEvents ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
            </button>
            {showEvents && (
              <EventTimeline events={timelineEvents} t={t} />
            )}
          </div>
        )}
      </div>
    </>
  );
};

const DetailRow: React.FC<{ label: string; value: string; small?: boolean }> = ({ label, value, small }) => (
  <div className="flex justify-between">
    <span className="text-slate-500">{label}</span>
    <span className={`text-slate-900 ${small ? 'text-xs' : ''}`}>{value}</span>
  </div>
);
