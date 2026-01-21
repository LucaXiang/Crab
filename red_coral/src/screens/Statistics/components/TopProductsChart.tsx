import React from 'react';
import { 
  BarChart, 
  Bar, 
  XAxis, 
  YAxis, 
  CartesianGrid, 
  Tooltip, 
  ResponsiveContainer 
} from 'recharts';
import { useI18n } from '@/hooks/useI18n';
import { TopProduct } from '@/core/domain/types';

interface TopProductsChartProps {
  data: TopProduct[];
}

export const TopProductsChart: React.FC<TopProductsChartProps> = ({ data }) => {
  const { t } = useI18n();

  return (
    <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-100 min-w-0">
      <h3 className="text-lg font-bold text-gray-800 mb-6">{t('statistics.topProducts')}</h3>
      <div className="min-w-0">
        {data.length > 0 ? (
          <ResponsiveContainer width="100%" height={250}>
            <BarChart data={data} layout="vertical" margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
              <CartesianGrid strokeDasharray="3 3" horizontal={true} vertical={false} stroke="#E5E7EB" />
              <XAxis type="number" hide />
              <YAxis 
                dataKey="name" 
                type="category" 
                width={100} 
                tick={{ fill: '#4B5563', fontSize: 12 }}
                axisLine={false}
                tickLine={false}
              />
              <Tooltip cursor={{ fill: '#F3F4F6' }} />
              <Bar dataKey="sales" fill="#8B5CF6" radius={[0, 4, 4, 0]} barSize={20} />
            </BarChart>
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
