import React from 'react';
import {
  ArrowLeft,
  Activity,
  TrendingUp,
  FileText,
  ClipboardList,
  ShieldCheck
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { ActiveTab } from '@/core/domain/types';

interface SidebarProps {
  onBack: () => void;
  activeTab: ActiveTab;
  onTabChange: (tab: ActiveTab) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({ onBack, activeTab, onTabChange }) => {
  const { t } = useI18n();

  const menuItems = [
    { id: 'overview' as const, icon: TrendingUp, label: t('statistics.sidebar.overview') },
    { id: 'invoices' as const, icon: FileText, label: t('statistics.sidebar.invoices') },
    { id: 'reports_shifts' as const, icon: ClipboardList, label: t('statistics.sidebar.reports_shifts') },
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
    </div>
  );
};
