import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { ShieldAlert } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getStoreRedFlags } from '@/infrastructure/api/stats';
import { ApiError } from '@/infrastructure/api/client';
import { Spinner } from '@/presentation/components/ui/Spinner';
import type { RedFlagsResponse } from '@/core/types/stats';

function getRange(dateStr: string): { from: number; to: number } {
  const d = new Date(dateStr + 'T00:00:00');
  const next = new Date(d);
  next.setDate(next.getDate() + 1);
  return { from: d.getTime(), to: next.getTime() };
}

export const RedFlagsScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [data, setData] = useState<RedFlagsResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [dateInput, setDateInput] = useState(new Date().toISOString().slice(0, 10));

  const loadData = useCallback(async (dateStr: string) => {
    if (!token) return;
    setLoading(true);
    setError('');
    try {
      const { from, to } = getRange(dateStr);
      setData(await getStoreRedFlags(token, storeId, from, to));
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  }, [token, storeId, clearAuth, navigate, t]);

  useEffect(() => { loadData(dateInput); }, [loadData, dateInput]);

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
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
          {/* KPI Cards */}
          <div className="grid grid-cols-2 lg:grid-cols-5 gap-3">
            <FlagCard value={data.summary.item_removals} label={t('red_flags.item_removals')} borderColor="border-red-200" textColor="text-red-600" />
            <FlagCard value={data.summary.item_comps} label={t('red_flags.item_comps')} borderColor="border-orange-200" textColor="text-orange-600" />
            <FlagCard value={data.summary.order_voids} label={t('red_flags.order_voids')} borderColor="border-red-200" textColor="text-red-600" />
            <FlagCard value={data.summary.order_discounts} label={t('red_flags.order_discounts')} borderColor="border-yellow-200" textColor="text-yellow-600" />
            <FlagCard value={data.summary.price_modifications} label={t('red_flags.price_modifications')} borderColor="border-orange-200" textColor="text-orange-600" />
          </div>

          {/* Operator breakdown */}
          {data.operator_breakdown.length === 0 ? (
            <div className="bg-white rounded-2xl border border-slate-200 p-8 text-center text-slate-400 text-sm">
              {t('red_flags.no_data')}
            </div>
          ) : (
            <OperatorTable t={t} breakdown={data.operator_breakdown} />
          )}

          {/* Compliance */}
          <div className="text-xs text-slate-400 text-center py-2">
            {t('red_flags.compliance')}
          </div>
        </>
      ) : null}
    </div>
  );
};

const FlagCard: React.FC<{ value: number; label: string; borderColor: string; textColor: string }> = ({ value, label, borderColor, textColor }) => (
  <div className={`bg-white rounded-xl border ${borderColor} p-4 text-center`}>
    <div className={`text-2xl font-bold ${textColor}`}>{value}</div>
    <div className="text-xs text-slate-500 mt-1">{label}</div>
  </div>
);

const OperatorTable: React.FC<{
  t: (key: string) => string;
  breakdown: RedFlagsResponse['operator_breakdown'];
}> = ({ t, breakdown }) => {
  const totalFlags = breakdown.reduce((s, o) => s + o.total_flags, 0);
  const avgPerOperator = breakdown.length > 0 ? totalFlags / breakdown.length : 0;

  return (
    <div className="bg-white rounded-2xl border border-slate-200 overflow-hidden">
      <div className="px-4 md:px-6 py-4 border-b border-slate-100">
        <h2 className="font-semibold text-slate-900">{t('red_flags.operator')}</h2>
      </div>

      {/* Desktop table */}
      <div className="hidden md:block overflow-x-auto">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-slate-100 text-left text-slate-500">
              <th className="px-6 py-3 font-medium">{t('red_flags.operator')}</th>
              <th className="px-4 py-3 font-medium text-center">{t('red_flags.item_removals')}</th>
              <th className="px-4 py-3 font-medium text-center">{t('red_flags.item_comps')}</th>
              <th className="px-4 py-3 font-medium text-center">{t('red_flags.order_voids')}</th>
              <th className="px-4 py-3 font-medium text-center">{t('red_flags.order_discounts')}</th>
              <th className="px-4 py-3 font-medium text-center">{t('red_flags.price_modifications')}</th>
              <th className="px-4 py-3 font-medium text-center">{t('red_flags.total')}</th>
            </tr>
          </thead>
          <tbody>
            {breakdown.map((op, i) => {
              const isHigh = op.total_flags > avgPerOperator * 2;
              return (
                <tr key={i} className={`border-b border-slate-50 ${isHigh ? 'bg-red-50' : ''}`}>
                  <td className="px-6 py-3 font-medium text-slate-900">{op.operator_name ?? t('red_flags.unknown_operator')}</td>
                  <td className="px-4 py-3 text-center tabular-nums">{op.item_removals || '-'}</td>
                  <td className="px-4 py-3 text-center tabular-nums">{op.item_comps || '-'}</td>
                  <td className="px-4 py-3 text-center tabular-nums">{op.order_voids || '-'}</td>
                  <td className="px-4 py-3 text-center tabular-nums">{op.order_discounts || '-'}</td>
                  <td className="px-4 py-3 text-center tabular-nums">{op.price_modifications || '-'}</td>
                  <td className={`px-4 py-3 text-center font-bold tabular-nums ${isHigh ? 'text-red-600' : 'text-slate-900'}`}>{op.total_flags}</td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {/* Mobile cards */}
      <div className="md:hidden divide-y divide-slate-100">
        {breakdown.map((op, i) => {
          const isHigh = op.total_flags > avgPerOperator * 2;
          return (
            <div key={i} className={`px-4 py-3 ${isHigh ? 'bg-red-50' : ''}`}>
              <div className="flex items-center justify-between mb-2">
                <span className="font-medium text-slate-900">{op.operator_name ?? t('red_flags.unknown_operator')}</span>
                <span className={`text-lg font-bold tabular-nums ${isHigh ? 'text-red-600' : 'text-slate-900'}`}>{op.total_flags}</span>
              </div>
              <div className="flex flex-wrap gap-1.5">
                {op.item_removals > 0 && <span className="px-2 py-0.5 text-xs rounded-full bg-red-100 text-red-700 tabular-nums">{t('red_flags.item_removals')} {op.item_removals}</span>}
                {op.item_comps > 0 && <span className="px-2 py-0.5 text-xs rounded-full bg-orange-100 text-orange-700 tabular-nums">{t('red_flags.item_comps')} {op.item_comps}</span>}
                {op.order_voids > 0 && <span className="px-2 py-0.5 text-xs rounded-full bg-red-100 text-red-700 tabular-nums">{t('red_flags.order_voids')} {op.order_voids}</span>}
                {op.order_discounts > 0 && <span className="px-2 py-0.5 text-xs rounded-full bg-yellow-100 text-yellow-700 tabular-nums">{t('red_flags.order_discounts')} {op.order_discounts}</span>}
                {op.price_modifications > 0 && <span className="px-2 py-0.5 text-xs rounded-full bg-orange-100 text-orange-700 tabular-nums">{t('red_flags.price_modifications')} {op.price_modifications}</span>}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
};
