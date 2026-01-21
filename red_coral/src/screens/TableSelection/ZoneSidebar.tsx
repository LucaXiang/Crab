import React from 'react';
import { Zone } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';

interface ZoneSidebarProps {
  zones: Zone[];
  activeZoneId: string;
  onZoneSelect: (zoneId: string) => void;
}

export const ZoneSidebar: React.FC<ZoneSidebarProps> = React.memo(
  ({ zones, activeZoneId, onZoneSelect }) => {
    const { t } = useI18n();

    return (
      <div className="w-40 bg-white border-r border-gray-200 flex flex-col overflow-y-auto shrink-0">
        <div className="p-2">
          <h2 className="text-[10px] font-bold text-gray-400 uppercase tracking-wider mb-2 px-2">
            {t('table.zones')}
          </h2>
          <div className="space-y-1">
            <button
              onClick={() => onZoneSelect('ALL')}
              className={`
                w-full p-2.5 rounded-lg flex items-center gap-2 transition-all text-left
                ${
                  activeZoneId === 'ALL'
                    ? 'bg-[#FF5E5E] text-white shadow-md font-bold'
                    : 'bg-gray-50 text-gray-600 hover:bg-gray-100'
                }
              `}
            >
              <span className="text-xs">{t('common.status.all')}</span>
            </button>
            {zones.length === 0 ? (
              <div className="text-center text-gray-400 text-xs py-4">
                {t('common.label.none')}
              </div>
            ) : (
              zones.map((zone) => (
                <button
                  key={zone.id}
                  onClick={() => onZoneSelect(zone.id)}
                  className={`
                    w-full p-2.5 rounded-lg flex items-center justify-between gap-2 transition-all text-left
                    ${
                      activeZoneId === zone.id
                        ? 'bg-[#FF5E5E] text-white shadow-md font-bold'
                        : 'bg-gray-50 text-gray-600 hover:bg-gray-100'
                    }
                  `}
                >
                  <span className="text-xs truncate">{zone.name}</span>
                </button>
              ))
            )}
          </div>
        </div>
      </div>
    );
  }
);
