import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import {
  Search, Clock, ChevronRight, Receipt, Calendar, CreditCard, Coins,
  Gift, Ban, ChevronDown, ChevronUp, Cloud, Wifi, X, Users,
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getOrders, getOrderDetail } from '@/infrastructure/api/orders';
import { ApiError } from '@/infrastructure/api/client';
import { formatCurrency } from '@/utils/format';
import { Spinner } from '@/presentation/components/ui/Spinner';
import { TimelineCard } from '@/shared/components/Timeline';
import type { TimelineEvent } from '@/shared/components/Timeline';
import type { OrderSummary, OrderDetailResponse, OrderItem, OrderPayment, OrderEvent } from '@/core/types/order';

const ACCENT_COLORS = [
  '#ef4444', '#f97316', '#eab308', '#22c55e', '#06b6d4',
  '#3b82f6', '#8b5cf6', '#ec4899', '#14b8a6', '#f43f5e',
];

function toTimelineEvents(events: OrderEvent[]): TimelineEvent[] {
  return events.map(e => ({
    event_type: e.event_type,
    timestamp: e.timestamp,
    operator_name: e.operator_name,
    payload: e.data ? (() => { try { return JSON.parse(e.data!); } catch { return {}; } })() : {},
  }));
}

export const OrdersScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [orders, setOrders] = useState<OrderSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentPage, setCurrentPage] = useState(1);
  const [hasMore, setHasMore] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [search, setSearch] = useState('');

  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [detail, setDetail] = useState<OrderDetailResponse | null>(null);
  const [detailLoading, setDetailLoading] = useState(false);

  const loadPage = useCallback(async (page: number, reset: boolean) => {
    if (!token) return;
    try {
      const batch = await getOrders(token, storeId, page, 20);
      if (reset) setOrders(batch); else setOrders(prev => [...prev, ...batch]);
      setHasMore(batch.length === 20);
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); }
    }
  }, [token, storeId, clearAuth, navigate]);

  useEffect(() => {
    (async () => {
      setLoading(true);
      await loadPage(1, true);
      setLoading(false);
    })();
  }, [loadPage]);

  // Auto-select first order (desktop only)
  useEffect(() => {
    if (orders.length > 0 && !selectedId) {
      setSelectedId(orders[0].source_id);
    }
  }, [orders, selectedId]);

  // Load detail when selection changes
  useEffect(() => {
    if (!token || !selectedId) return;
    let cancelled = false;
    (async () => {
      setDetailLoading(true);
      try {
        const res = await getOrderDetail(token, storeId, selectedId);
        if (!cancelled) setDetail(res);
      } catch {
        if (!cancelled) setDetail(null);
      } finally {
        if (!cancelled) setDetailLoading(false);
      }
    })();
    return () => { cancelled = true; };
  }, [token, storeId, selectedId]);

  const handleLoadMore = async () => {
    const next = currentPage + 1;
    setLoadingMore(true);
    setCurrentPage(next);
    await loadPage(next, false);
    setLoadingMore(false);
  };

  const filteredOrders = search
    ? orders.filter(o => (o.receipt_number ?? o.source_id).toLowerCase().includes(search.toLowerCase()))
    : orders;

  return (
    <>
      {/* ── Desktop: split pane ── */}
      <div className="hidden md:flex h-full overflow-hidden">
        {/* Left sidebar: order list */}
        <div className="w-80 lg:w-96 bg-white border-r border-slate-200 flex flex-col shrink-0">
          <div className="p-4 border-b border-slate-100 shrink-0">
            <h2 className="text-lg font-bold text-slate-800 flex items-center gap-2 mb-3">
              <Clock className="w-5 h-5 text-primary-500" />
              <span>{t('orders.title')}</span>
            </h2>
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400 w-4 h-4" />
              <input
                type="text"
                placeholder={`${t('orders.receipt')}...`}
                value={search}
                onChange={e => setSearch(e.target.value)}
                className="w-full bg-slate-100 pl-9 pr-4 py-2.5 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-100 transition-all"
              />
            </div>
          </div>

          <div className="flex-1 overflow-y-auto relative">
            {loading && (
              <div className="absolute inset-0 bg-white/50 flex items-center justify-center z-10">
                <Spinner className="w-8 h-8 text-primary-500" />
              </div>
            )}
            {filteredOrders.length === 0 && !loading ? (
              <div className="flex flex-col items-center justify-center p-8 text-center text-slate-400 gap-4">
                <span className="text-sm">{t('orders.empty')}</span>
              </div>
            ) : (
              <div className="divide-y divide-slate-50">
                {filteredOrders.map(order => {
                  const isSelected = selectedId === order.source_id;
                  const isVoid = order.status === 'VOID';
                  const isMerged = order.status === 'MERGED';
                  return (
                    <button
                      key={order.id}
                      onClick={() => setSelectedId(order.source_id)}
                      className={`w-full p-4 text-left transition-colors flex justify-between items-start group cursor-pointer ${isSelected ? 'bg-primary-50' : 'hover:bg-slate-50'}`}
                    >
                      <div>
                        <div className="flex items-center gap-2 mb-1">
                          <span className={`font-bold ${isSelected ? 'text-primary-600' : 'text-slate-800'}`}>
                            {order.receipt_number ?? order.source_id.slice(0, 8)}
                          </span>
                        </div>
                        <div className="flex gap-2 text-[0.625rem] items-center mb-1">
                          <span className={`px-1.5 py-0.5 rounded-full font-bold ${isVoid ? 'bg-red-100 text-red-600' : isMerged ? 'bg-blue-100 text-blue-700' : 'bg-green-100 text-green-700'}`}>
                            {isVoid ? t('orders.void') : isMerged ? t('orders.merged') : t('orders.completed')}
                          </span>
                        </div>
                        <div className="text-xs text-slate-400 font-mono">
                          {new Date(order.end_time ?? order.synced_at).toLocaleString([], { hour12: false })}
                        </div>
                      </div>
                      <div className="text-right">
                        <div className={`font-bold ${isVoid || isMerged ? 'text-slate-400 line-through' : 'text-slate-800'}`}>
                          {order.total != null ? formatCurrency(order.total) : '\u2014'}
                        </div>
                        <ChevronRight className={`w-4 h-4 ml-auto mt-1 transition-opacity ${isSelected ? 'text-primary-400 opacity-100' : 'text-slate-300 opacity-0 group-hover:opacity-100'}`} />
                      </div>
                    </button>
                  );
                })}
              </div>
            )}
          </div>

          <div className="p-4 border-t border-slate-100 bg-slate-50 flex justify-center text-sm">
            <button
              onClick={handleLoadMore}
              disabled={!hasMore || loadingMore}
              className="px-4 py-2 rounded-lg border border-slate-200 bg-white text-slate-700 hover:bg-slate-100 disabled:opacity-50 disabled:cursor-default flex items-center gap-2 cursor-pointer"
            >
              {loadingMore ? <Spinner className="w-4 h-4" /> : null}
              <span>{hasMore ? t('orders.load_more') : t('orders.empty')}</span>
            </button>
          </div>
        </div>

        {/* Right: order detail */}
        <div className="flex-1 overflow-y-auto bg-slate-50 p-4 lg:p-6" style={{ scrollbarGutter: 'stable' }}>
          {detailLoading ? (
            <div className="h-full flex items-center justify-center">
              <Spinner className="w-10 h-10 text-primary-500" />
            </div>
          ) : detail ? (
            <OrderDetail detail={detail} orderKey={selectedId ?? ''} t={t} />
          ) : (
            <div className="h-full flex flex-col items-center justify-center text-slate-300">
              <Receipt className="w-16 h-16 mb-4 opacity-50" />
              <p>{t('orders.empty')}</p>
            </div>
          )}
        </div>
      </div>

      {/* ── Mobile: card list + bottom sheet ── */}
      <div className="md:hidden px-4 py-4 space-y-4">
        {/* Header + Search */}
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-primary-100 rounded-xl flex items-center justify-center">
            <Clock className="w-5 h-5 text-primary-600" />
          </div>
          <div className="flex-1">
            <h1 className="text-xl font-bold text-slate-900">{t('orders.title')}</h1>
            <p className="text-sm text-slate-500">{filteredOrders.length} {t('orders.title')}</p>
          </div>
        </div>

        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-slate-400 w-4 h-4" />
          <input
            type="text"
            placeholder={`${t('orders.receipt')}...`}
            value={search}
            onChange={e => setSearch(e.target.value)}
            className="w-full bg-white border border-slate-200 pl-9 pr-4 py-2.5 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-100 transition-all"
          />
        </div>

        {loading ? (
          <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>
        ) : filteredOrders.length === 0 ? (
          <div className="bg-white rounded-2xl border border-slate-200 p-12 text-center">
            <Receipt className="w-12 h-12 text-slate-300 mx-auto mb-3" />
            <p className="text-slate-500">{t('orders.empty')}</p>
          </div>
        ) : (
          <div className="space-y-3">
            {filteredOrders.map(order => {
              const isVoid = order.status === 'VOID';
              const isMerged = order.status === 'MERGED';
              return (
                <button
                  key={order.id}
                  type="button"
                  onClick={() => setSelectedId(order.source_id)}
                  className="bg-white rounded-xl border border-slate-200 p-4 w-full text-left transition-all hover:border-primary-200 hover:shadow-sm cursor-pointer"
                >
                  <div className="flex items-center justify-between mb-2">
                    <div className="flex items-center gap-2">
                      <span className="text-lg font-bold text-slate-900">
                        {order.receipt_number ?? order.source_id.slice(0, 8)}
                      </span>
                      <span className={`px-2 py-0.5 rounded-full text-xs font-medium ${isVoid ? 'bg-red-100 text-red-600' : isMerged ? 'bg-blue-100 text-blue-700' : 'bg-green-100 text-green-700'}`}>
                        {isVoid ? t('orders.void') : isMerged ? t('orders.merged') : t('orders.completed')}
                      </span>
                    </div>
                    <span className={`text-lg font-bold ${isVoid || isMerged ? 'text-slate-400 line-through' : 'text-slate-900'}`}>
                      {order.total != null ? formatCurrency(order.total) : '\u2014'}
                    </span>
                  </div>
                  <div className="flex items-center gap-3 text-xs text-slate-500">
                    <span className="flex items-center gap-1">
                      <Clock className="w-3 h-3" />
                      {new Date(order.end_time ?? order.synced_at).toLocaleString([], { hour12: false })}
                    </span>
                  </div>
                </button>
              );
            })}

            <div className="flex justify-center pt-2 pb-4">
              <button
                onClick={handleLoadMore}
                disabled={!hasMore || loadingMore}
                className="px-4 py-2 rounded-lg border border-slate-200 bg-white text-slate-700 hover:bg-slate-100 disabled:opacity-50 disabled:cursor-default flex items-center gap-2 cursor-pointer text-sm"
              >
                {loadingMore ? <Spinner className="w-4 h-4" /> : null}
                <span>{hasMore ? t('orders.load_more') : t('orders.empty')}</span>
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Mobile bottom sheet modal */}
      {selectedId && (
        <div
          className="md:hidden fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-end justify-center"
          onClick={() => setSelectedId(null)}
        >
          <div
            className="bg-white rounded-t-2xl w-full max-h-[90vh] overflow-y-auto"
            onClick={e => e.stopPropagation()}
            style={{ animation: 'slideUp 0.25s ease-out' }}
          >
            {/* Modal header */}
            <div className="sticky top-0 z-10 px-5 py-3 border-b border-slate-100 bg-white/95 backdrop-blur flex items-center justify-between">
              <span className="text-lg font-bold text-slate-900">{selectedId.slice(0, 8)}</span>
              <button type="button" className="p-1.5 hover:bg-slate-200 rounded-lg transition-colors cursor-pointer" onClick={() => setSelectedId(null)}>
                <X className="w-4 h-4 text-slate-500" />
              </button>
            </div>
            <div className="p-4">
              {detailLoading ? (
                <div className="flex items-center justify-center py-12"><Spinner className="w-8 h-8 text-primary-500" /></div>
              ) : detail ? (
                <MobileOrderDetail detail={detail} orderKey={selectedId} t={t} />
              ) : (
                <div className="text-center text-slate-400 py-8">{t('orders.empty')}</div>
              )}
            </div>
          </div>
        </div>
      )}
    </>
  );
};

