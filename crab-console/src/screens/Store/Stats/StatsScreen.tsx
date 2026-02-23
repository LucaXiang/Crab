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

interface DailyReport {
  business_date: string;
  total_orders: number;
  completed_orders: number;
  void_orders: number;
  total_sales: number;
  total_paid: number;
  total_discount: number;
}

function parseReport(entry: DailyReportEntry): DailyReport {
  const d = entry.data as Record<string, unknown>;
  return {
    business_date: (d.business_date as string) ?? '',
    total_orders: (d.total_orders as number) ?? 0,
    completed_orders: (d.completed_orders as number) ?? 0,
    void_orders: (d.void_orders as number) ?? 0,
    total_sales: (d.total_sales as number) ?? 0,
    total_paid: (d.total_paid as number) ?? 0,
    total_discount: (d.total_discount as number) ?? 0,
  };
}

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
        <div className="bg-white rounded-2xl border border-slate-200 p-6">
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
                {reports.map(entry => {
                  const r = parseReport(entry);
                  return (
                    <tr key={entry.id} className="border-b border-slate-50 last:border-0 hover:bg-slate-50 transition-colors">
                      <td className="py-2 text-slate-700">
                        <Link to={`/stores/${storeId}/stats/${r.business_date}`} className="text-primary-500 hover:text-primary-600 font-medium hover:underline">
                          {r.business_date}
                        </Link>
                      </td>
                      <td className="py-2 text-right font-semibold text-slate-900">{formatCurrency(r.total_sales)}</td>
                      <td className="py-2 text-right text-slate-700">{r.completed_orders}</td>
                      <td className="py-2 text-right text-slate-700">{r.void_orders}</td>
                      <td className="py-2 text-right text-slate-700">{formatCurrency(r.total_paid)}</td>
                      <td className="py-2 text-right text-orange-500">{formatCurrency(r.total_discount)}</td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        </div>
      ) : (
        <div className="bg-white rounded-2xl border border-slate-200 p-8 text-center">
          <ScrollText className="w-10 h-10 text-slate-300 mx-auto mb-3" />
          <p className="text-sm text-slate-500">{t('stats.no_data')}</p>
        </div>
      )}
    </div>
  );
};
