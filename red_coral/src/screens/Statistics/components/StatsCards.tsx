import React from 'react';
import { Euro, ShoppingCart, Users, CreditCard, Banknote, TrendingUp, Ban, Tag, Clock, UserCheck } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { OverviewStats, TimeRange } from '@/core/domain/types';
import { formatCurrency } from '@/utils/formatCurrency';

interface StatsCardsProps {
  overview: OverviewStats;
  timeRange: TimeRange;
}

export const StatsCards: React.FC<StatsCardsProps> = ({ overview, timeRange }) => {
  const { t } = useI18n();

  const getLabel = (key: string) => {
    return t(`statistics.${key}.${timeRange}`);
  };

  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      {/* 1. Revenue */}
      <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
        <div className="flex items-center justify-between mb-3">
          <div className="p-3 bg-green-100 rounded-lg">
            <Euro className="text-green-600" size={20} />
          </div>
        </div>
        <div className="text-2xl font-bold text-gray-800 mb-1">
          {formatCurrency(overview.todayRevenue)}
        </div>
        <div className="text-sm text-gray-500">{getLabel('revenue')}</div>
      </div>

      {/* 2. Orders */}
      <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
        <div className="flex items-center justify-between mb-3">
          <div className="p-3 bg-blue-100 rounded-lg">
            <ShoppingCart className="text-blue-600" size={20} />
          </div>
        </div>
        <div className="text-2xl font-bold text-gray-800 mb-1">
          {overview.todayOrders}
        </div>
        <div className="text-sm text-gray-500">{getLabel('orders')}</div>
      </div>

      {/* 3. Customers */}
      <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
        <div className="flex items-center justify-between mb-3">
          <div className="p-3 bg-purple-100 rounded-lg">
            <Users className="text-purple-600" size={20} />
          </div>
        </div>
        <div className="text-2xl font-bold text-gray-800 mb-1">
          {overview.todayCustomers}
        </div>
        <div className="text-sm text-gray-500">{getLabel('customers')}</div>
      </div>

      {/* 4. Avg Order Value */}
      <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
        <div className="flex items-center justify-between mb-3">
          <div className="p-3 bg-orange-100 rounded-lg">
            <TrendingUp className="text-orange-600" size={20} />
          </div>
          <span className="text-xs font-medium text-gray-500 bg-gray-50 px-2 py-1 rounded-full">
            Avg
          </span>
        </div>
        <div className="text-2xl font-bold text-gray-800 mb-1">
          {formatCurrency(overview.averageOrderValue)}
        </div>
        <div className="text-sm text-gray-500">{t("statistics.metric.avgOrderValue")}</div>
      </div>

      {/* 5. Cash Revenue */}
      <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
        <div className="flex items-center justify-between mb-3">
          <div className="p-3 bg-emerald-100 rounded-lg">
            <Banknote className="text-emerald-600" size={20} />
          </div>
        </div>
        <div className="text-2xl font-bold text-gray-800 mb-1">
          {formatCurrency(overview.cashRevenue)}
        </div>
        <div className="text-sm text-gray-500">{t("statistics.metric.cashRevenue")}</div>
      </div>

      {/* 6. Card Revenue */}
      <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
        <div className="flex items-center justify-between mb-3">
          <div className="p-3 bg-indigo-100 rounded-lg">
            <CreditCard className="text-indigo-600" size={20} />
          </div>
        </div>
        <div className="text-2xl font-bold text-gray-800 mb-1">
          {formatCurrency(overview.cardRevenue)}
        </div>
        <div className="text-sm text-gray-500">{t("statistics.metric.cardRevenue")}</div>
      </div>

      {/* 7. Other Revenue (Conditional) */}
      {(overview.otherRevenue !== undefined && overview.otherRevenue > 0) && (
        <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
          <div className="flex items-center justify-between mb-3">
            <div className="p-3 bg-gray-100 rounded-lg">
              <Banknote className="text-gray-600" size={20} />
            </div>
          </div>
          <div className="text-2xl font-bold text-gray-800 mb-1">
            {formatCurrency(overview.otherRevenue)}
          </div>
          <div className="text-sm text-gray-500">{t("statistics.metric.otherRevenue")}</div>
        </div>
      )}

      {/* 8. Avg Guest Spend */}
      <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
        <div className="flex items-center justify-between mb-3">
          <div className="p-3 bg-cyan-100 rounded-lg">
            <UserCheck className="text-cyan-600" size={20} />
          </div>
        </div>
        <div className="text-2xl font-bold text-gray-800 mb-1">
          {formatCurrency(overview.avgGuestSpend)}
        </div>
        <div className="text-sm text-gray-500">{t("statistics.metric.avgGuestSpend")}</div>
      </div>

      {/* 9. Avg Dining Time */}
      <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
        <div className="flex items-center justify-between mb-3">
          <div className="p-3 bg-teal-100 rounded-lg">
            <Clock className="text-teal-600" size={20} />
          </div>
        </div>
        <div className="text-2xl font-bold text-gray-800 mb-1">
          {Math.round(overview.avgDiningTime)} min
        </div>
        <div className="text-sm text-gray-500">{t("statistics.metric.avgDiningTime")}</div>
      </div>

      {/* 10. Voided Orders */}
      <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
        <div className="flex items-center justify-between mb-3">
          <div className="p-3 bg-red-100 rounded-lg">
            <Ban className="text-red-600" size={20} />
          </div>
        </div>
        <div className="text-2xl font-bold text-gray-800 mb-1">
          {overview.voidedOrders}
        </div>
        <div className="text-sm text-gray-500">
             {t("statistics.metric.voidedOrders")} ({formatCurrency(overview.voidedAmount)})
        </div>
      </div>

      {/* 11. Total Discount */}
      <div className="bg-white rounded-xl p-4 shadow-sm border border-gray-100 hover:shadow-md transition-shadow">
        <div className="flex items-center justify-between mb-3">
          <div className="p-3 bg-yellow-100 rounded-lg">
            <Tag className="text-yellow-600" size={20} />
          </div>
        </div>
        <div className="text-2xl font-bold text-gray-800 mb-1">
          {formatCurrency(overview.totalDiscount)}
        </div>
        <div className="text-sm text-gray-500">{t("statistics.metric.totalDiscount")}</div>
      </div>
    </div>
  );
};
