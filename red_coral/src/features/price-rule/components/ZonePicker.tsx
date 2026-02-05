import React from 'react';
import { X, Globe, ShoppingCart, Armchair, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useZoneStore } from '@/features/zone/store';

interface ZonePickerProps {
  isOpen: boolean;
  selectedZone: string;
  onSelect: (zoneScope: string) => void;
  onClose: () => void;
}

interface ZoneOption {
  value: string;
  icon: React.ElementType;
  label: string;
  description: string;
}

export const ZonePicker: React.FC<ZonePickerProps> = ({
  isOpen,
  selectedZone,
  onSelect,
  onClose,
}) => {
  const { t } = useI18n();
  const zones = useZoneStore(state => state.items);

  if (!isOpen) return null;

  // Build zone options
  const builtInOptions: ZoneOption[] = [
    {
      value: 'zone:all',
      icon: Globe,
      label: t('settings.price_rule.zone.all'),
      description: t('settings.price_rule.zone.all_desc'),
    },
    {
      value: 'zone:retail',
      icon: ShoppingCart,
      label: t('settings.price_rule.zone.retail'),
      description: t('settings.price_rule.zone.retail_desc'),
    },
  ];

  // Custom zones from store
  const customZoneOptions: ZoneOption[] = zones
    .filter(z => z.is_active)
    .map(z => ({
      value: `zone:${z.id.replace('zone:', '')}`,
      icon: Armchair,
      label: z.name,
      description: z.description || t('settings.price_rule.zone.custom_desc'),
    }));

  const allOptions = [...builtInOptions, ...customZoneOptions];

  const handleSelect = (value: string) => {
    onSelect(value);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-md max-h-[80vh] overflow-hidden animate-in zoom-in-95">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-200">
          <h3 className="text-lg font-bold text-gray-900">
            {t('settings.price_rule.picker.select_zone')}
          </h3>
          <button
            onClick={onClose}
            className="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Options */}
        <div className="overflow-y-auto max-h-[60vh] divide-y divide-gray-100">
          {allOptions.map(option => {
            const Icon = option.icon;
            const isSelected = selectedZone === option.value;

            return (
              <button
                key={option.value}
                onClick={() => handleSelect(option.value)}
                className={`
                  w-full flex items-center gap-3 px-5 py-3 transition-all
                  ${isSelected
                    ? 'bg-teal-50'
                    : 'hover:bg-gray-50'
                  }
                `}
              >
                <div
                  className={`
                    w-8 h-8 rounded-lg flex items-center justify-center shrink-0
                    ${isSelected ? 'bg-teal-500 text-white' : 'bg-gray-100 text-gray-500'}
                  `}
                >
                  <Icon size={16} />
                </div>
                <div className="flex-1 text-left min-w-0">
                  <div className="font-medium text-gray-900 truncate">{option.label}</div>
                </div>
                {isSelected && (
                  <Check size={18} className="text-teal-500 shrink-0" />
                )}
              </button>
            );
          })}
        </div>

        {/* Footer */}
        <div className="px-5 py-4 border-t border-gray-200">
          <button
            onClick={onClose}
            className="w-full py-3 bg-gray-100 text-gray-700 rounded-xl font-medium hover:bg-gray-200 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
        </div>
      </div>
    </div>
  );
};
