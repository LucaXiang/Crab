import React from 'react';
import { List, RotateCcw, PauseCircle, Save, Trash2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

interface CartFooterActionsProps {
  isCartEmpty: boolean;
  heldOrdersCount: number;
  draftOrdersCount: number;
  onManageTable: () => void;
  onSaveDraft: () => void;
  onRestoreDraft: () => void;
  onClear: () => void;
}

export const CartFooterActions = React.memo<CartFooterActionsProps>(({
  isCartEmpty,
  heldOrdersCount,
  draftOrdersCount,
  onManageTable,
  onSaveDraft,
  onRestoreDraft,
  onClear
}) => {
  const { t } = useI18n();

  return (
    <div className="flex text-sm font-medium text-gray-600 h-12 border-t border-gray-200 divide-x divide-gray-300 bg-white">
      {isCartEmpty ? (
        <>
          <button
            onClick={onManageTable}
            className="flex-1 hover:bg-gray-50 transition-colors flex flex-col items-center justify-center gap-0.5 pt-1"
          >
            <List size={18} className="text-gray-500" />
            <span className="text-xs">{t('pos.sidebar.get_order')} ({heldOrdersCount})</span>
          </button>
          <button
            onClick={onRestoreDraft}
            disabled={draftOrdersCount === 0}
            className={`flex-1 hover:bg-gray-50 transition-colors flex flex-col items-center justify-center gap-0.5 pt-1 ${
              draftOrdersCount === 0 ? 'opacity-40 cursor-not-allowed' : 'text-blue-600'
            }`}
          >
            <RotateCcw size={18} />
            <span className="text-xs">{draftOrdersCount > 0 ? `${t('pos.sidebar.drafts')} (${draftOrdersCount})` : t('pos.sidebar.no_drafts')}</span>
          </button>
        </>
      ) : (
        <>
          <button
            onClick={onManageTable}
            className="flex-1 hover:bg-gray-50 transition-colors flex flex-col items-center justify-center gap-0.5 pt-1 text-orange-600"
          >
            <PauseCircle size={18} />
            <span className="text-xs">{t('pos.sidebar.hold')}</span>
          </button>
          <button
            onClick={onSaveDraft}
            className="flex-1 hover:bg-gray-50 transition-colors flex flex-col items-center justify-center gap-0.5 pt-1 text-blue-600"
          >
            <Save size={18} />
            <span className="text-xs">{t('pos.sidebar.save')}</span>
          </button>
          <button
            onClick={onClear}
            className="flex-1 hover:bg-red-50 transition-colors flex flex-col items-center justify-center gap-0.5 pt-1 text-red-500"
          >
            <Trash2 size={18} />
            <span className="text-xs">{t('pos.sidebar.clear')}</span>
          </button>
        </>
      )}
    </div>
  );
});

CartFooterActions.displayName = 'CartFooterActions';
