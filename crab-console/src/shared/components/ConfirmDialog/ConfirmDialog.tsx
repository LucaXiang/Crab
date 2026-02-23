import React from 'react';
import { AlertTriangle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

interface ConfirmDialogProps {
  isOpen: boolean;
  title: string;
  description: string;
  confirmText?: string;
  cancelText?: string;
  onConfirm: () => void;
  onCancel: () => void;
  variant?: 'danger' | 'warning' | 'info';
  showCancel?: boolean;
}

export const ConfirmDialog: React.FC<ConfirmDialogProps> = ({
  isOpen,
  title,
  description,
  confirmText,
  cancelText,
  onConfirm,
  onCancel,
  variant = 'danger',
  showCancel = true,
}) => {
  const { t } = useI18n();

  if (!isOpen) return null;

  const handleBackdropClick = (e: React.MouseEvent<HTMLDivElement>) => {
    // Only close on cancel if clicking the backdrop itself, not its children
    if (e.target === e.currentTarget) {
      onCancel();
    }
  };

  return (
    <div
      className="fixed inset-0 z-[60] flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm animate-in fade-in duration-200"
      onClick={handleBackdropClick}
    >
      <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-sm overflow-hidden animate-in zoom-in-95 duration-200 border border-gray-100">
        <div className="p-6 text-center">
          <div className={`mx-auto w-14 h-14 rounded-full flex items-center justify-center mb-5 ${
            variant === 'danger' ? 'bg-red-50 text-red-500' :
            variant === 'warning' ? 'bg-amber-50 text-amber-500' :
            'bg-blue-50 text-blue-500'
          }`}>
            <AlertTriangle size={28} />
          </div>
          <h3 className="text-xl font-bold text-gray-900 mb-3">{title}</h3>
          <p className="text-gray-600 mb-8 leading-relaxed text-sm whitespace-pre-line">{description}</p>

          <div className={`grid gap-3 ${showCancel ? 'grid-cols-2' : 'grid-cols-1'}`}>
            {showCancel && (
              <button
                onClick={onCancel}
                className="w-full py-3 bg-gray-100 text-gray-700 rounded-xl text-sm font-bold hover:bg-gray-200 transition-colors active:scale-95 transform"
              >
                {cancelText || t('common.action.cancel')}
              </button>
            )}
            <button
              onClick={onConfirm}
              className={`w-full py-3 text-white rounded-xl text-sm font-bold shadow-lg transition-all active:scale-95 transform ${
                variant === 'danger'
                  ? 'bg-red-500 hover:bg-red-600 shadow-red-500/20'
                  : variant === 'warning'
                    ? 'bg-amber-500 hover:bg-amber-600 shadow-amber-500/20'
                    : 'bg-blue-500 hover:bg-blue-600 shadow-blue-500/20'
              }`}
            >
              {confirmText || t('common.action.confirm')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
