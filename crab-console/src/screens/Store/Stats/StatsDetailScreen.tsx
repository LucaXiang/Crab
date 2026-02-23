import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import { BarChart3, Calendar } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getStoreOverview } from '@/infrastructure/api/stats';
import { ApiError } from '@/infrastructure/api/client';
import { Spinner } from '@/presentation/components/ui/Spinner';
import type { StoreOverview } from '@/core/types/stats';
import { StoreOverviewDisplay } from '../Overview/StoreOverviewDisplay';

function getDateRange(dateStr: string): { from: number; to: number } {
  const start = new Date(dateStr + 'T00:00:00');
  const end = new Date(dateStr + 'T23:59:59.999');
  return { from: start.getTime(), to: end.getTime() };
}

export const StatsDetailScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const { date: dateStr } = useParams<{ date: string }>();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [overview, setOverview] = useState<StoreOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!token || !dateStr) return;
    (async () => {
      try {
        const { from, to } = getDateRange(dateStr);
        setOverview(await getStoreOverview(token, storeId, from, to));
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
        setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      } finally {
        setLoading(false);
      }
    })();
  }, [token, storeId, dateStr, clearAuth, navigate, t]);

  if (loading) return <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>;
  if (error) return <div className="max-w-5xl mx-auto px-6 py-8"><div className="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div></div>;

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
      <div className="flex items-center gap-2">
        <h1 className="text-lg md:text-xl font-bold text-slate-900">{t('stats.daily_report')}</h1>
        <span className="text-slate-300">/</span>
        <span className="text-slate-600 font-medium flex items-center gap-1">
          <Calendar className="w-4 h-4" />
          {dateStr}
        </span>
      </div>

      {overview ? (
        <StoreOverviewDisplay overview={overview} showHeader={false} />
      ) : (
        <div className="bg-white rounded-2xl border border-slate-200 p-8 text-center">
          <BarChart3 className="w-10 h-10 text-slate-300 mx-auto mb-3" />
          <p className="text-sm text-slate-500">{t('stats.no_data')}</p>
        </div>
      )}
    </div>
  );
};
