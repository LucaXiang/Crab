import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { Sidebar } from './components/Sidebar';
import { Overview } from './components/Overview';
import { SalesReport } from './components/SalesReport';
import { DailyReportManagement } from '@/features/daily-report/DailyReportManagement';
import { AuditLog } from './components/AuditLog';
import type { TimeRange, ActiveTab, StoreOverview } from '@/core/domain/types';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { WheelDateTimePicker } from '@/shared/components/FormField';
import { useStoreInfo } from '@/core/stores/settings/useStoreInfoStore';

const EMPTY_OVERVIEW: StoreOverview = {
  revenue: 0, orders: 0, guests: 0, average_order_value: 0,
  per_guest_spend: 0, average_dining_minutes: 0,
  total_tax: 0, total_discount: 0, total_surcharge: 0,
  avg_items_per_order: 0, voided_orders: 0, voided_amount: 0,
  loss_orders: 0, loss_amount: 0, refund_count: 0, refund_amount: 0,
  revenue_trend: [], daily_trend: [], payment_breakdown: [],
  tax_breakdown: [], category_sales: [], top_products: [],
  tag_sales: [], refund_method_breakdown: [], service_type_breakdown: [],
  zone_sales: [],
};

/** Compute from/to millis for a given time range relative to a cutoff. */
function computeRange(range: TimeRange, cutoffMinutes: number, customStart?: string, customEnd?: string): { from: number; to: number } | null {
  if (range === 'custom') {
    if (!customStart || !customEnd) return null;
    return { from: new Date(customStart).getTime(), to: new Date(customEnd).getTime() };
  }

  const now = new Date();
  const todayStart = new Date(now);
  todayStart.setHours(0, 0, 0, 0);
  // Shift by cutoff
  const cutoffMs = cutoffMinutes * 60_000;
  const bizDayStart = todayStart.getTime() + cutoffMs;
  // If we haven't reached the cutoff yet, roll back one day
  const effectiveStart = now.getTime() < bizDayStart ? bizDayStart - 86_400_000 : bizDayStart;

  if (range === 'today') {
    return { from: effectiveStart, to: effectiveStart + 86_400_000 };
  }
  if (range === 'week') {
    return { from: effectiveStart - 6 * 86_400_000, to: effectiveStart + 86_400_000 };
  }
  if (range === 'month') {
    return { from: effectiveStart - 29 * 86_400_000, to: effectiveStart + 86_400_000 };
  }
  return null;
}

/** Compute the previous period (same duration, immediately before). */
function previousRange(r: { from: number; to: number }): { from: number; to: number } {
  const duration = r.to - r.from;
  return { from: r.from - duration, to: r.from };
}

/** Compute same day last week. */
function lastWeekRange(r: { from: number; to: number }): { from: number; to: number } {
  const shift = 7 * 86_400_000;
  return { from: r.from - shift, to: r.to - shift };
}

interface StatisticsScreenProps {
  isVisible: boolean;
  onBack: () => void;
}

