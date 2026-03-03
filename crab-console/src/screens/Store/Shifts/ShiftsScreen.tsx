import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Clock } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { listShifts } from '@/infrastructure/api/stats';
import { ApiError } from '@/infrastructure/api/client';
import { formatCurrency, formatDateTime } from '@/utils/format';
import { Spinner } from '@/presentation/components/ui/Spinner';
import type { ShiftEntry } from '@/core/types/stats';

function formatDuration(startMs: number, endMs: number | null): string {
  if (!endMs) return '-';
  const diff = endMs - startMs;
  const hours = Math.floor(diff / 3600000);
  const minutes = Math.floor((diff % 3600000) / 60000);
  return `${hours}h ${minutes}m`;
}

function StatusBadge({ status, abnormal }: { status: string; abnormal: boolean }) {
  if (status === 'OPEN') {
    return <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-700">OPEN</span>;
  }
  if (abnormal) {
    return <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-amber-100 text-amber-700">ABNORMAL</span>;
  }
  return <span className="inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium bg-slate-100 text-slate-600">CLOSED</span>;
}

export const ShiftsScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [shifts, setShifts] = useState<ShiftEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    const tk = useAuthStore.getState().token;
    if (!tk) return;
    (async () => {
      try {
        setShifts(await listShifts(tk, storeId));
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
        setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      } finally {
        setLoading(false);
      }
    })();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [storeId]);

  if (loading) return <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>;
  if (error) return <div className="max-w-5xl mx-auto px-6 py-8"><div className="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div></div>;

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
      <h1 className="text-xl font-bold text-slate-900">{t('shifts.title')}</h1>

      {shifts.length > 0 ? (
        <>
        {/* Desktop table */}
        <div className="hidden md:block bg-white rounded-2xl border border-slate-200 p-6">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-slate-100">
                  <th className="text-left py-2 text-xs font-medium text-slate-400">{t('shifts.status')}</th>
                  <th className="text-left py-2 text-xs font-medium text-slate-400">{t('shifts.operator')}</th>
                  <th className="text-left py-2 text-xs font-medium text-slate-400">{t('shifts.start')}</th>
                  <th className="text-left py-2 text-xs font-medium text-slate-400">{t('shifts.end')}</th>
                  <th className="text-left py-2 text-xs font-medium text-slate-400">{t('shifts.duration')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('shifts.starting_cash')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('shifts.expected_cash')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('shifts.actual_cash')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('shifts.variance')}</th>
                </tr>
              </thead>
              <tbody>
                {shifts.map(shift => (
                  <tr key={shift.source_id} className="border-b border-slate-50 last:border-0 hover:bg-slate-50 transition-colors">
                    <td className="py-2"><StatusBadge status={shift.status} abnormal={shift.abnormal_close} /></td>
                    <td className="py-2 text-slate-700">{shift.operator_name}</td>
                    <td className="py-2 text-slate-600 text-xs">{formatDateTime(shift.start_time)}</td>
                    <td className="py-2 text-slate-600 text-xs">{shift.end_time ? formatDateTime(shift.end_time) : '-'}</td>
                    <td className="py-2 text-slate-600 text-xs">{formatDuration(shift.start_time, shift.end_time)}</td>
                    <td className="py-2 text-right text-slate-700">{formatCurrency(shift.starting_cash)}</td>
                    <td className="py-2 text-right text-slate-700">{formatCurrency(shift.expected_cash)}</td>
                    <td className="py-2 text-right text-slate-700">{shift.actual_cash != null ? formatCurrency(shift.actual_cash) : '-'}</td>
                    <td className={`py-2 text-right font-medium ${
                      shift.cash_variance != null && shift.cash_variance < 0 ? 'text-red-500' :
                      shift.cash_variance != null && shift.cash_variance > 0 ? 'text-emerald-500' : 'text-slate-500'
                    }`}>
                      {shift.cash_variance != null ? formatCurrency(shift.cash_variance) : '-'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* Mobile cards */}
        <div className="md:hidden space-y-3">
          {shifts.map(shift => (
            <div key={shift.source_id} className="bg-white rounded-xl border border-slate-200 p-4">
              <div className="flex justify-between items-center mb-3 pb-3 border-b border-slate-50">
                <div className="flex items-center gap-2">
                  <StatusBadge status={shift.status} abnormal={shift.abnormal_close} />
                  <span className="text-sm font-semibold text-slate-900">{shift.operator_name}</span>
                </div>
                <span className="text-xs text-slate-400">{formatDuration(shift.start_time, shift.end_time)}</span>
              </div>
              <div className="grid grid-cols-2 gap-x-4 gap-y-3 text-xs">
                <div>
                  <p className="text-[10px] uppercase tracking-wider text-slate-400 font-medium mb-0.5">{t('shifts.start')}</p>
                  <p className="text-slate-700">{formatDateTime(shift.start_time)}</p>
                </div>
                <div>
                  <p className="text-[10px] uppercase tracking-wider text-slate-400 font-medium mb-0.5">{t('shifts.end')}</p>
                  <p className="text-slate-700">{shift.end_time ? formatDateTime(shift.end_time) : '-'}</p>
                </div>
                <div>
                  <p className="text-[10px] uppercase tracking-wider text-slate-400 font-medium mb-0.5">{t('shifts.starting_cash')}</p>
                  <p className="font-semibold text-slate-800">{formatCurrency(shift.starting_cash)}</p>
                </div>
                <div>
                  <p className="text-[10px] uppercase tracking-wider text-slate-400 font-medium mb-0.5">{t('shifts.variance')}</p>
                  <p className={`font-semibold ${
                    shift.cash_variance != null && shift.cash_variance < 0 ? 'text-red-500' :
                    shift.cash_variance != null && shift.cash_variance > 0 ? 'text-emerald-500' : 'text-slate-500'
                  }`}>
                    {shift.cash_variance != null ? formatCurrency(shift.cash_variance) : '-'}
                  </p>
                </div>
              </div>
              {shift.note && (
                <p className="mt-3 pt-3 border-t border-slate-50 text-xs text-slate-500">{shift.note}</p>
              )}
            </div>
          ))}
        </div>
        </>
      ) : (
        <div className="bg-white rounded-2xl border border-slate-200 p-8 text-center">
          <Clock className="w-10 h-10 text-slate-300 mx-auto mb-3" />
          <p className="text-sm text-slate-500">{t('shifts.no_data')}</p>
        </div>
      )}
    </div>
  );
};
