import React from 'react';
import { useI18n } from '@/hooks/useI18n';

interface StatusToggleProps {
  isActive: boolean;
  onClick: (e: React.MouseEvent) => void;
  disabled?: boolean;
}

export const StatusToggle: React.FC<StatusToggleProps> = ({ isActive, onClick, disabled = false }) => {
  const { t } = useI18n();

  return (
    <button
      onClick={(ev) => { ev.stopPropagation(); if (!disabled) onClick(ev); }}
      disabled={disabled}
      className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium transition-colors ${
        isActive ? 'bg-green-50 text-green-700 hover:bg-green-100' : 'bg-gray-100 text-gray-500 hover:bg-gray-200'
      } ${disabled ? 'cursor-not-allowed opacity-60' : 'cursor-pointer'}`}
    >
      {isActive ? t('settings.common.active') : t('settings.common.inactive')}
    </button>
  );
};
