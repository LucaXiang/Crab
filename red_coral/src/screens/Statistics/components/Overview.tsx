import React from 'react';
import { useI18n } from '@/hooks/useI18n';
import { StatsCards } from './StatsCards';
import { RevenueTrendChart } from './RevenueTrendChart';
import { CategoryDistributionChart } from './CategoryDistributionChart';
import { TopProductsChart } from './TopProductsChart';
import { OverviewStats, RevenueTrendPoint, CategorySale, TopProduct, TimeRange } from '@/core/domain/types';

interface OverviewProps {
  overview: OverviewStats;
  revenue_trend: RevenueTrendPoint[];
  category_sales: CategorySale[];
  top_products: TopProduct[];
  timeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
  customStartDate?: string;
  customEndDate?: string;
  onCustomStartDateChange?: (date: string) => void;
  onCustomEndDateChange?: (date: string) => void;
}

export const Overview: React.FC<OverviewProps> = ({
  overview,
  revenue_trend,
  category_sales,
  top_products,
  timeRange,
  onTimeRangeChange,
  customStartDate,
  customEndDate,
  onCustomStartDateChange,
  onCustomEndDateChange,
}) => {
  const { t } = useI18n();
  const hasData = overview.orders > 0 || revenue_trend.length > 0;

  return (
    <div className="space-y-6 min-w-0">


      <StatsCards overview={overview} />

      {/* Main Charts Area */}
      {hasData ? (
        <>
          <div className="grid grid-cols-1 lg:grid-cols-3 gap-6 min-w-0">
            <RevenueTrendChart 
              data={revenue_trend} 
              timeRange={timeRange} 
              onTimeRangeChange={onTimeRangeChange} 
            />
            <CategoryDistributionChart data={category_sales} />
          </div>

          <TopProductsChart data={top_products} />
        </>
      ) : (
        <div className="flex flex-col items-center justify-center py-20 bg-white rounded-xl border border-gray-100 shadow-sm">
          <div className="p-4 bg-gray-50 rounded-full mb-4">
            <svg className="w-12 h-12 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
          </div>
          <h3 className="text-lg font-medium text-gray-900 mb-1">{t('common.empty.no_data')}</h3>
          <p className="text-sm text-gray-500">{t('statistics.sidebar.analytics')}</p>
        </div>
      )}
    </div>
  );
};