/* ═══════════════════════════════════════════════════════════════════════
   Desktop Order Detail (2-column grid)
   ═══════════════════════════════════════════════════════════════════════ */

const OrderDetail: React.FC<{
  detail: OrderDetailResponse;
  orderKey: string;
  t: (key: string) => string;
}> = ({ detail, orderKey, t }) => {
  const d = detail.detail;
  const isVoid = d.void_type != null;
  const timelineEvents = useMemo(() => toTimelineEvents(d.events ?? []), [d.events]);

  const categoryColorMap = new Map<string, number>();
  let ci = 0;
  for (const item of d.items) {
    const cat = item.category_name ?? '__none__';
    if (!categoryColorMap.has(cat)) { categoryColorMap.set(cat, ci % ACCENT_COLORS.length); ci++; }
  }

  return (
    <div className="max-w-6xl mx-auto space-y-4">
      <OrderHeader detail={detail} orderKey={orderKey} isVoid={isVoid} t={t} />

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        <div className="lg:col-span-2 space-y-4">
          <ItemsCard items={d.items} categoryColorMap={categoryColorMap} detail={detail} t={t} />
          <PaymentsCard payments={d.payments} t={t} />
          {detail.desglose.length > 0 && <TaxCard desglose={detail.desglose} t={t} />}
        </div>
        <div className="lg:col-span-1">
          <TimelineCard events={timelineEvents} t={t} />
        </div>
      </div>
    </div>
  );
};

