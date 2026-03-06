import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ShieldAlert, Package, ClipboardList, CreditCard, ChevronDown, ChevronUp } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getStoreRedFlags, getStoreRedFlagLog } from '@/infrastructure/api/stats';
import { ApiError } from '@/infrastructure/api/client';
import { Spinner } from '@/presentation/components/ui/Spinner';
import type { RedFlagsResponse, RedFlagLogResponse } from '@/core/types/stats';

function getRange(dateStr: string): { from: number; to: number } {
  const d = new Date(dateStr + 'T00:00:00');
  const next = new Date(d);
  next.setDate(next.getDate() + 1);
  return { from: d.getTime(), to: next.getTime() };
}

const EVENT_TYPES = [
  'ITEM_REMOVED', 'ITEM_COMPED', 'ITEM_UNCOMPED', 'ITEM_MODIFIED',
  'ORDER_VOIDED', 'ORDER_DISCOUNT_APPLIED', 'ORDER_SURCHARGE_APPLIED', 'RULE_SKIP_TOGGLED',
  'PAYMENT_CANCELLED', 'REFUND',
] as const;

const EVENT_COLORS: Record<string, string> = {
  ITEM_REMOVED: 'bg-red-100 text-red-700',
  ITEM_COMPED: 'bg-emerald-100 text-emerald-700',
  ITEM_UNCOMPED: 'bg-teal-100 text-teal-700',
  ITEM_MODIFIED: 'bg-orange-100 text-orange-700',
  ORDER_VOIDED: 'bg-red-100 text-red-700',
  ORDER_DISCOUNT_APPLIED: 'bg-amber-100 text-amber-700',
  ORDER_SURCHARGE_APPLIED: 'bg-purple-100 text-purple-700',
  RULE_SKIP_TOGGLED: 'bg-sky-100 text-sky-700',
  PAYMENT_CANCELLED: 'bg-rose-100 text-rose-700',
  REFUND: 'bg-violet-100 text-violet-700',
};

