import React from 'react';
import { Utensils, ClipboardList, Archive, Settings as SettingsIcon, LogOut, ChartArea } from 'lucide-react';
import { IconBtn } from '@/presentation/components/ui/IconBtn';
import { useI18n } from '@/hooks/useI18n';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { Permission } from '@/core/domain/types';

interface ActionBarProps {
  screen: 'POS' | 'HISTORY' | 'SETTINGS' | 'STATISTICS';
  isDbOnline: boolean | null;
  onSetScreen: (screen: 'POS' | 'HISTORY' | 'SETTINGS' | 'STATISTICS') => void;
  onOpenCashDrawer: () => void;
  onRequestExit: () => void;
}

export const ActionBar: React.FC<ActionBarProps> = ({
  screen,
  isDbOnline,
  onSetScreen,
  onOpenCashDrawer,
  onRequestExit,
}) => {
  const { t } = useI18n();

  return (
    <div className="h-16 bg-primary-500 flex items-center justify-between shrink-0 px-3 text-white shadow-md z-20">
      <div className="flex items-center gap-2">
        <button
          onClick={() => onSetScreen('POS')}
          onMouseDown={(e) => e.preventDefault()}
          className={`p-3 rounded-xl transition-all flex items-center gap-3 ${
            screen === 'POS'
              ? 'bg-white text-primary-500 font-bold shadow-sm'
              : 'text-white/80 hover:bg-white/10'
          }`}
        >
          <Utensils size={24} />
          <span
            className={`inline-block w-2.5 h-2.5 rounded-full ${
              isDbOnline === null
                ? 'bg-white/50'
                : isDbOnline
                ? 'bg-green-500'
                : 'bg-red-500'
            }`}
          />
        </button>
        <button
          onClick={() => onSetScreen('HISTORY')}
          onMouseDown={(e) => e.preventDefault()}
          className={`p-3 rounded-xl transition-all flex items-center gap-3 ${
            screen === 'HISTORY'
              ? 'bg-white text-primary-500 font-bold shadow-sm'
              : 'text-white/80 hover:bg-white/10'
          }`}
        >
          <ClipboardList size={24} />
        </button>
      </div>

      <div className="flex items-center gap-2">
        <IconBtn icon={SettingsIcon} size={24} className="p-3 hover:bg-white/10 rounded-xl" onClick={() => onSetScreen('SETTINGS')} onMouseDown={(e) => e.preventDefault()} />
        <ProtectedGate permission={Permission.VIEW_STATISTICS}>
          <IconBtn icon={ChartArea} size={24} className="p-3 hover:bg-white/10 rounded-xl" onClick={() => onSetScreen('STATISTICS')} onMouseDown={(e) => e.preventDefault()} />
        </ProtectedGate>
        <EscalatableGate 
          permission={Permission.OPEN_CASH_DRAWER}
          mode="intercept"
          description={t('app.action.open_cash_drawer')}
          onAuthorized={onOpenCashDrawer}
        >
          <IconBtn icon={Archive} size={24} className="p-3 hover:bg-white/10 rounded-xl" onClick={onOpenCashDrawer} onMouseDown={(e) => e.preventDefault()} />
        </EscalatableGate>
        <IconBtn icon={LogOut} size={24} className="p-3 hover:bg-white/10 rounded-xl" onClick={onRequestExit} onMouseDown={(e) => e.preventDefault()} />
      </div>
    </div>
  );
};
