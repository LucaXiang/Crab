import React from 'react';
import { CheckCircle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

interface ConfirmButtonProps {
  isOccupied: boolean;
  guestInput: string;
  onConfirm: () => void;
}

export const ConfirmButton: React.FC<ConfirmButtonProps> = ({
  isOccupied,
  guestInput,
  onConfirm,
}) => {
  const { t } = useI18n();

  return (
    <div className="p-4 border-t border-gray-100 bg-white shrink-0 z-20 shadow-[0_-5px_15px_rgba(0,0,0,0.05)]">
      <button
        onClick={onConfirm}
        disabled={!isOccupied && (!guestInput || parseInt(guestInput) === 0)}
        className="w-full h-11 rounded-xl bg-[#FF5E5E] text-white text-base font-bold flex items-center justify-center gap-2 hover:bg-red-600 disabled:bg-gray-300 disabled:cursor-not-allowed shadow-lg shadow-red-200 transition-all active:scale-[0.98]"
      >
        <CheckCircle size={18} />
        {isOccupied ? t('table.confirm_add') : t('table.confirm_open')}
      </button>
    </div>
  );
};