export const RedFlagsScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [data, setData] = useState<RedFlagsResponse | null>(null);
  const [log, setLog] = useState<RedFlagLogResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [dateInput, setDateInput] = useState(new Date().toISOString().slice(0, 10));
  const [expandedOps, setExpandedOps] = useState(false);
  const [eventFilter, setEventFilter] = useState('');
  const [operatorFilter, setOperatorFilter] = useState<number | ''>('');
  const [logPage, setLogPage] = useState(1);

  const et = (key: string) => t(`red_flags.events.${key}`);
  const formatTime = (ts: number) => new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

  const loadData = useCallback(async (dateStr: string) => {
    const tk = useAuthStore.getState().token;
    if (!tk) return;
    setLoading(true);
    setError('');
    try {
      const { from, to } = getRange(dateStr);
      const [summary, logData] = await Promise.all([
        getStoreRedFlags(tk, storeId, from, to),
        getStoreRedFlagLog(tk, storeId, from, to),
      ]);
      setData(summary);
      setLog(logData);
      setLogPage(1);
      setEventFilter('');
      setOperatorFilter('');
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [storeId]);

  useEffect(() => { loadData(dateInput); }, [loadData, dateInput]);

  // Reload log when filters change
  useEffect(() => {
    const tk = useAuthStore.getState().token;
    if (!tk || !data) return;
    const { from, to } = getRange(dateInput);
    getStoreRedFlagLog(
      tk, storeId, from, to,
      eventFilter || undefined,
      operatorFilter !== '' ? operatorFilter : undefined,
      1,
    ).then(d => { setLog(d); setLogPage(1); }).catch(() => {});
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [eventFilter, operatorFilter]);

  const handleLoadMore = () => {
    const tk = useAuthStore.getState().token;
    if (!tk) return;
    const nextPage = logPage + 1;
    const { from, to } = getRange(dateInput);
    getStoreRedFlagLog(
      tk, storeId, from, to,
      eventFilter || undefined,
      operatorFilter !== '' ? operatorFilter : undefined,
      nextPage,
    ).then(d => {
      setLog(prev => prev ? { ...d, entries: [...prev.entries, ...d.entries] } : d);
      setLogPage(nextPage);
    }).catch(() => {});
  };

  const operators = data?.operator_breakdown ?? [];

  return (
    <div className="max-w-6xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
      <div className="flex flex-col md:flex-row md:items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-red-100 rounded-xl flex items-center justify-center shrink-0">
            <ShieldAlert className="w-5 h-5 text-red-600" />
          </div>
          <h1 className="text-xl font-bold text-slate-900">{t('red_flags.title')}</h1>
        </div>
        <input
          type="date"
          value={dateInput}
          onChange={e => setDateInput(e.target.value)}
          className="rounded-lg border border-slate-200 text-sm px-3 py-2 focus:border-primary-500 focus:ring-primary-500 focus:outline-none"
        />
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>
      ) : error ? (
        <div className="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div>
      ) : data ? (
        <>
          {/* Summary Cards */}
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="bg-white rounded-xl border border-orange-200 p-4">
              <h3 className="text-sm font-semibold text-orange-700 flex items-center gap-2 mb-3">
                <Package className="w-4 h-4" />{t('red_flags.group_items')}
              </h3>
              <div className="space-y-1.5 text-sm">
                {data.item_flags.removals > 0 && <FlagRow label={et('ITEM_REMOVED')} count={data.item_flags.removals} />}
                {data.item_flags.comps > 0 && <FlagRow label={et('ITEM_COMPED')} count={data.item_flags.comps} />}
                {data.item_flags.uncomps > 0 && <FlagRow label={et('ITEM_UNCOMPED')} count={data.item_flags.uncomps} />}
                {data.item_flags.price_modifications > 0 && <FlagRow label={et('ITEM_MODIFIED')} count={data.item_flags.price_modifications} />}
                {(data.item_flags.removals + data.item_flags.comps + data.item_flags.uncomps + data.item_flags.price_modifications) === 0 && (
                  <p className="text-slate-400 text-xs">{t('red_flags.no_data')}</p>
                )}
              </div>
            </div>

            <div className="bg-white rounded-xl border border-red-200 p-4">
              <h3 className="text-sm font-semibold text-red-700 flex items-center gap-2 mb-3">
                <ClipboardList className="w-4 h-4" />{t('red_flags.group_orders')}
              </h3>
              <div className="space-y-1.5 text-sm">
                {data.order_flags.voids > 0 && <FlagRow label={et('ORDER_VOIDED')} count={data.order_flags.voids} />}
                {data.order_flags.discounts > 0 && <FlagRow label={et('ORDER_DISCOUNT_APPLIED')} count={data.order_flags.discounts} />}
                {data.order_flags.surcharges > 0 && <FlagRow label={et('ORDER_SURCHARGE_APPLIED')} count={data.order_flags.surcharges} />}
                {data.order_flags.rule_skips > 0 && <FlagRow label={et('RULE_SKIP_TOGGLED')} count={data.order_flags.rule_skips} />}
                {(data.order_flags.voids + data.order_flags.discounts + data.order_flags.surcharges + data.order_flags.rule_skips) === 0 && (
                  <p className="text-slate-400 text-xs">{t('red_flags.no_data')}</p>
                )}
              </div>
            </div>

            <div className="bg-white rounded-xl border border-purple-200 p-4">
              <h3 className="text-sm font-semibold text-purple-700 flex items-center gap-2 mb-3">
                <CreditCard className="w-4 h-4" />{t('red_flags.group_payments')}
              </h3>
              <div className="space-y-1.5 text-sm">
                {data.payment_flags.cancellations > 0 && <FlagRow label={et('PAYMENT_CANCELLED')} count={data.payment_flags.cancellations} />}
                {data.payment_flags.refund_count > 0 && (
                  <div className="flex justify-between">
                    <span className="text-slate-600">{et('REFUND')}</span>
                    <span className="font-semibold tabular-nums">
                      {data.payment_flags.refund_count}
                      <span className="text-xs text-slate-400 ml-1">({data.payment_flags.refund_amount.toFixed(2)})</span>
                    </span>
                  </div>
                )}
                {(data.payment_flags.cancellations + data.payment_flags.refund_count) === 0 && (
                  <p className="text-slate-400 text-xs">{t('red_flags.no_data')}</p>
                )}
              </div>
            </div>
          </div>

          {/* Operator Breakdown */}
          {operators.length > 0 && (
            <div className="bg-white rounded-2xl border border-slate-200 overflow-hidden">
              <button
                onClick={() => setExpandedOps(!expandedOps)}
                className="w-full px-4 md:px-6 py-4 flex items-center justify-between text-sm font-semibold text-slate-700 hover:bg-slate-50"
              >
                {t('red_flags.operator_breakdown')} ({operators.length})
                {expandedOps ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
              </button>
              {expandedOps && (
                <>
                  {/* Desktop table */}
                  <div className="hidden md:block overflow-x-auto border-t border-slate-100">
                    <table className="w-full text-xs">
                      <thead>
                        <tr className="bg-slate-50 text-slate-500">
                          <th className="px-3 py-2 text-left font-medium">{t('red_flags.operator')}</th>
                          <th className="px-2 py-2 text-center font-medium">{et('ITEM_REMOVED')}</th>
                          <th className="px-2 py-2 text-center font-medium">{et('ITEM_COMPED')}</th>
                          <th className="px-2 py-2 text-center font-medium">{et('ITEM_UNCOMPED')}</th>
                          <th className="px-2 py-2 text-center font-medium">{et('ITEM_MODIFIED')}</th>
                          <th className="px-2 py-2 text-center font-medium">{et('ORDER_VOIDED')}</th>
                          <th className="px-2 py-2 text-center font-medium">{et('ORDER_DISCOUNT_APPLIED')}</th>
                          <th className="px-2 py-2 text-center font-medium">{et('ORDER_SURCHARGE_APPLIED')}</th>
                          <th className="px-2 py-2 text-center font-medium">{et('RULE_SKIP_TOGGLED')}</th>
                          <th className="px-2 py-2 text-center font-medium">{et('PAYMENT_CANCELLED')}</th>
                          <th className="px-2 py-2 text-center font-medium">{et('REFUND')}</th>
                          <th className="px-2 py-2 text-center font-medium">{t('red_flags.total')}</th>
                        </tr>
                      </thead>
                      <tbody>
                        {operators.map(op => (
                          <tr key={op.operator_id} className="border-t border-slate-50 hover:bg-slate-50">
                            <td className="px-3 py-2 font-medium text-slate-800">{op.operator_name || `#${op.operator_id}`}</td>
                            <td className="px-2 py-2 text-center tabular-nums">{op.removals || '-'}</td>
                            <td className="px-2 py-2 text-center tabular-nums">{op.comps || '-'}</td>
                            <td className="px-2 py-2 text-center tabular-nums">{op.uncomps || '-'}</td>
                            <td className="px-2 py-2 text-center tabular-nums">{op.price_modifications || '-'}</td>
                            <td className="px-2 py-2 text-center tabular-nums">{op.voids || '-'}</td>
                            <td className="px-2 py-2 text-center tabular-nums">{op.discounts || '-'}</td>
                            <td className="px-2 py-2 text-center tabular-nums">{op.surcharges || '-'}</td>
                            <td className="px-2 py-2 text-center tabular-nums">{op.rule_skips || '-'}</td>
                            <td className="px-2 py-2 text-center tabular-nums">{op.cancellations || '-'}</td>
                            <td className="px-2 py-2 text-center tabular-nums">{op.refund_count || '-'}</td>
                            <td className="px-2 py-2 text-center font-bold tabular-nums">{op.total_flags}</td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                  {/* Mobile cards */}
                  <div className="md:hidden divide-y divide-slate-100 border-t border-slate-100">
                    {operators.map(op => (
                      <div key={op.operator_id} className="px-4 py-3">
                        <div className="flex items-center justify-between mb-2">
                          <span className="font-medium text-slate-900">{op.operator_name || `#${op.operator_id}`}</span>
                          <span className="text-lg font-bold tabular-nums text-slate-900">{op.total_flags}</span>
                        </div>
                        <div className="flex flex-wrap gap-1.5">
                          {op.removals > 0 && <MobileBadge label={et('ITEM_REMOVED')} count={op.removals} color="bg-red-100 text-red-700" />}
                          {op.comps > 0 && <MobileBadge label={et('ITEM_COMPED')} count={op.comps} color="bg-emerald-100 text-emerald-700" />}
                          {op.uncomps > 0 && <MobileBadge label={et('ITEM_UNCOMPED')} count={op.uncomps} color="bg-teal-100 text-teal-700" />}
                          {op.price_modifications > 0 && <MobileBadge label={et('ITEM_MODIFIED')} count={op.price_modifications} color="bg-orange-100 text-orange-700" />}
                          {op.voids > 0 && <MobileBadge label={et('ORDER_VOIDED')} count={op.voids} color="bg-red-100 text-red-700" />}
                          {op.discounts > 0 && <MobileBadge label={et('ORDER_DISCOUNT_APPLIED')} count={op.discounts} color="bg-amber-100 text-amber-700" />}
                          {op.surcharges > 0 && <MobileBadge label={et('ORDER_SURCHARGE_APPLIED')} count={op.surcharges} color="bg-purple-100 text-purple-700" />}
                          {op.rule_skips > 0 && <MobileBadge label={et('RULE_SKIP_TOGGLED')} count={op.rule_skips} color="bg-sky-100 text-sky-700" />}
                          {op.cancellations > 0 && <MobileBadge label={et('PAYMENT_CANCELLED')} count={op.cancellations} color="bg-rose-100 text-rose-700" />}
                          {op.refund_count > 0 && <MobileBadge label={et('REFUND')} count={op.refund_count} color="bg-violet-100 text-violet-700" />}
                        </div>
                      </div>
                    ))}
                  </div>
                </>
              )}
            </div>
          )}

          {/* Event Log */}
          <div className="bg-white rounded-2xl border border-slate-200 overflow-hidden">
            <div className="px-4 md:px-6 py-4 border-b border-slate-100 flex flex-col md:flex-row md:items-center justify-between gap-3">
              <h2 className="font-semibold text-slate-900">{t('red_flags.event_log')}</h2>
              <div className="flex gap-2">
                <select
                  value={eventFilter}
                  onChange={e => setEventFilter(e.target.value)}
                  className="text-xs border border-slate-200 rounded-lg px-2 py-1.5"
                >
                  <option value="">{t('red_flags.all_types')}</option>
                  {EVENT_TYPES.map(type => (
                    <option key={type} value={type}>{et(type)}</option>
                  ))}
                </select>
                {operators.length > 0 && (
                  <select
                    value={operatorFilter}
                    onChange={e => setOperatorFilter(e.target.value ? Number(e.target.value) : '')}
                    className="text-xs border border-slate-200 rounded-lg px-2 py-1.5"
                  >
                    <option value="">{t('red_flags.all_operators')}</option>
                    {operators.map(op => (
                      <option key={op.operator_id} value={op.operator_id}>{op.operator_name}</option>
                    ))}
                  </select>
                )}
              </div>
            </div>

            {log && log.entries.length > 0 ? (
              <div className="divide-y divide-slate-50">
                {log.entries.map((entry, i) => (
                  <div key={i} className="px-4 md:px-6 py-2.5 hover:bg-slate-50">
                    {/* Desktop: single row */}
                    <div className="hidden md:flex items-center gap-3 text-sm">
                      <span className="text-xs text-slate-400 tabular-nums w-12 shrink-0">{formatTime(entry.timestamp)}</span>
                      <span className="text-slate-600 w-24 shrink-0 truncate">{entry.operator_name}</span>
                      <span className={`text-xs px-2 py-0.5 rounded-full font-medium shrink-0 ${EVENT_COLORS[entry.event_type] ?? 'bg-slate-100 text-slate-600'}`}>
                        {et(entry.event_type)}
                      </span>
                      <span className="text-slate-800 font-mono text-xs truncate">{entry.receipt_number}</span>
                      {entry.detail && <span className="text-slate-400 text-xs truncate ml-auto">{entry.detail}</span>}
                    </div>
                    {/* Mobile: stacked */}
                    <div className="md:hidden text-sm space-y-1">
                      <div className="flex items-center justify-between">
                        <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${EVENT_COLORS[entry.event_type] ?? 'bg-slate-100 text-slate-600'}`}>
                          {et(entry.event_type)}
                        </span>
                        <span className="text-xs text-slate-400 tabular-nums">{formatTime(entry.timestamp)}</span>
                      </div>
                      <div className="flex items-center justify-between">
                        <span className="text-slate-600 text-xs">{entry.operator_name}</span>
                        <span className="text-slate-800 font-mono text-xs">{entry.receipt_number}</span>
                      </div>
                      {entry.detail && <div className="text-slate-400 text-xs truncate">{entry.detail}</div>}
                    </div>
                  </div>
                ))}
                {log.entries.length < log.total && (
                  <button
                    onClick={handleLoadMore}
                    className="w-full py-3 text-sm text-primary-600 font-medium hover:bg-primary-50 transition-colors"
                  >
                    {t('red_flags.load_more')} ({log.entries.length}/{log.total})
                  </button>
                )}
              </div>
            ) : (
              <div className="px-4 py-8 text-center text-slate-400 text-sm">
                {t('red_flags.no_data')}
              </div>
            )}
          </div>

          <div className="text-xs text-slate-400 text-center py-2">
            {t('red_flags.compliance')}
          </div>
        </>
      ) : null}
    </div>
  );
};

const FlagRow: React.FC<{ label: string; count: number }> = ({ label, count }) => (
  <div className="flex justify-between">
    <span className="text-slate-600">{label}</span>
    <span className="font-semibold tabular-nums">{count}</span>
  </div>
);

const MobileBadge: React.FC<{ label: string; count: number; color: string }> = ({ label, count, color }) => (
  <span className={`px-2 py-0.5 text-xs rounded-full tabular-nums ${color}`}>{label} {count}</span>
);
