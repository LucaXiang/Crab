import React from 'react';
import {
  ArrowLeft,
  Activity,
  TrendingUp,
  BarChart as BarChartIcon,
  ShoppingCart,
  PieChart as PieChartIcon,
  Calendar,
  FileText,
  ShieldCheck
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { ActiveTab } from '@/core/domain/types';

import { TimeRange } from '@/core/domain/types';

interface SidebarProps {
  onBack: () => void;
  activeTab: ActiveTab;
  onTabChange: (tab: ActiveTab) => void;
  timeRange: TimeRange;
  customStartDate?: string;
  customEndDate?: string;
}

export const Sidebar: React.FC<SidebarProps> = ({ 
  onBack, 
  activeTab, 
  onTabChange,
  timeRange,
  customStartDate,
  customEndDate
}) => {
  const { t, locale } = useI18n();

  const getDateRangeLabel = () => {
    const today = new Date();
    const formatDate = (date: Date) => {
      return date.toLocaleDateString(locale, { month: '2-digit', day: '2-digit' });
    };

    if (timeRange === 'custom' && customStartDate && customEndDate) {
      return `${customStartDate} - ${customEndDate}`;
    }
    
    if (timeRange === 'today') {
      return `${t('statistics.time.today')}, ${formatDate(today)}`;
    }

    if (timeRange === 'week') {
      const day = today.getDay();
      const diff = today.getDate() - day + (day === 0 ? -6 : 1);
      const monday = new Date(today.setDate(diff));
      // Reset today for end date (since setDate modifies it)
      const endDate = new Date(); 
      return `${formatDate(monday)} - ${formatDate(endDate)}`;
    }

    if (timeRange === 'month') {
      const firstDay = new Date(today.getFullYear(), today.getMonth(), 1);
      return `${formatDate(firstDay)} - ${formatDate(today)}`;
    }

    return t(`statistics.time.${timeRange}`);
  };

  const menuItems = [
    { id: 'overview' as const, icon: TrendingUp, label: t('statistics.sidebar.overview') },
    { id: 'sales' as const, icon: BarChartIcon, label: t('statistics.report.sales') },
    { id: 'daily_report' as const, icon: FileText, label: t('statistics.sidebar.daily_report') },
    { id: 'products' as const, icon: ShoppingCart, label: t('statistics.report.product') },
    { id: 'categories' as const, icon: PieChartIcon, label: t('statistics.report.category') },
    { id: 'audit_log' as const, icon: ShieldCheck, label: t('statistics.sidebar.audit_log') },
  ];

  return (
    <div className="w-72 bg-white border-r border-gray-200 flex flex-col shrink-0">
      <div className="p-6 border-b border-gray-100 shrink-0">
        <div className="flex items-center gap-3 mb-6">
          <button
            onClick={onBack}
            className="p-2 -ml-2 hover:bg-gray-100 rounded-full text-gray-600 transition-colors"
          >
            <ArrowLeft size={20} />
          </button>
          <h2 className="text-xl font-bold text-gray-800 flex items-center gap-2">
            <Activity className="text-blue-600" size={24} />
            <span>{t('statistics.sidebar.title')}</span>
          </h2>
        </div>
        <div className="text-xs text-gray-400 font-medium uppercase tracking-wider mb-2">
          {t('statistics.sidebar.analytics')}
        </div>
      </div>

      {/* Stats Menu */}
      <div className="flex-1 overflow-y-auto p-4">
        <div className="space-y-1">
          {menuItems.map((item) => (
            <button 
              key={item.id}
              onClick={() => onTabChange(item.id)}
              className={`w-full text-left px-4 py-3 rounded-lg font-medium transition-all flex items-center gap-3 ${
                activeTab === item.id 
                  ? 'bg-blue-50 text-blue-600' 
                  : 'text-gray-600 hover:bg-gray-50'
              }`}
            >
              <item.icon size={18} />
              {item.label}
            </button>
          ))}
        </div>
      </div>
      
      <div className="p-4 border-t border-gray-100">
         <div className="bg-gray-50 rounded-lg p-4">
           <div className="flex items-center gap-2 text-gray-600 mb-2">
            <Calendar size={16} />
            <span className="text-sm font-medium">{t('statistics.date_range')}</span>
          </div>
          <div className="text-sm text-gray-500" suppressHydrationWarning>
            {getDateRangeLabel()}
          </div>
        </div>
      </div>
    </div>
  );
};
