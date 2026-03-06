import React, { useState, useEffect, useCallback } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { logger } from '@/utils/logger';
import { toast } from '@/presentation/components/Toast';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { TimeRangeSelector, useTimeRange } from './TimeRangeSelector';
import { Package, ClipboardList, CreditCard, ChevronDown, ChevronUp } from 'lucide-react';

// ── Types ──

interface ItemFlags {
  removals: number;
  comps: number;
  uncomps: number;
  price_modifications: number;
}
interface OrderFlags {
  voids: number;
  discounts: number;
  surcharges: number;
  rule_skips: number;
}
interface PaymentFlags {
  cancellations: number;
  refund_count: number;
  refund_amount: number;
}
interface OperatorRedFlags {
  operator_id: number;
  operator_name: string;
  removals: number;
  comps: number;
  uncomps: number;
  price_modifications: number;
  voids: number;
  discounts: number;
  surcharges: number;
  rule_skips: number;
  cancellations: number;
  refund_count: number;
  refund_amount: number;
  total_flags: number;
}
interface RedFlagsResponse {
  item_flags: ItemFlags;
  order_flags: OrderFlags;
  payment_flags: PaymentFlags;
  operator_breakdown: OperatorRedFlags[];
}
interface LogEntry {
  timestamp: number;
  event_type: string;
  operator_id: number;
  operator_name: string;
  receipt_number: string;
  order_id: number;
  detail: string | null;
}
interface LogResponse {
  entries: LogEntry[];
  total: number;
  page: number;
  per_page: number;
}

// ── Event type labels + colors ──

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

// ── Main component ──

