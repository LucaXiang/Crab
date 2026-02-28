import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { BarChart3 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getStoreOverview } from '@/infrastructure/api/stats';
import { ApiError } from '@/infrastructure/api/client';
import { Spinner } from '@/presentation/components/ui/Spinner';
import { TimeRangeSelector, getPresetRange, getPreviousRange, getLastWeekSameDayRange } from '@/shared/components';
import type { TimeRange } from '@/shared/components';
import type { StoreOverview } from '@/core/types/stats';
import { StoreOverviewDisplay } from './StoreOverviewDisplay';

export const StoreOverviewScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [timeRange, setTimeRange] = useState<TimeRange>(() => getPresetRange('today', t));
  const [overview, setOverview] = useState<StoreOverview | null>(null);
  const [previousOverview, setPreviousOverview] = useState<StoreOverview | null>(null);
  const [lastWeekOverview, setLastWeekOverview] = useState<StoreOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  const fetchData = useCallback(async (range: TimeRange) => {
    if (!token) return;
    setLoading(true);
    setError('');
    try {
      const prevRange = getPreviousRange(range);
      const lwRange = getLastWeekSameDayRange(range);
      const [current, prev, lastWeek] = await Promise.all([
        getStoreOverview(token, storeId, range.from, range.to),
        getStoreOverview(token, storeId, prevRange.from, prevRange.to),
        getStoreOverview(token, storeId, lwRange.from, lwRange.to),
      ]);
      setOverview(current);
      setPreviousOverview(prev);
      setLastWeekOverview(lastWeek);
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  }, [token, storeId, clearAuth, navigate, t]);

  useEffect(() => { fetchData(timeRange); }, [fetchData, timeRange]);

  const handleRangeChange = (range: TimeRange) => {
    setTimeRange(range);
  };

  if (error) return <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8"><div className="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div></div>;

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
      <div className="flex flex-col gap-3">
        <h1 className="text-lg md:text-xl font-bold text-slate-900">{t('stats.overview')}</h1>
        <TimeRangeSelector value={timeRange} onChange={handleRangeChange} />
      </div>

      {loading ? (
        <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>
      ) : !overview ? (
        <div className="bg-white rounded-2xl border border-slate-200 p-8 text-center">
          <BarChart3 className="w-10 h-10 text-slate-300 mx-auto mb-3" />
          <p className="text-sm text-slate-500">{t('stats.no_data')}</p>
        </div>
      ) : (
        <StoreOverviewDisplay overview={overview} previousOverview={previousOverview} lastWeekOverview={lastWeekOverview} showHeader={false} rangeLabel={timeRange.label} />
      )}
    </div>
  );
};
