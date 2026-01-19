import React, { useState, useEffect } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { BarChart as BarChartIcon } from 'lucide-react';
import { toast } from '@/presentation/components/Toast';
import { Sidebar } from './components/Sidebar';
import { Overview } from './components/Overview';
import { SalesReport } from './components/SalesReport';
import { TimeRange, ActiveTab, StatisticsResponse } from '@/core/domain/types';
import { getStatistics } from '@/infrastructure/apiValidator';

interface StatisticsScreenProps {
  isVisible: boolean;
  onBack: () => void;
}

export const StatisticsScreen: React.FC<StatisticsScreenProps> = ({ isVisible, onBack }) => {
  const { t } = useI18n();
  const [activeTab, setActiveTab] = useState<ActiveTab>('overview');
  const [timeRange, setTimeRange] = useState<TimeRange>('today');
  const [customStartDate, setCustomStartDate] = useState<string>('');
  const [customEndDate, setCustomEndDate] = useState<string>('');


  const [data, setData] = useState<StatisticsResponse>({
    overview: {
      todayRevenue: 0,
      todayOrders: 0,
      todayCustomers: 0,
      averageOrderValue: 0,
      cashRevenue: 0,
      cardRevenue: 0,
      otherRevenue: 0,
      voidedOrders: 0,
      voidedAmount: 0,
      totalDiscount: 0,
      avgGuestSpend: 0,
      avgDiningTime: 0,
    },
    revenueTrend: [],
    categorySales: [],
    topProducts: [],
  });

  useEffect(() => {
    if (isVisible) {
      if (timeRange === 'custom' && (!customStartDate || !customEndDate)) {
        return;
      }
      fetchStatistics();
    }
  }, [isVisible, timeRange, customStartDate, customEndDate]);

  const fetchStatistics = async () => {
    try {
      if (!('__TAURI__' in window)) {
        console.warn('[Statistics] Not running inside Tauri; skipping invoke');
        toast.error(t('statistics.error.load'));
        return;
      }

      const result = await getStatistics(timeRange, customStartDate, customEndDate);
      setData(result);
    } catch (error) {
      console.error('Failed to fetch statistics:', error);
      toast.error(t('statistics.error.load'));
    } finally {
    }
  };

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
      <div className="flex-1 overflow-y-auto p-8 min-w-0">
        <div className="max-w-7xl mx-auto">
          <div className="flex items-center justify-between mb-8">
            <h1 className="text-2xl font-bold text-gray-800">
              {activeTab === 'overview' && t('statistics.sidebar.overview')}
              {activeTab === 'sales' && t("statistics.report.sales")}
              {activeTab === 'products' && t("statistics.report.product")}
              {activeTab === 'categories' && t("statistics.report.category")}
            </h1>

            <div className="flex items-center gap-3">
              {timeRange === 'custom' && (
                <div className="flex items-center gap-2 bg-white rounded-md border border-gray-200 p-1 shadow-sm">
                  <input 
                    type="datetime-local" 
                    value={customStartDate} 
                    onChange={(e) => setCustomStartDate(e.target.value)}
                    className="text-sm border-none focus:ring-0 text-gray-600 p-1 outline-none"
                  />
                  <span className="text-gray-400">-</span>
                  <input 
                    type="datetime-local" 
                    value={customEndDate} 
                    onChange={(e) => setCustomEndDate(e.target.value)}
                    className="text-sm border-none focus:ring-0 text-gray-600 p-1 outline-none"
                  />
                </div>
              )}
              
              <select 
                value={timeRange} 
                onChange={(e) => setTimeRange(e.target.value as TimeRange)} 
                className="text-sm border-gray-200 rounded-md text-gray-500 focus:ring-blue-500 focus:border-blue-500 bg-white py-2 pl-3 pr-8 shadow-sm cursor-pointer" 
              > 
                <option value="today">{ t ('statistics.time.today')}</option> 
                <option value="week">{ t ('statistics.time.week')}</option> 
                <option value="month">{ t ('statistics.time.month')}</option> 
                <option value="custom">{ t ('statistics.time.custom') || 'Custom'}</option> 
              </select>
            </div>

          </div>

          {activeTab === 'overview' && (
            <Overview 
              overview={data.overview}
              revenueTrend={data.revenueTrend}
              categorySales={data.categorySales}
              topProducts={data.topProducts}
              timeRange={timeRange}
              onTimeRangeChange={setTimeRange}
              customStartDate={customStartDate}
              customEndDate={customEndDate}
              onCustomStartDateChange={setCustomStartDate}
              onCustomEndDateChange={setCustomEndDate}
            />
          )}

          {activeTab === 'sales' && (
            <SalesReport 
              timeRange={timeRange}
              customStartDate={customStartDate}
              customEndDate={customEndDate}
            />
          )}

          {(activeTab === 'products' || activeTab === 'categories') && (
            <div className="flex flex-col items-center justify-center h-96 text-gray-400">
              <BarChartIcon size={48} className="mb-4 opacity-20" />
              <p>{t("statistics.report.detailedComingSoon")}</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
