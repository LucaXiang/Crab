import React from 'react';
import { DraftOrder } from '@/core/domain/types';
import { Permission as PermissionValues } from '@/core/domain/types';
import { X, Clock, RotateCcw, Trash2, ClipboardList } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { EscalatableGate } from './auth/EscalatableGate';
import { formatCurrency } from '@/utils/formatCurrency';

interface DraftListModalProps {
  draftOrders: DraftOrder[];
  onClose: () => void;
  onRestore: (id: string) => void;
  onDelete: (id: string) => void;
}

export const DraftListModal = React.memo<DraftListModalProps>(({
  draftOrders,
  onClose,
  onRestore,
  onDelete
}) => {
  const { t } = useI18n();

  return (
    <div className="fixed inset-0 z-60 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl overflow-hidden flex flex-col h-[600px] animate-in zoom-in-95 duration-200">
        <div className="p-4 border-b border-gray-100 flex justify-between items-center bg-gray-50">
          <h2 className="text-xl font-bold text-gray-800 flex items-center gap-2">
            <ClipboardList className="text-blue-500" />
            {t('draft.list.title')}
          </h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-200 rounded-full transition-colors">
            <X size={20} className="text-gray-500" />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-4 space-y-3">
          {draftOrders.length === 0 ? (
            <div className="text-center text-gray-400 py-8">{t('draft.list.empty')}</div>
          ) : (
            draftOrders.map(draft => (
              <div
                key={draft.order_id}
                className="bg-white border border-gray-200 rounded-xl p-4 shadow-sm flex items-center justify-between hover:shadow-md transition-shadow"
              >
                <div className="flex-1">
                  <div className="flex items-center gap-2 mb-1">
                    <span className="font-bold text-gray-800 text-lg">
                      {draft.items.reduce((acc, item) => acc + item.quantity, 0)} {t('common.label.quantity')}
                    </span>
                    <span className="text-gray-400 text-sm flex items-center gap-1">
                      <Clock size={14} /> {new Date(draft.created_at).toLocaleString()}
                    </span>
                  </div>
                  <div className="text-gray-500 text-sm line-clamp-1">
                    {draft.items.map(item => item.name).join(', ')}
                  </div>
                </div>

                <div className="flex items-center gap-3">
                  <span className="font-bold text-gray-900 text-lg">
                    {formatCurrency(draft.total)}
                  </span>
                  <EscalatableGate
                    permission={PermissionValues.RESTORE_ORDER}
                    mode="intercept"
                    description={t('draft.action.restore')}
                    onAuthorized={() => draft.order_id && onRestore(draft.order_id)}
                  >
                    <button
                      onClick={() => draft.order_id && onRestore(draft.order_id)}
                      className="px-4 py-2 bg-blue-50 text-blue-600 rounded-lg font-bold hover:bg-blue-100 transition-colors flex items-center gap-1 text-sm"
                    >
                      <RotateCcw size={16} /> {t('draft.action.restore')}
                    </button>
                  </EscalatableGate>
                  <button
                    onClick={() => draft.order_id && onDelete(draft.order_id)}
                    className="p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-lg transition-colors"
                  >
                    <Trash2 size={18} />
                  </button>
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
});

DraftListModal.displayName = 'DraftListModal';
