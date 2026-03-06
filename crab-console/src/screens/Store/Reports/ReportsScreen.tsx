import React, { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { ScrollText } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getStats } from '@/infrastructure/api/stats';
import { ApiError } from '@/infrastructure/api/client';
import { formatCurrency } from '@/utils/format';
import { Spinner } from '@/presentation/components/ui/Spinner';
import type { DailyReportEntry } from '@/core/types/stats';

export const ReportsScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [reports, setReports] = useState<DailyReportEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    const tk = useAuthStore.getState().token;
    if (!tk) return;
    (async () => {
      try {
        setReports(await getStats(tk, storeId));
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
      <h1 className="text-xl font-bold text-slate-900">{t('stats.daily_report')}</h1>

      {reports.length > 0 ? (
        <>
        {/* Desktop table */}
        <div className="hidden md:block bg-white rounded-2xl border border-slate-200 p-6">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-slate-100">
                  <th className="text-left py-2 text-xs font-medium text-slate-400">{t('stats.business_date')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('stats.net_revenue')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('stats.total_orders')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('stats.refunds')}</th>
                  <th className="text-center py-2 text-xs font-medium text-slate-400">{t('stats.source')}</th>
                </tr>
              </thead>
              <tbody>
                {reports.map(entry => (
                    <tr key={entry.id} className="border-b border-slate-50 last:border-0 hover:bg-slate-50 transition-colors">
                      <td className="py-2 text-slate-700">
                        <Link to={`/stores/${storeId}/reports/${entry.business_date}`} className="text-primary-500 hover:text-primary-600 font-medium hover:underline">
                          {entry.business_date}
                        </Link>
                      </td>
                      <td className="py-2 text-right font-semibold text-emerald-600">{formatCurrency(entry.net_revenue)}</td>
                      <td className="py-2 text-right text-slate-700">{entry.total_orders}</td>
                      <td className="py-2 text-right">
                        <span className={entry.refund_amount > 0 ? 'text-red-600 font-medium' : 'text-slate-400'}>{formatCurrency(entry.refund_amount)}</span>
                        {entry.refund_count > 0 && <span className="text-xs text-slate-400 ml-1">({entry.refund_count})</span>}
                      </td>
                      <td className="py-2 text-center">
                        <span className={`px-2 py-0.5 rounded text-xs font-medium ${entry.auto_generated ? 'bg-blue-100 text-blue-700' : 'bg-purple-100 text-purple-700'}`}>
                          {entry.auto_generated ? t('stats.source_auto') : t('stats.source_manual')}
                        </span>
                      </td>
                    </tr>
                  ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* Mobile cards */}
        <div className="md:hidden space-y-3">
          {reports.map(entry => (
              <Link key={entry.id} to={`/stores/${storeId}/reports/${entry.business_date}`} className="block bg-white rounded-xl border border-slate-200 p-4 hover:border-primary-200 transition-all active:scale-[0.99]">
                <div className="flex justify-between items-center mb-3 pb-3 border-b border-slate-50">
                  <div className="flex items-center gap-2">
                    <span className="text-slate-900 font-bold">{entry.business_date}</span>
                    <span className={`px-1.5 py-0.5 rounded text-[10px] font-medium ${entry.auto_generated ? 'bg-blue-100 text-blue-700' : 'bg-purple-100 text-purple-700'}`}>
                      {entry.auto_generated ? t('stats.source_auto') : t('stats.source_manual')}
                    </span>
                  </div>
                  <div className="text-emerald-600 font-bold text-lg">{formatCurrency(entry.net_revenue)}</div>
                </div>
                <div className="grid grid-cols-2 gap-x-4 gap-y-3">
                  <div>
                    <p className="text-[10px] uppercase tracking-wider text-slate-400 font-medium mb-0.5">{t('stats.total_orders')}</p>
                    <p className="font-semibold text-slate-800">{entry.total_orders}</p>
                  </div>
                  <div>
                    <p className="text-[10px] uppercase tracking-wider text-slate-400 font-medium mb-0.5">{t('stats.refunds')}</p>
                    <p className={`font-medium ${entry.refund_amount > 0 ? 'text-red-500' : 'text-slate-400'}`}>
                      {formatCurrency(entry.refund_amount)}
                      {entry.refund_count > 0 && <span className="text-xs text-slate-400 ml-1">({entry.refund_count})</span>}
                    </p>
                  </div>
                </div>
              </Link>
            ))}
        </div>
        </>
      ) : (
        <div className="bg-white rounded-2xl border border-slate-200 p-8 text-center">
          <ScrollText className="w-10 h-10 text-slate-300 mx-auto mb-3" />
          <p className="text-sm text-slate-500">{t('stats.no_data')}</p>
        </div>
      )}
    </div>
  );
};