/* ═══════════════════════════════════════════════════════════════════════
   Mobile Order Detail (single column, compact)
   ═══════════════════════════════════════════════════════════════════════ */

const MobileOrderDetail: React.FC<{
  detail: OrderDetailResponse;
  orderKey: string;
  t: (key: string) => string;
}> = ({ detail, orderKey, t }) => {
  const d = detail.detail;
  const isVoid = d.void_type != null;
  const timelineEvents = useMemo(() => toTimelineEvents(d.events ?? []), [d.events]);
  const [showTimeline, setShowTimeline] = useState(false);

  return (
    <div className="space-y-4">
      {/* Compact header */}
      <div className="flex justify-between items-start">
        <div>
          <div className="flex items-center gap-2 mb-1">
            <span className={`text-lg font-bold ${isVoid ? 'text-slate-400 line-through' : 'text-slate-900'}`}>{orderKey}</span>
            {isVoid && <span className="px-2 py-0.5 bg-red-100 text-red-700 text-xs font-bold rounded uppercase">{t('orders.voided')}</span>}
          </div>
          <div className="flex flex-wrap gap-3 text-xs text-slate-500">
            {d.operator_name && <span>{d.operator_name}</span>}
            <span className="flex items-center gap-1"><Calendar className="w-3.5 h-3.5" />{new Date(d.start_time).toLocaleDateString()}</span>
            <span className="flex items-center gap-1"><Clock className="w-3.5 h-3.5" />{new Date(d.start_time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}</span>
          </div>
          <div className="flex items-center gap-3 mt-1 text-xs text-slate-400">
            {d.zone_name && <span>{d.zone_name}{d.table_name ? ` · ${d.table_name}` : ''}</span>}
            {d.guest_count != null && d.guest_count > 0 && <span className="flex items-center gap-1"><Users className="w-3 h-3" />{d.guest_count}</span>}
            {detail.source === 'cache'
              ? <span className="flex items-center gap-1"><Cloud className="w-3 h-3" />{t('orders.source_cache')}</span>
              : <span className="flex items-center gap-1"><Wifi className="w-3 h-3" />{t('orders.source_edge')}</span>
            }
          </div>
        </div>
        <div className="text-right shrink-0 pl-4">
          <p className={`text-2xl font-bold ${isVoid ? 'text-slate-400 line-through' : 'text-primary-500'}`}>
            {formatCurrency(d.paid_amount)}
          </p>
        </div>
      </div>

      {isVoid && (
        <div className="p-3 bg-red-50 border border-red-100 rounded-xl text-sm space-y-1">
          <p className="text-red-700 font-medium">{d.void_type}</p>
          {d.loss_reason && <p className="text-slate-600">{d.loss_reason}</p>}
          {d.loss_amount != null && d.loss_amount > 0 && <p className="text-orange-600 font-bold">{t('orders.loss_amount')}: {formatCurrency(d.loss_amount)}</p>}
        </div>
      )}

      {/* Items */}
      <div className="border-t border-slate-100 pt-3">
        <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{t('orders.items')}</h3>
        <div className="space-y-2">
          {d.items.map((item, i) => (
            <div key={i} className="flex items-start justify-between text-sm gap-2">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-1.5 flex-wrap">
                  <span className="font-medium text-slate-900">{item.quantity}x</span>
                  <span className="text-slate-800">{item.name}</span>
                  {item.spec_name && <span className="text-xs text-slate-500">({item.spec_name})</span>}
                  {item.is_comped && <span className="px-1.5 py-0.5 text-[10px] font-bold bg-emerald-100 text-emerald-700 rounded">{t('orders.comped')}</span>}
                </div>
                {item.options.length > 0 && (
                  <div className="ml-5 text-xs text-slate-500">
                    {item.options.map((opt, j) => (
                      <span key={j} className="inline-block mr-2">
                        {opt.option_name}
                        {opt.price > 0 && <span className="text-orange-500 ml-0.5">+{formatCurrency(opt.price)}</span>}
                      </span>
                    ))}
                  </div>
                )}
              </div>
              <span className="text-slate-900 font-medium shrink-0">{formatCurrency(item.line_total)}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Price summary */}
      <div className="border-t border-slate-100 pt-3 space-y-1.5 text-sm">
        {d.comp_total_amount > 0 && <SummaryRow label={t('orders.comped')} value={`-${formatCurrency(d.comp_total_amount)}`} color="text-emerald-600" />}
        {d.discount_amount > 0 && <SummaryRow label={t('orders.discount')} value={`-${formatCurrency(d.discount_amount)}`} color="text-orange-500" />}
        {d.surcharge_amount > 0 && <SummaryRow label={t('orders.surcharge')} value={`+${formatCurrency(d.surcharge_amount)}`} color="text-purple-500" />}
        <div className="flex justify-between pt-2 border-t border-slate-100 font-bold">
          <span className="text-slate-900">{t('orders.total')}</span>
          <span className="text-primary-500">{formatCurrency(d.paid_amount)}</span>
        </div>
      </div>

      {/* Payments */}
      <div className="border-t border-slate-100 pt-3">
        <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{t('orders.payments')}</h3>
        {d.payments.length === 0 ? (
          <p className="text-sm text-slate-400 italic">{t('orders.empty')}</p>
        ) : (
          <div className="space-y-2">
            {d.payments.map((payment, i) => (
              <div key={i} className={`flex items-center justify-between text-sm ${payment.cancelled ? 'opacity-50' : ''}`}>
                <div className="flex items-center gap-2">
                  <CreditCard className="w-3.5 h-3.5 text-slate-400" />
                  <span className="text-slate-700 capitalize">{payment.method}</span>
                  {payment.cancelled && <span className="px-1.5 py-0.5 text-[10px] bg-red-100 text-red-600 rounded font-medium">{t('orders.cancelled_payment')}</span>}
                </div>
                <span className={`font-medium ${payment.cancelled ? 'text-slate-400 line-through' : 'text-green-600'}`}>{formatCurrency(payment.amount)}</span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Tax breakdown */}
      {detail.desglose.length > 0 && (
        <div className="border-t border-slate-100 pt-3">
          <h3 className="text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2">{t('orders.tax_breakdown')}</h3>
          <div className="space-y-1">
            {detail.desglose.map((row, i) => (
              <div key={i} className="flex justify-between text-sm">
                <span className="text-slate-600">{row.tax_rate}%</span>
                <span className="text-slate-900">{formatCurrency(row.tax_amount)}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Timeline (collapsible) */}
      {timelineEvents.length > 0 && (
        <div className="border-t border-slate-100 pt-3">
          <button
            type="button"
            className="flex items-center justify-between w-full text-xs font-semibold text-slate-400 uppercase tracking-wider mb-2 cursor-pointer"
            onClick={() => setShowTimeline(!showTimeline)}
          >
            <span className="flex items-center gap-1.5">
              <Clock className="w-3.5 h-3.5" />
              {t('timeline.title')} ({timelineEvents.length})
            </span>
            {showTimeline ? <ChevronUp className="w-3.5 h-3.5" /> : <ChevronDown className="w-3.5 h-3.5" />}
          </button>
          {showTimeline && (
            <TimelineCard events={timelineEvents} t={t} />
          )}
        </div>
      )}
    </div>
  );
};

/* ═══════════════════════════════════════════════════════════════════════
   Shared Sub-Components
   ═══════════════════════════════════════════════════════════════════════ */

const OrderHeader: React.FC<{
  detail: OrderDetailResponse;
  orderKey: string;
  isVoid: boolean;
  t: (key: string) => string;
}> = ({ detail, orderKey, isVoid, t }) => {
  const d = detail.detail;
  return (
    <div className="bg-white rounded-2xl p-5 shadow-sm border border-slate-200 flex justify-between items-start">
      <div>
        <div className="flex items-center gap-3 mb-2">
          <h1 className={`text-2xl font-bold ${isVoid ? 'text-slate-400 line-through' : 'text-slate-900'}`}>
            {orderKey}
          </h1>
          {isVoid && (
            <span className="px-2 py-1 bg-red-100 text-red-700 text-xs font-bold rounded uppercase">
              {t('orders.voided')}
            </span>
          )}
        </div>
        <div className="flex flex-wrap gap-4 text-sm text-slate-500">
          {d.operator_name && <span>{t('orders.operator')}: {d.operator_name}</span>}
          <span className="flex items-center gap-1.5">
            <Calendar className="w-4 h-4" />
            {new Date(d.start_time).toLocaleDateString()}
          </span>
          <span className="flex items-center gap-1.5">
            <Clock className="w-4 h-4" />
            {new Date(d.start_time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}
          </span>
          {d.zone_name && <span>{d.zone_name}{d.table_name ? ` · ${d.table_name}` : ''}</span>}
          {d.guest_count != null && d.guest_count > 0 && <span>{d.guest_count} {t('orders.guests')}</span>}
        </div>
        <div className="flex items-center gap-2 mt-2 text-xs text-slate-400">
          {detail.source === 'cache'
            ? <><Cloud className="w-3.5 h-3.5" /><span>{t('orders.source_cache')}</span></>
            : <><Wifi className="w-3.5 h-3.5" /><span>{t('orders.source_edge')}</span></>
          }
        </div>

        {isVoid && (
          <div className="mt-4 pt-3 border-t border-red-100 flex flex-wrap gap-6 text-sm">
            <div>
              <p className="text-xs text-red-400 font-medium uppercase">{t('orders.voided')}</p>
              <p className="text-red-700 font-medium">{d.void_type}</p>
            </div>
            {d.loss_reason && (
              <div>
                <p className="text-xs text-slate-400 font-medium uppercase">{t('orders.void_reason')}</p>
                <p className="text-slate-700 font-medium">{d.loss_reason}</p>
              </div>
            )}
            {d.loss_amount != null && d.loss_amount > 0 && (
              <div>
                <p className="text-xs text-orange-400 font-medium uppercase">{t('orders.loss_amount')}</p>
                <p className="text-orange-600 font-bold">{formatCurrency(d.loss_amount)}</p>
              </div>
            )}
          </div>
        )}
      </div>
      <div className="text-right shrink-0 pl-6">
        <p className="text-sm text-slate-400 uppercase font-bold tracking-wider mb-1">{t('orders.total')}</p>
        <p className={`text-3xl font-bold ${isVoid ? 'text-slate-400 line-through' : 'text-primary-500'}`}>
          {formatCurrency(d.paid_amount)}
        </p>
      </div>
    </div>
  );
};

const ItemsCard: React.FC<{
  items: OrderItem[];
  categoryColorMap: Map<string, number>;
  detail: OrderDetailResponse;
  t: (key: string) => string;
}> = ({ items, categoryColorMap, detail, t }) => {
  const d = detail.detail;
  return (
    <div className="bg-white rounded-2xl shadow-sm border border-slate-200 overflow-hidden">
      <div className="p-4 border-b border-slate-100 bg-slate-50 flex items-center gap-2 font-bold text-slate-700">
        <Receipt className="w-[18px] h-[18px]" />
        <span>{t('orders.items')}</span>
      </div>
      <div className="divide-y divide-slate-100">
        {items.map((item, i) => (
          <ItemRow key={i} item={item} accentColor={ACCENT_COLORS[categoryColorMap.get(item.category_name ?? '__none__') ?? 0]} t={t} />
        ))}
      </div>
      <div className="p-4 bg-slate-50 border-t border-slate-200 space-y-2">
        {d.comp_total_amount > 0 && <SummaryRow label={t('orders.comped')} value={`-${formatCurrency(d.comp_total_amount)}`} color="text-emerald-600" />}
        {d.discount_amount > 0 && <SummaryRow label={t('orders.discount')} value={`-${formatCurrency(d.discount_amount)}`} color="text-orange-500" />}
        {d.surcharge_amount > 0 && <SummaryRow label={t('orders.surcharge')} value={`+${formatCurrency(d.surcharge_amount)}`} color="text-purple-500" />}
        <div className="flex justify-between items-end pt-3 mt-1 border-t border-slate-200">
          <span className="text-slate-800 font-bold">{t('orders.total')}</span>
          <span className="text-xl font-bold text-primary-500">{formatCurrency(d.paid_amount)}</span>
        </div>
      </div>
    </div>
  );
};

const PaymentsCard: React.FC<{ payments: OrderPayment[]; t: (key: string) => string }> = ({ payments, t }) => (
  <div className="bg-white rounded-2xl shadow-sm border border-slate-200 overflow-hidden">
    <div className="p-4 border-b border-slate-100 bg-slate-50 flex items-center gap-2 font-bold text-slate-700">
      <CreditCard className="w-[18px] h-[18px]" />
      <span>{t('orders.payments')}</span>
    </div>
    <div className="divide-y divide-slate-100">
      {payments.length === 0 ? (
        <div className="p-4 text-center text-slate-400 text-sm">{t('orders.empty')}</div>
      ) : (
        payments.map((payment, i) => <PaymentRow key={i} payment={payment} t={t} />)
      )}
    </div>
  </div>
);

const TaxCard: React.FC<{ desglose: OrderDetailResponse['desglose']; t: (key: string) => string }> = ({ desglose, t }) => (
  <div className="bg-white rounded-2xl shadow-sm border border-slate-200 overflow-hidden">
    <div className="p-4 border-b border-slate-100 bg-slate-50 font-bold text-slate-700">
      {t('orders.tax_breakdown')}
    </div>
    <div className="divide-y divide-slate-100">
      {desglose.map((row, i) => (
        <div key={i} className="px-4 py-3 flex justify-between items-center text-sm">
          <span className="text-slate-700 font-medium">{row.tax_rate}%</span>
          <div className="flex flex-col gap-2 sm:flex-row sm:gap-6">
            <div className="text-right">
              <p className="text-[10px] text-slate-400 uppercase">{t('orders.tax_base')}</p>
              <p className="text-slate-600">{formatCurrency(row.base_amount)}</p>
            </div>
            <div className="text-right">
              <p className="text-[10px] text-slate-400 uppercase">{t('orders.tax_amount')}</p>
              <p className="font-bold text-slate-900">{formatCurrency(row.tax_amount)}</p>
            </div>
          </div>
        </div>
      ))}
    </div>
  </div>
);

/* ── Item Row ── */

const ItemRow: React.FC<{ item: OrderItem; accentColor: string; t: (k: string) => string }> = ({ item, accentColor, t }) => {
  const [expanded, setExpanded] = useState(false);
  const hasOptions = item.options.length > 0;

  return (
    <div>
      <div
        className={`px-4 py-3 flex justify-between items-center transition-colors select-none ${hasOptions ? 'cursor-pointer hover:bg-slate-50/50' : ''}`}
        onClick={() => hasOptions && setExpanded(!expanded)}
      >
        <div className="flex items-center gap-3 flex-1 min-w-0">
          <span className="w-2 h-2 rounded-full shrink-0" style={{ backgroundColor: accentColor }} />
          <div className={`w-8 h-8 rounded flex items-center justify-center font-bold text-sm shrink-0 ${
            item.is_comped ? 'bg-emerald-100 text-emerald-600' : 'bg-slate-100 text-slate-500'
          }`}>
            x{item.quantity}
          </div>
          <div className="flex-1 min-w-0">
            <div className="font-medium text-slate-800 flex items-center gap-2 flex-wrap">
              <span className="shrink-0">{item.name}</span>
              {item.spec_name && <span className="text-xs text-slate-500">({item.spec_name})</span>}
              {item.is_comped && (
                <span className="text-[0.625rem] font-bold bg-emerald-100 text-emerald-700 px-1.5 py-0.5 rounded flex items-center gap-0.5">
                  <Gift size={10} /> {t('orders.comped')}
                </span>
              )}
              {item.discount_amount > 0 && (
                <span className="text-[0.625rem] font-bold bg-orange-100 text-orange-700 px-1.5 py-0.5 rounded-full">
                  -{formatCurrency(item.discount_amount)}
                </span>
              )}
              {item.surcharge_amount > 0 && (
                <span className="text-[0.625rem] font-bold bg-purple-100 text-purple-700 px-1.5 py-0.5 rounded-full">
                  +{formatCurrency(item.surcharge_amount)}
                </span>
              )}
            </div>
            <div className="text-xs text-slate-400 flex items-center gap-2">
              <span>{formatCurrency(item.unit_price)}</span>
              <span>/ {t('orders.subtotal')}</span>
              {hasOptions && (
                <span className="flex items-center gap-1 ml-1 text-slate-400 bg-slate-100 px-1.5 py-0.5 rounded-md">
                  {expanded ? <ChevronUp size={10} /> : <ChevronDown size={10} />}
                </span>
              )}
            </div>
          </div>
        </div>
        <div className="font-bold text-slate-800 pl-4 shrink-0">{formatCurrency(item.line_total)}</div>
      </div>
      {expanded && hasOptions && (
        <div className="px-4 sm:px-16 pb-4 pt-0">
          <div className="p-3 bg-white rounded-lg border border-slate-100 space-y-1 shadow-sm">
            {item.options.map((opt, j) => (
              <div key={j} className="text-sm">
                <span className="text-slate-500 font-medium">{opt.attribute_name}: </span>
                <span className="text-slate-800">
                  {opt.option_name}
                  {opt.price > 0 && <span className="text-xs font-bold text-orange-600 ml-0.5">+{formatCurrency(opt.price)}</span>}
                </span>
              </div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};

/* ── Payment Row ── */

const PaymentRow: React.FC<{ payment: OrderPayment; t: (k: string) => string }> = ({ payment, t }) => {
  const isCash = /cash|efectivo|现金/i.test(payment.method);
  const iconBg = isCash ? 'bg-green-100 text-green-600' : 'bg-indigo-100 text-indigo-600';
  const Icon = isCash ? Coins : CreditCard;

  return (
    <div className="px-4 py-3 flex justify-between items-center">
      <div className="flex items-center gap-3">
        <div className={`p-2 rounded-full ${iconBg}`}>
          <Icon size={16} />
        </div>
        <div>
          <div className="font-medium text-slate-800 flex items-center gap-2 flex-wrap">
            <span className="capitalize">{payment.method}</span>
            {payment.cancelled && (
              <span className="text-xs bg-red-100 text-red-600 px-1.5 py-0.5 rounded font-bold flex items-center gap-1">
                <Ban size={10} /> {t('orders.cancelled_payment')}
              </span>
            )}
          </div>
          <div className="text-xs text-slate-400">
            {new Date(payment.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}
          </div>
        </div>
      </div>
      <div className={`font-bold ${payment.cancelled ? 'text-slate-400 line-through' : 'text-slate-800'}`}>
        {formatCurrency(payment.amount)}
      </div>
    </div>
  );
};

/* ── Helpers ── */

const SummaryRow: React.FC<{ label: string; value: string; color: string }> = ({ label, value, color }) => (
  <div className="flex justify-between text-sm">
    <span className={color}>{label}</span>
    <span className={color}>{value}</span>
  </div>
);
