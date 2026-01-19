import React from 'react';
import { ArrowLeft, Settings } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { Table } from '@/core/domain/types';

interface GuestInputHeaderProps {
  selectedTable: Table;
  isOccupied: boolean;
  onBack: () => void;
  onManage?: () => void;
}

export const GuestInputHeader: React.FC<GuestInputHeaderProps> = ({
  selectedTable,
  isOccupied,
  onBack,
  onManage,
}) => {
  const { t } = useI18n();

  return (
    <div className="p-3 bg-gray-50 border-b border-gray-100 flex justify-between items-center shrink-0">
      <div>
        <h3 className="font-bold text-gray-800 flex items-center gap-2 text-sm">
          {isOccupied ? t('table.addItems') : t('table.openTable')}
          <span className="bg-gray-200 px-2 rounded text-xs text-gray-600">
            {selectedTable.name}
          </span>
        </h3>
      </div>
      <div className="flex items-center gap-2">
        {isOccupied && onManage && (
          <button
            onClick={onManage}
            className="text-gray-600 hover:text-gray-800 flex items-center gap-1 hover:bg-gray-200 px-2 py-1 rounded transition-colors"
          >
            <Settings size={14} />
            <span className="text-xs font-bold">{t('table.action.manage')}</span>
          </button>
        )}
        <button
          onClick={onBack}
          className="text-gray-400 hover:text-gray-600 flex items-center gap-1 hover:bg-gray-100 px-2 py-1 rounded"
        >
          <ArrowLeft size={14} />
          <span className="text-xs font-bold">{t('table.selection.back')}</span>
        </button>
      </div>
    </div>
  );
};
