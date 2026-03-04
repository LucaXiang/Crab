import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { Sidebar } from './components/Sidebar';
import { Overview } from './components/Overview';
import { InvoiceList } from './components/InvoiceList';
import { ReportsAndShifts } from './components/ReportsAndShifts';
import { AuditLog } from './components/AuditLog';
import type { RedFlagsData } from './components/RedFlagsBar';
import type { TimeRange, ActiveTab, StoreOverview } from '@/core/domain/types';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { WheelDateTimePicker } from '@/shared/components/FormField';
import { useStoreInfo } from '@/core/stores/settings/useStoreInfoStore';

const EMPTY_OVERVIEW: StoreOverview = {
  revenue: 0, net_revenue: 0, orders: 0, guests: 0, average_order_value: 0,
  per_guest_spend: 0, average_dining_minutes: 0,
  total_tax: 0, total_discount: 0, total_surcharge: 0,
  avg_items_per_order: 0, voided_orders: 0, voided_amount: 0,
  loss_orders: 0, loss_amount: 0, anulacion_count: 0, anulacion_amount: 0,
  refund_count: 0, refund_amount: 0,
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
  const cutoffMs = cutoffMinutes * 60_000;
  const DAY = 86_400_000;
  const bizDayStart = todayStart.getTime() + cutoffMs;
  const todayBiz = now.getTime() < bizDayStart ? bizDayStart - DAY : bizDayStart;

  switch (range) {
    case 'today':
      return { from: todayBiz, to: todayBiz + DAY };
    case 'yesterday':
      return { from: todayBiz - DAY, to: todayBiz };
    case 'this_week': {
      const day = now.getDay(); // 0=Sun
      const daysSinceMonday = day === 0 ? 6 : day - 1;
      const weekStart = todayBiz - daysSinceMonday * DAY;
      return { from: weekStart, to: todayBiz + DAY };
    }
    case 'this_month': {
      const d = new Date(todayBiz);
      d.setDate(1);
      d.setHours(0, 0, 0, 0);
      const monthStart = d.getTime() + cutoffMs;
      return { from: monthStart, to: todayBiz + DAY };
    }
    case 'last_month': {
      const d = new Date(todayBiz);
      d.setDate(1);
      d.setHours(0, 0, 0, 0);
      const thisMonthStart = d.getTime() + cutoffMs;
      const d2 = new Date(thisMonthStart - DAY);
      d2.setDate(1);
      d2.setHours(0, 0, 0, 0);
      const lastMonthStart = d2.getTime() + cutoffMs;
      return { from: lastMonthStart, to: thisMonthStart };
    }
    default:
      return null;
  }
}

/** Compute the previous period for comparison. Uses calendar-aware logic for months. */
function previousRange(r: { from: number; to: number }, preset: TimeRange): { from: number; to: number } {
  if (preset === 'this_month' || preset === 'last_month') {
    const prevStart = new Date(r.from);
    prevStart.setMonth(prevStart.getMonth() - 1);
    const prevEnd = new Date(r.to);
    prevEnd.setMonth(prevEnd.getMonth() - 1);
    return { from: prevStart.getTime(), to: prevEnd.getTime() };
  }
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
  const [redFlags, setRedFlags] = useState<RedFlagsData | null>(null);

  const cutoffMinutes = storeInfo.business_day_cutoff ?? 0;
  const range = useMemo(
    () => computeRange(timeRange, cutoffMinutes, customStartDate, customEndDate),
    [timeRange, cutoffMinutes, customStartDate, customEndDate]
  );

  const fetchOverview = useCallback(async (from: number, to: number): Promise<StoreOverview> => {
    return invokeApi<StoreOverview>('get_statistics', { from, to });
  }, []);

  useEffect(() => {
    if (!isVisible || !range || activeTab !== 'overview') return;

    let cancelled = false;

    const load = async () => {
      try {
        const prev = previousRange(range, timeRange);
        const [current, prevData, lw, flags] = await Promise.all([
          fetchOverview(range.from, range.to),
          fetchOverview(prev.from, prev.to).catch(() => null),
          timeRange === 'today'
            ? fetchOverview(lastWeekRange(range).from, lastWeekRange(range).to).catch(() => null)
            : Promise.resolve(null),
          invokeApi<RedFlagsData>('get_red_flags', { from: range.from, to: range.to }).catch(() => null),
        ]);
        if (cancelled) return;
        setData(current);
        setPrevData(prevData);
        setLastWeekData(lw);
        setRedFlags(flags);
      } catch (error) {
        if (cancelled) return;
        logger.error('Failed to fetch statistics', error);
        toast.error(t('statistics.error.load'));
      }
    };

    load();
    return () => { cancelled = true; };
  }, [isVisible, range, timeRange, activeTab, fetchOverview, t]);

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
              {activeTab === 'invoices' && t('statistics.sidebar.invoices')}
              {activeTab === 'reports_shifts' && t('statistics.sidebar.reports_shifts')}
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
                <option value="yesterday">{t('statistics.time.yesterday')}</option>
                <option value="this_week">{t('statistics.time.this_week')}</option>
                <option value="this_month">{t('statistics.time.this_month')}</option>
                <option value="last_month">{t('statistics.time.last_month')}</option>
                <option value="custom">{t('statistics.time.custom')}</option>
              </select>
            </div>
          </div>

          {activeTab === 'overview' && (
            <Overview
              overview={data}
              previousOverview={prevData}
              lastWeekOverview={lastWeekData}
              cutoffMinutes={cutoffMinutes}
              redFlags={redFlags}
            />
          )}

          {activeTab === 'invoices' && range && (
            <InvoiceList from={range.from} to={range.to} />
          )}

          {activeTab === 'reports_shifts' && range && (
            <ReportsAndShifts from={range.from} to={range.to} />
          )}

          {activeTab === 'audit_log' && (
            <AuditLog />
          )}
        </div>
      </div>
    </div>
  );
};
