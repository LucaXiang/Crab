import React from 'react';
import { Users, ShoppingBag, Utensils } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { CartItem } from '@/core/domain/types';

interface TimelinePreviewProps {
  isOccupied: boolean;
  cart: CartItem[];
  guestInput: string;
}

export const TimelinePreview: React.FC<TimelinePreviewProps> = ({
  isOccupied,
  cart,
  guestInput,
}) => {
  const { t } = useI18n();

  const isAddingItems = cart.length > 0;
  const color = isAddingItems ? '#F97316' : '#3B82F6';
  const title = isAddingItems ? t('table.add_items') : t('table.open_table');
  const Icon = isAddingItems ? ShoppingBag : Utensils;
  const time = new Date().toLocaleTimeString([], {
    hour: '2-digit',
    minute: '2-digit',
    hour12: false,
  });

  const groupedItems = React.useMemo(() => {
    const map = new Map<string, { name: string; quantity: number }>();

    cart.forEach((item) => {
        const key = `${item.id}-${item.name}`;
        if (!map.has(key)) {
            map.set(key, {
                name: item.name,
                quantity: 0
            });
        }
        map.get(key)!.quantity += item.quantity;
    });

    return Array.from(map.values());
  }, [cart]);

  return (
    <div className="flex flex-col h-full bg-gray-50/50 rounded-xl p-6 relative overflow-hidden items-center justify-center">
      <div className="flex flex-col justify-center items-center w-full">
        <div className="relative border-l-2 border-gray-200 pl-6 pb-2 w-full max-w-[17.5rem]">
          {isOccupied && (
            <div className="absolute -left-[5px] -top-8 opacity-40">
              <div className="w-2.5 h-2.5 rounded-full bg-gray-400 mb-2"></div>
            </div>
          )}

          <div
            className="absolute -left-[9px] top-0 w-5 h-5 rounded-full border border-white shadow-sm transition-colors duration-300 flex items-center justify-center text-white"
            style={{ backgroundColor: color }}
          >
            <Icon size={12} strokeWidth={2.5} />
          </div>

          <div className="bg-white p-4 rounded-lg shadow-sm border border-gray-100 -mt-1 relative">
            <div className="flex justify-between items-start mb-2">
              <span
                className="text-sm font-bold text-gray-800"
              >
                {title}
              </span>
              <span className="text-xs font-mono text-gray-400">{time}</span>
            </div>

            <div className="text-xs text-gray-600 max-h-40 overflow-y-auto space-y-1 custom-scrollbar">
              {isAddingItems ? (
                groupedItems.map((item, idx) => (
                  <div key={idx} className="flex justify-between items-center py-1 border-b border-gray-50 last:border-0">
                    <div className="flex items-center gap-1.5 min-w-0 pr-2">
                        {/* User requested to use InstanceID instead of ExternalID
                        {item.external_id && (
                            <span className="text-[10px] text-white bg-gray-900/85 font-bold font-mono px-1.5 py-0.5 rounded backdrop-blur-[1px] shrink-0">
                                {item.external_id}
                            </span>
                        )} */}
                        <span className="font-medium text-gray-700 truncate">{item.name}</span>
                    </div>
                    <span className="font-bold text-gray-900 bg-gray-100 px-1.5 py-0.5 rounded text-[0.625rem] shrink-0">x{item.quantity}</span>
                  </div>
                ))
              ) : (
                <div className="flex items-center gap-2 text-gray-500">
                  <Users size={14} />
                  <span className="font-medium">
                    {guestInput || '0'} {t('table.guests')}
                  </span>
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
