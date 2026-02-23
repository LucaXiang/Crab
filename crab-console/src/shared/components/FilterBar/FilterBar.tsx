import React from 'react';
import { Search, Filter } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

interface FilterBarProps {
  searchQuery: string;
  onSearchChange: (value: string) => void;
  searchPlaceholder?: string;
  totalCount: number;
  countUnit: string;
  themeColor?: 'blue' | 'purple' | 'orange' | 'teal' | 'indigo';
  children?: React.ReactNode;
}

const dotColorClasses = {
  blue: 'bg-blue-500',
  purple: 'bg-purple-500',
  orange: 'bg-orange-500',
  teal: 'bg-teal-500',
  indigo: 'bg-indigo-500'
};

const focusColorClasses = {
  blue: 'focus:ring-blue-500/20 focus:border-blue-500',
  purple: 'focus:ring-purple-500/20 focus:border-purple-500',
  orange: 'focus:ring-orange-500/20 focus:border-orange-500',
  teal: 'focus:ring-teal-500/20 focus:border-teal-500',
  indigo: 'focus:ring-indigo-500/20 focus:border-indigo-500'
};

export const FilterBar: React.FC<FilterBarProps> = ({
  searchQuery,
  onSearchChange,
  searchPlaceholder,
  totalCount,
  countUnit,
  themeColor = 'blue',
  children
}) => {
  const { t } = useI18n();
  const placeholder = searchPlaceholder || t('common.hint.search_placeholder');

  return (
    <div className="bg-white rounded-xl border border-gray-200 p-3 md:p-4 shadow-sm">
      <div className="flex flex-col sm:flex-row sm:items-center gap-3">
        {/* Filter icon + label — hidden on mobile */}
        <div className="hidden sm:flex items-center gap-2 text-gray-500 shrink-0">
          <Filter size={16} />
          <span className="text-sm font-medium">{t('common.action.filter')}</span>
        </div>
        <div className="hidden sm:block h-5 w-px bg-gray-200" />

        {/* Search input — full width on mobile */}
        <div className="relative flex-1 sm:max-w-xs">
          <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            placeholder={placeholder}
            className={`w-full pl-9 pr-3 py-2 sm:py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 ${focusColorClasses[themeColor]}`}
          />
        </div>

        {children}

        {/* Count badge */}
        <div className="flex items-center gap-2 sm:ml-auto">
          <div className={`w-2 h-2 rounded-full ${dotColorClasses[themeColor]}`} />
          <span className="text-sm text-gray-600">{t('common.label.total')}</span>
          <span className="text-sm font-bold text-gray-900">{totalCount}</span>
          <span className="text-sm text-gray-600">{countUnit}</span>
        </div>
      </div>
    </div>
  );
};
