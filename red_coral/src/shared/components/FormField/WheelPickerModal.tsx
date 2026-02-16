import React from 'react';
import { X, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

interface WheelPickerModalProps {
  title: string;
  icon: React.ReactNode;
  onClose: () => void;
  onConfirm: () => void;
  onClear: () => void;
  children: React.ReactNode;
  preview: React.ReactNode;
}

export const WheelPickerModal: React.FC<WheelPickerModalProps> = ({
  title,
  icon,
  onClose,
  onConfirm,
  onClear,
  children,
  preview,
}) => {
  const { t } = useI18n();

  return (
    <div className="fixed inset-0 z-[100] bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-sm overflow-hidden animate-in zoom-in-95 duration-200">
        <div className="px-5 py-4 border-b border-gray-100 flex items-center justify-between">
          <h3 className="text-base font-bold text-gray-900 flex items-center gap-2">
            {icon}
            {title}
          </h3>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-xl transition-colors">
            <X size={16} className="text-gray-500" />
          </button>
        </div>

        <div className="px-5 pt-4 text-center">
          {preview}
        </div>

        {children}

        <div className="px-5 py-4 border-t border-gray-100 flex gap-3">
          <button
            onClick={onClear}
            className="flex-1 py-3 bg-gray-100 text-gray-600 rounded-xl text-sm font-semibold hover:bg-gray-200 active:scale-[0.98] transition-all"
          >
            {t('common.action.clear')}
          </button>
          <button
            onClick={onConfirm}
            className="flex-1 py-3 bg-teal-600 text-white rounded-xl text-sm font-semibold hover:bg-teal-700 active:scale-[0.98] transition-all shadow-lg shadow-teal-600/20 flex items-center justify-center gap-1.5"
          >
            <Check size={16} />
            {t('common.action.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
};
