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
    <div className="h-14 bg-[#FF5E5E] flex items-center justify-between shrink-0 px-2 text-white shadow-md z-20">
      <div className="flex items-center space-x-1">
        <button
          onClick={() => onSetScreen('POS')}
          onMouseDown={(e) => e.preventDefault()}
          className={`p-2 rounded-lg transition-all flex items-center gap-2 ${
            screen === 'POS'
              ? 'bg-white text-[#FF5E5E] font-bold shadow-sm'
              : 'text-white/80 hover:bg-white/10'
          }`}
        >
          <Utensils size={20} />
          <span className="text-sm flex items-center gap-2">
            {t('app.nav.pos')}
            <span
              className={`inline-block w-2 h-2 rounded-full ${
                isDbOnline === null
                  ? 'bg-white/50'
                  : isDbOnline
                  ? 'bg-green-500'
                  : 'bg-red-500'
              }`}
            />
          </span>
        </button>
        <button
          onClick={() => onSetScreen('HISTORY')}
          onMouseDown={(e) => e.preventDefault()}
          className={`p-2 rounded-lg transition-all flex items-center gap-2 ${
            screen === 'HISTORY'
              ? 'bg-white text-[#FF5E5E] font-bold shadow-sm'
              : 'text-white/80 hover:bg-white/10'
          }`}
        >
          <ClipboardList size={20} />
          <span className="text-sm">{t('app.nav.history')}</span>
        </button>
      </div>

      <div className="flex items-center space-x-1">
        <IconBtn icon={SettingsIcon} onClick={() => onSetScreen('SETTINGS')} onMouseDown={(e) => e.preventDefault()} />
        <ProtectedGate permission={Permission.VIEW_STATISTICS}>
          <IconBtn icon={ChartArea} onClick={() => onSetScreen('STATISTICS')} onMouseDown={(e) => e.preventDefault()} />
        </ProtectedGate>
        <EscalatableGate 
          permission={Permission.OPEN_CASH_DRAWER}
          mode="intercept"
          description={t('app.action.open_cash_drawer')}
          onAuthorized={onOpenCashDrawer}
        >
          <IconBtn icon={Archive} onClick={onOpenCashDrawer} onMouseDown={(e) => e.preventDefault()} />
        </EscalatableGate>
        <IconBtn icon={LogOut} onClick={onRequestExit} onMouseDown={(e) => e.preventDefault()} />
      </div>
    </div>
  );
};