export const RedFlagsTab: React.FC = () => {
  const { t } = useI18n();
  const [range, setRange] = useTimeRange();
  const [summary, setSummary] = useState<RedFlagsResponse | null>(null);
  const [log, setLog] = useState<LogResponse | null>(null);
  const [logPage, setLogPage] = useState(1);
  const [eventFilter, setEventFilter] = useState('');
  const [operatorFilter, setOperatorFilter] = useState<number | ''>('');
  const [expandedOps, setExpandedOps] = useState(false);

  const loadSummary = useCallback(async () => {
    try {
      const data = await invokeApi<RedFlagsResponse>('get_red_flags', { from: range.from, to: range.to });
      setSummary(data);
    } catch (e) {
      logger.error('Failed to load red flags summary', e);
      toast.error(t('statistics.error.load'));
    }
  }, [range, t]);

  const loadLog = useCallback(async (page: number) => {
    try {
      const params: Record<string, unknown> = { from: range.from, to: range.to, page };
      if (eventFilter) params.eventType = eventFilter;
      if (operatorFilter !== '') params.operatorId = operatorFilter;
      const data = await invokeApi<LogResponse>('get_red_flag_log', params);
      setLog(data);
    } catch (e) {
      logger.error('Failed to load red flags log', e);
    }
  }, [range, eventFilter, operatorFilter]);

  useEffect(() => { loadSummary(); }, [loadSummary]);
  useEffect(() => { setLogPage(1); loadLog(1); }, [loadLog]);

  const handleLoadMore = () => {
    const nextPage = logPage + 1;
    setLogPage(nextPage);
    invokeApi<LogResponse>('get_red_flag_log', {
      from: range.from, to: range.to, page: nextPage,
      ...(eventFilter ? { eventType: eventFilter } : {}),
      ...(operatorFilter !== '' ? { operatorId: operatorFilter } : {}),
    }).then(data => {
      setLog(prev => prev ? { ...data, entries: [...prev.entries, ...data.entries] } : data);
    }).catch(() => {});
  };

  const et = (key: string) => t(`statistics.red_flags.events.${key}`);
  const formatTime = (ts: number) => new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

  const operators = summary?.operator_breakdown ?? [];

  return (
    <>
      <TimeRangeSelector value={range} onChange={setRange} />

      {/* ── Summary Cards ── */}
      {summary && (
        <div className="grid grid-cols-3 gap-4 mt-4">
          {/* Item Flags */}
          <div className="bg-white rounded-xl border border-orange-200 p-4">
            <h3 className="text-sm font-semibold text-orange-700 flex items-center gap-2 mb-3">
              <Package className="w-4 h-4" />{t('statistics.red_flags.group_items')}
            </h3>
            <div className="space-y-1.5 text-sm">
              {summary.item_flags.removals > 0 && <FlagRow label={et('ITEM_REMOVED')} count={summary.item_flags.removals} />}
              {summary.item_flags.comps > 0 && <FlagRow label={et('ITEM_COMPED')} count={summary.item_flags.comps} />}
              {summary.item_flags.uncomps > 0 && <FlagRow label={et('ITEM_UNCOMPED')} count={summary.item_flags.uncomps} />}
              {summary.item_flags.price_modifications > 0 && <FlagRow label={et('ITEM_MODIFIED')} count={summary.item_flags.price_modifications} />}
              {(summary.item_flags.removals + summary.item_flags.comps + summary.item_flags.uncomps + summary.item_flags.price_modifications) === 0 && (
                <p className="text-slate-400 text-xs">{t('statistics.red_flags.no_flags')}</p>
              )}
            </div>
          </div>

          {/* Order Flags */}
          <div className="bg-white rounded-xl border border-red-200 p-4">
            <h3 className="text-sm font-semibold text-red-700 flex items-center gap-2 mb-3">
              <ClipboardList className="w-4 h-4" />{t('statistics.red_flags.group_orders')}
            </h3>
            <div className="space-y-1.5 text-sm">
              {summary.order_flags.voids > 0 && <FlagRow label={et('ORDER_VOIDED')} count={summary.order_flags.voids} />}
              {summary.order_flags.discounts > 0 && <FlagRow label={et('ORDER_DISCOUNT_APPLIED')} count={summary.order_flags.discounts} />}
              {summary.order_flags.surcharges > 0 && <FlagRow label={et('ORDER_SURCHARGE_APPLIED')} count={summary.order_flags.surcharges} />}
              {summary.order_flags.rule_skips > 0 && <FlagRow label={et('RULE_SKIP_TOGGLED')} count={summary.order_flags.rule_skips} />}
              {(summary.order_flags.voids + summary.order_flags.discounts + summary.order_flags.surcharges + summary.order_flags.rule_skips) === 0 && (
                <p className="text-slate-400 text-xs">{t('statistics.red_flags.no_flags')}</p>
              )}
            </div>
          </div>

          {/* Payment Flags */}
          <div className="bg-white rounded-xl border border-purple-200 p-4">
            <h3 className="text-sm font-semibold text-purple-700 flex items-center gap-2 mb-3">
              <CreditCard className="w-4 h-4" />{t('statistics.red_flags.group_payments')}
            </h3>
            <div className="space-y-1.5 text-sm">
              {summary.payment_flags.cancellations > 0 && <FlagRow label={et('PAYMENT_CANCELLED')} count={summary.payment_flags.cancellations} />}
              {summary.payment_flags.refund_count > 0 && (
                <div className="flex justify-between">
                  <span className="text-slate-600">{et('REFUND')}</span>
                  <span className="font-semibold tabular-nums">
                    {summary.payment_flags.refund_count}
                    <span className="text-xs text-slate-400 ml-1">({formatCurrency(summary.payment_flags.refund_amount)})</span>
                  </span>
                </div>
              )}
              {(summary.payment_flags.cancellations + summary.payment_flags.refund_count) === 0 && (
                <p className="text-slate-400 text-xs">{t('statistics.red_flags.no_flags')}</p>
              )}
            </div>
          </div>
        </div>
      )}

      {/* ── Operator Breakdown (collapsible) ── */}
      {operators.length > 0 && (
        <div className="mt-4 bg-white rounded-xl border border-slate-200 overflow-hidden">
          <button
            onClick={() => setExpandedOps(!expandedOps)}
            className="w-full px-4 py-3 flex items-center justify-between text-sm font-semibold text-slate-700 hover:bg-slate-50"
          >
            {t('statistics.red_flags.operator_breakdown')} ({operators.length})
            {expandedOps ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
          </button>
          {expandedOps && (
            <div className="overflow-x-auto border-t border-slate-100">
              <table className="w-full text-xs">
                <thead>
                  <tr className="bg-slate-50 text-slate-500">
                    <th className="px-3 py-2 text-left font-medium">{t('statistics.red_flags.operator')}</th>
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
                    <th className="px-2 py-2 text-center font-medium">{t('statistics.red_flags.total')}</th>
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
          )}
        </div>
      )}

      {/* ── Event Log ── */}
      <div className="mt-4 bg-white rounded-xl border border-slate-200 overflow-hidden">
        <div className="px-4 py-3 border-b border-slate-100 flex items-center justify-between">
          <h3 className="text-sm font-semibold text-slate-700">{t('statistics.red_flags.event_log')}</h3>
          <div className="flex gap-2">
            <select
              value={eventFilter}
              onChange={e => setEventFilter(e.target.value)}
              className="text-xs border border-slate-200 rounded-lg px-2 py-1.5"
            >
              <option value="">{t('statistics.red_flags.all_types')}</option>
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
                <option value="">{t('statistics.red_flags.all_operators')}</option>
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
              <div key={i} className="px-4 py-2.5 flex items-center gap-3 text-sm hover:bg-slate-50">
                <span className="text-xs text-slate-400 tabular-nums w-12 shrink-0">{formatTime(entry.timestamp)}</span>
                <span className="text-slate-600 w-20 shrink-0 truncate">{entry.operator_name}</span>
                <span className={`text-xs px-2 py-0.5 rounded-full font-medium shrink-0 ${EVENT_COLORS[entry.event_type] ?? 'bg-slate-100 text-slate-600'}`}>
                  {et(entry.event_type)}
                </span>
                <span className="text-slate-800 font-mono text-xs truncate">{entry.receipt_number}</span>
                {entry.detail && <span className="text-slate-400 text-xs truncate ml-auto">{entry.detail}</span>}
              </div>
            ))}
            {log.entries.length < log.total && (
              <button
                onClick={handleLoadMore}
                className="w-full py-3 text-sm text-primary-600 font-medium hover:bg-primary-50 transition-colors"
              >
                {t('statistics.red_flags.load_more')} ({log.entries.length}/{log.total})
              </button>
            )}
          </div>
        ) : (
          <div className="px-4 py-8 text-center text-slate-400 text-sm">
            {t('statistics.red_flags.no_flags')}
          </div>
        )}
      </div>
    </>
  );
};

const FlagRow: React.FC<{ label: string; count: number }> = ({ label, count }) => (
  <div className="flex justify-between">
    <span className="text-slate-600">{label}</span>
    <span className="font-semibold tabular-nums">{count}</span>
  </div>
);