export const StatisticsScreen: React.FC<StatisticsScreenProps> = ({ isVisible, onBack }) => {
  const { t } = useI18n();
  const storeInfo = useStoreInfo();
  const [activeTab, setActiveTab] = useState<ActiveTab>('overview');
  const [timeRange, setTimeRange] = useState<TimeRange>('today');
  const [customStartDate, setCustomStartDate] = useState<string>('');
  const [customEndDate, setCustomEndDate] = useState<string>('');

  const [data, setData] = useState<StoreOverview>(EMPTY_OVERVIEW);
  const [prevData, setPrevData] = useState<StoreOverview | null>(null);
  const [lastWeekData, setLastWeekData] = useState<StoreOverview | null>(null);

  // Parse cutoff — backend now sends i32 (minutes), but TS type is still string until Task 6
  const cutoffMinutes = useMemo(() => {
    const raw = storeInfo.business_day_cutoff;
    if (typeof raw === 'number') return raw;
    // Legacy string format "HH:MM"
    if (typeof raw === 'string' && raw.includes(':')) {
      const [h, m] = raw.split(':').map(Number);
      return (h || 0) * 60 + (m || 0);
    }
    return Number(raw) || 0;
  }, [storeInfo.business_day_cutoff]);

  const fetchOverview = useCallback(async (from: number, to: number): Promise<StoreOverview> => {
    return invokeApi<StoreOverview>('get_statistics', { from, to });
  }, []);

  useEffect(() => {
    if (!isVisible) return;

    const range = computeRange(timeRange, cutoffMinutes, customStartDate, customEndDate);
    if (!range) return;

    let cancelled = false;

    const load = async () => {
      try {
        const [current, prev, lw] = await Promise.all([
          fetchOverview(range.from, range.to),
          fetchOverview(previousRange(range).from, previousRange(range).to).catch(() => null),
          timeRange === 'today'
            ? fetchOverview(lastWeekRange(range).from, lastWeekRange(range).to).catch(() => null)
            : Promise.resolve(null),
        ]);
        if (cancelled) return;
        setData(current);
        setPrevData(prev);
        setLastWeekData(lw);
      } catch (error) {
        if (cancelled) return;
        logger.error('Failed to fetch statistics', error);
        toast.error(t('statistics.error.load'));
      }
    };

    load();
    return () => { cancelled = true; };
  }, [isVisible, timeRange, customStartDate, customEndDate, cutoffMinutes, fetchOverview, t]);

  if (!isVisible) return null;

  return (
    <div className="flex h-full w-full bg-gray-50 overflow-hidden font-sans">
      <Sidebar
        onBack={onBack}
        activeTab={activeTab}
        onTabChange={setActiveTab}
        timeRange={timeRange}
        customStartDate={customStartDate}
        customEndDate={customEndDate}
      />

      {/* Main Content */}
      <div className="flex-1 overflow-y-auto p-8 min-w-0" style={{ scrollbarGutter: 'stable' }}>
        <div className="max-w-7xl mx-auto">
          <div className="flex items-center justify-between mb-8">
            <h1 className="text-2xl font-bold text-gray-800">
              {activeTab === 'overview' && t('statistics.sidebar.overview')}
              {activeTab === 'sales' && t('statistics.report.sales')}
              {activeTab === 'daily_report' && t('statistics.sidebar.daily_report')}
              {activeTab === 'audit_log' && t('statistics.sidebar.audit_log')}
            </h1>

            <div className="flex items-center gap-3">
              {timeRange === 'custom' && (
                <div className="flex items-center gap-2 bg-white rounded-md border border-gray-200 p-1 shadow-sm">
                  <WheelDateTimePicker
                    value={customStartDate}
                    onChange={setCustomStartDate}
                  />
                  <span className="text-gray-400">-</span>
                  <WheelDateTimePicker
                    value={customEndDate}
                    onChange={setCustomEndDate}
                  />
                </div>
              )}

              <select
                value={timeRange}
                onChange={(e) => setTimeRange(e.target.value as TimeRange)}
                className="text-sm border-gray-200 rounded-md text-gray-500 focus:ring-blue-500 focus:border-blue-500 bg-white py-2 pl-3 pr-8 shadow-sm cursor-pointer"
              >
                <option value="today">{t('statistics.time.today')}</option>
                <option value="week">{t('statistics.time.week')}</option>
                <option value="month">{t('statistics.time.month')}</option>
                <option value="custom">{t('statistics.time.custom') || 'Custom'}</option>
              </select>
            </div>
          </div>

          {activeTab === 'overview' && (
            <Overview
              overview={data}
              previousOverview={prevData}
              lastWeekOverview={lastWeekData}
              cutoffMinutes={cutoffMinutes}
            />
          )}

          {activeTab === 'sales' && (
            <SalesReport
              timeRange={timeRange}
              customStartDate={customStartDate}
              customEndDate={customEndDate}
            />
          )}

          {activeTab === 'daily_report' && (
            <DailyReportManagement />
          )}

          {activeTab === 'audit_log' && (
            <AuditLog />
          )}
        </div>
      </div>
    </div>
  );
};
