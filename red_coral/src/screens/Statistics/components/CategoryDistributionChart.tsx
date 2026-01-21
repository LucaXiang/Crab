import React from 'react';
import {
  PieChart,
  Pie,
  Cell,
  Legend,
  Tooltip,
  ResponsiveContainer
} from 'recharts';
import { useI18n } from '@/hooks/useI18n';
import { CategorySale } from '@/core/domain/types';

interface CategoryDistributionChartProps {
  data: CategorySale[];
}

// Define chart data format
interface ChartDataItem {
  name: string;
  value: number;
  color?: string;
}

// Predefined colors for chart
const COLORS = ['#3B82F6', '#10B981', '#F59E0B', '#EF4444', '#8B5CF6', '#EC4899', '#06B6D4', '#84CC16'];

export const CategoryDistributionChart: React.FC<CategoryDistributionChartProps> = ({ data }) => {
  const { t } = useI18n();

  // Transform data for chart
  const chartData: ChartDataItem[] = data.map((item, index) => ({
    name: item.name,  // Backend returns 'name' field
    value: item.value,  // Backend returns 'value' field
    color: item.color || COLORS[index % COLORS.length]
  }));

  return (
    <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-100 min-w-0">
      <h3 className="text-lg font-bold text-gray-800 mb-6">{t("statistics.chart.salesByCategory")}</h3>
      <div className="min-w-0">
        {data.length > 0 ? (
          <ResponsiveContainer width="100%" height={300}>
            <PieChart>
              <Pie
                data={chartData as any}
                cx="50%"
                cy="50%"
                innerRadius={60}
                outerRadius={80}
                paddingAngle={5}
                dataKey="value"
                nameKey="name"
              >
                {chartData.map((entry, index) => (
                  <Cell key={`cell-${index}`} fill={entry.color} />
                ))}
              </Pie>
              <Tooltip />
              <Legend verticalAlign="bottom" height={36} />
            </PieChart>
          </ResponsiveContainer>
        ) : (
          <div className="flex items-center justify-center h-full text-gray-400">
            {t('common.empty.noData')}
          </div>
        )}
      </div>
    </div>
  );
};
