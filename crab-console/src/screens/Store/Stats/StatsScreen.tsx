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

export const StatsScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [reports, setReports] = useState<DailyReportEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!token) return;
    (async () => {
      try {
        setReports(await getStats(token, storeId));
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
        setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      } finally {
        setLoading(false);
      }
    })();
  }, [token, storeId, clearAuth, navigate, t]);

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
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('stats.total_sales')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('stats.completed_orders')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('stats.void_orders')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('stats.total_paid')}</th>
                  <th className="text-right py-2 text-xs font-medium text-slate-400">{t('stats.total_discount')}</th>
                </tr>
              </thead>
              <tbody>
                {reports.map(entry => (
                    <tr key={entry.id} className="border-b border-slate-50 last:border-0 hover:bg-slate-50 transition-colors">
                      <td className="py-2 text-slate-700">
                        <Link to={`/stores/${storeId}/stats/${entry.business_date}`} className="text-primary-500 hover:text-primary-600 font-medium hover:underline">
                          {entry.business_date}
                        </Link>
                      </td>
                      <td className="py-2 text-right font-semibold text-slate-900">{formatCurrency(entry.total_sales)}</td>
                      <td className="py-2 text-right text-slate-700">{entry.completed_orders}</td>
                      <td className="py-2 text-right text-slate-700">{entry.void_orders}</td>
                      <td className="py-2 text-right text-slate-700">{formatCurrency(entry.total_paid)}</td>
                      <td className="py-2 text-right text-orange-500">{formatCurrency(entry.total_discount)}</td>
                    </tr>
                  ))}
              </tbody>
            </table>
          </div>
        </div>

        {/* Mobile cards */}
        <div className="md:hidden space-y-3">
          {reports.map(entry => (
              <Link key={entry.id} to={`/stores/${storeId}/stats/${entry.business_date}`} className="block bg-white rounded-xl border border-slate-200 p-4 hover:border-primary-200 transition-colors">
                <div className="text-primary-500 font-medium mb-2">{entry.business_date}</div>
                <div className="grid grid-cols-2 gap-x-4 gap-y-2">
                  <div>
                    <p className="text-xs text-slate-400">{t('stats.total_sales')}</p>
                    <p className="font-semibold text-slate-900">{formatCurrency(entry.total_sales)}</p>
                  </div>
                  <div>
                    <p className="text-xs text-slate-400">{t('stats.completed_orders')}</p>
                    <p className="font-semibold text-slate-900">{entry.completed_orders}</p>
                  </div>
                  <div>
                    <p className="text-xs text-slate-400">{t('stats.total_paid')}</p>
                    <p className="text-slate-700">{formatCurrency(entry.total_paid)}</p>
                  </div>
                  <div>
                    <p className="text-xs text-slate-400">{t('stats.total_discount')}</p>
                    <p className="text-orange-500">{formatCurrency(entry.total_discount)}</p>
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
