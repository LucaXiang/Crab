import React from 'react';
import { X, Trash2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

interface DetailPanelProps {
  title: string;
  isCreating: boolean;
  onClose: () => void;
  onSave: () => void;
  onDelete?: () => void;
  saving?: boolean;
  saveDisabled?: boolean;
  saveLabel?: string;
  deleteLabel?: string;
  children: React.ReactNode;
}

export const DetailPanel: React.FC<DetailPanelProps> = ({
  title, isCreating, onClose, onSave, onDelete,
  saving, saveDisabled, saveLabel, deleteLabel, children,
}) => {
  const { t } = useI18n();

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between px-5 lg:px-6 py-4 border-b border-gray-200">
        <h2 className="text-lg font-bold text-slate-900">{title}</h2>
        <button
          onClick={onClose}
          className="hidden lg:flex p-1.5 rounded-lg hover:bg-gray-100 transition-colors"
        >
          <X className="w-4 h-4 text-gray-400" />
        </button>
      </div>

      {/* Form content */}
      <div className="flex-1 overflow-y-auto px-5 lg:px-6 py-5 space-y-5">
        {children}
      </div>

      {/* Footer buttons — 手机端 sticky bottom */}
      <div className="flex items-center gap-3 px-5 lg:px-6 py-4 border-t border-gray-100 bg-white sticky bottom-0">
        {!isCreating && onDelete && (
          <button
            onClick={onDelete}
            className="flex items-center gap-1.5 px-3 py-2.5 text-sm font-medium text-red-600 border border-red-200 rounded-lg hover:bg-red-50 transition-colors"
          >
            <Trash2 className="w-4 h-4" />
            {deleteLabel || t('common.action.delete')}
          </button>
        )}
        <div className="flex-1" />
        <button
          onClick={onClose}
          className="px-4 py-2.5 text-sm text-slate-500 hover:bg-slate-100 rounded-lg transition-colors"
        >
          {t('common.action.cancel')}
        </button>
        <button
          onClick={onSave}
          disabled={saving || saveDisabled}
          className="px-5 py-2.5 text-sm font-medium text-white bg-primary-500 hover:bg-primary-600 rounded-lg transition-colors disabled:opacity-50"
        >
          {saving ? t('catalog.saving') : (saveLabel || (isCreating ? t('common.action.create') : t('common.action.save')))}
        </button>
      </div>
    </div>
  );
};
