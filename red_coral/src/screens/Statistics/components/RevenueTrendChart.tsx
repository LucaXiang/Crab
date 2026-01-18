import React from 'react';
import { 
  AreaChart, 
  Area, 
  XAxis, 
  YAxis, 
  CartesianGrid, 
  Tooltip, 
  ResponsiveContainer 
} from 'recharts';
import { useI18n } from '@/hooks/useI18n';
import { RevenueTrendPoint, TimeRange } from '@/core/domain/types';
import { CustomTooltip } from './CustomTooltip';
import { formatCurrency } from '@/utils/formatCurrency';

interface RevenueTrendChartProps {
  data: RevenueTrendPoint[];
  timeRange: TimeRange;
  onTimeRangeChange: (range: TimeRange) => void;
}

export const RevenueTrendChart: React.FC<RevenueTrendChartProps> = ({ 
  data, 
  timeRange, 
  onTimeRangeChange 
}) => {
  const { t } = useI18n();

  return (
    <div className="lg:col-span-2 bg-white p-6 rounded-xl shadow-sm border border-gray-100 min-w-0">
      <div className="flex items-center justify-between mb-6">
        <h3 className="text-lg font-bold text-gray-800">{t("statistics.chart.revenueTrend")}</h3>
      </div>
      <div className="min-w-0">
        {data.length > 0 ? (
          <ResponsiveContainer width="100%" height={300}>
            <AreaChart data={data}>
              <defs>
                <linearGradient id="colorRevenue" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#3B82F6" stopOpacity={0.1}/>
                  <stop offset="95%" stopColor="#3B82F6" stopOpacity={0}/>
                </linearGradient>
              </defs>
              <CartesianGrid strokeDasharray="3 3" vertical={false} stroke="#E5E7EB" />
              <XAxis 
                dataKey="time" 
                axisLine={false} 
                tickLine={false} 
                tick={{ fill: '#6B7280', fontSize: 12 }} 
                dy={10}
              />
              <YAxis 
	                axisLine={false} 
	                tickLine={false} 
	                tick={{ fill: '#6B7280', fontSize: 12 }}
	                tickFormatter={(value) => formatCurrency(Number(value))}
	              />
              <Tooltip content={<CustomTooltip />} />
              <Area 
                type="monotone" 
                dataKey="value" 
                stroke="#3B82F6" 
                strokeWidth={3}
                fillOpacity={1} 
                fill="url(#colorRevenue)" 
              />
            </AreaChart>
          </ResponsiveContainer>
        ) : (
          <div className="flex items-center justify-center h-full text-gray-400">
            {t('statistics.noData')}
          </div>
        )}
      </div>
    </div>
  );
};
