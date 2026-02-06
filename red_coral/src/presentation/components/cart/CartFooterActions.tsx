import React from 'react';
import { List, RotateCcw, Bookmark, Save, Trash2 } from 'lucide-react';
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
    <div className="flex text-sm font-medium text-gray-600 h-14 border-t border-gray-200 divide-x divide-gray-300 bg-white">
      {isCartEmpty ? (
        <>
          <button
            onClick={onManageTable}
            className="flex-1 hover:bg-gray-50 active:bg-gray-100 active:scale-[0.98] transition-all flex flex-col items-center justify-center gap-1 pt-1"
          >
            <List size={20} className="text-gray-500" />
            <span className="text-sm">{t('pos.sidebar.get_order')} ({heldOrdersCount})</span>
          </button>
          <button
            onClick={onRestoreDraft}
            disabled={draftOrdersCount === 0}
            className={`flex-1 hover:bg-gray-50 active:bg-gray-100 active:scale-[0.98] transition-all flex flex-col items-center justify-center gap-1 pt-1 ${
              draftOrdersCount === 0 ? 'opacity-40 cursor-not-allowed' : 'text-blue-600'
            }`}
          >
            <RotateCcw size={20} />
            <span className="text-sm">{draftOrdersCount > 0 ? `${t('pos.sidebar.drafts')} (${draftOrdersCount})` : t('pos.sidebar.no_drafts')}</span>
          </button>
        </>
      ) : (
        <>
          <button
            onClick={onManageTable}
            className="flex-1 hover:bg-gray-50 active:bg-orange-100 active:scale-[0.98] transition-all flex flex-col items-center justify-center gap-1 pt-1 text-orange-600"
          >
            <Bookmark size={20} />
            <span className="text-sm">{t('pos.sidebar.hold')}</span>
          </button>
          <button
            onClick={onSaveDraft}
            className="flex-1 hover:bg-gray-50 active:bg-blue-100 active:scale-[0.98] transition-all flex flex-col items-center justify-center gap-1 pt-1 text-blue-600"
          >
            <Save size={20} />
            <span className="text-sm">{t('pos.sidebar.save')}</span>
          </button>
          <button
            onClick={onClear}
            className="flex-1 hover:bg-red-50 active:bg-red-100 active:scale-[0.98] transition-all flex flex-col items-center justify-center gap-1 pt-1 text-red-500"
          >
            <Trash2 size={20} />
            <span className="text-sm">{t('pos.sidebar.clear')}</span>
          </button>
        </>
      )}
    </div>
  );
});

CartFooterActions.displayName = 'CartFooterActions';
