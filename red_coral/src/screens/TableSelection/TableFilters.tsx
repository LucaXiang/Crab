import React from 'react';
import { LayoutGrid, CheckCircle, Utensils, Receipt } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { TableFilter } from './types';

interface FilterButtonProps {
  type: TableFilter;
  label: string;
  icon: React.FC<{ size?: number }>;
  colorClass: string;
  count: number;
  isActive: boolean;
  onClick: () => void;
}

const FilterButton: React.FC<FilterButtonProps> = React.memo(
  ({ label, icon: Icon, colorClass, count, isActive, onClick }) => {
    return (
      <button
        onClick={onClick}
        className={`
          flex items-center gap-1.5 px-3 py-1.5 rounded-lg text-xs font-bold transition-all border
          ${
            isActive
              ? `${colorClass} ring-1 ring-offset-1 ring-gray-200`
              : 'bg-white text-gray-500 border-gray-200 hover:bg-gray-50'
          }
        `}
      >
        <Icon size={14} />
        <span>{label}</span>
        <span
          className={`ml-1 px-1.5 py-0.5 rounded-full text-[0.625rem] ${
            isActive ? 'bg-white/20' : 'bg-gray-100 text-gray-600'
          }`}
        >
          {count}
        </span>
      </button>
    );
  }
);

interface TableFiltersProps {
  activeFilter: TableFilter;
  onFilterChange: (filter: TableFilter) => void;
  stats: {
    ALL: number;
    EMPTY: number;
    OCCUPIED: number;
    OVERTIME: number;
    PRE_PAYMENT: number;
  };
}

export const TableFilters: React.FC<TableFiltersProps> = React.memo(
  ({ activeFilter, onFilterChange, stats }) => {
    const { t } = useI18n();

    return (
      <div className="px-3 pt-3 pb-2 flex flex-wrap gap-2 items-center">
        <FilterButton
          type="ALL"
          label={t('table.filter.all')}
          icon={LayoutGrid}
          colorClass="bg-[#FF5E5E] text-white border-[#FF5E5E]"
          count={stats.ALL}
          isActive={activeFilter === 'ALL'}
          onClick={() => onFilterChange('ALL')}
        />
        <FilterButton
          type="EMPTY"
          label={t('table.filter.empty')}
          icon={CheckCircle}
          colorClass="bg-green-500 text-white border-green-500"
          count={stats.EMPTY}
          isActive={activeFilter === 'EMPTY'}
          onClick={() => onFilterChange('EMPTY')}
        />
        <FilterButton
          type="OCCUPIED"
          label={t('table.filter.occupied')}
          icon={Utensils}
          colorClass="bg-blue-500 text-white border-blue-500"
          count={stats.OCCUPIED}
          isActive={activeFilter === 'OCCUPIED'}
          onClick={() => onFilterChange('OCCUPIED')}
        />
        <FilterButton
          type="PRE_PAYMENT"
          label={t("table.filter.pre_payment")}
          icon={Receipt}
          colorClass="bg-purple-500 text-white border-purple-500"
          count={stats.PRE_PAYMENT}
          isActive={activeFilter === 'PRE_PAYMENT'}
          onClick={() => onFilterChange('PRE_PAYMENT')}
        />
      </div>
    );
  }
);
