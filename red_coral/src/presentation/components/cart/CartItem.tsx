import React from 'react';
import { Minus, Plus } from 'lucide-react';
import { CartItem as CartItemType } from '@/core/domain/types';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { calculateItemFinalPrice, calculateItemTotal, calculateOptionsModifier } from '@/utils/pricing';
import { formatCurrency } from '@/utils/currency';
import { groupOptionsByAttribute } from '@/utils/formatting';

import { useLongPress } from '@/hooks/useLongPress';

interface CartItemProps {
  item: CartItemType;
  onQuantityChange: (instanceId: string, delta: number) => void;
  onClick: (item: CartItemType) => void;
}

export const CartItem = React.memo<CartItemProps>(({
  item,
  onQuantityChange,
  onClick
}) => {
  const performanceMode = useSettingsStore(state => state.performanceMode);
  const discountPercent = item.manual_discount_percent || 0;
  const optionsModifier = calculateOptionsModifier(item.selected_options).toNumber();
  const baseUnitPrice = (item.original_price ?? item.price) + optionsModifier;
  const finalUnitPrice = calculateItemFinalPrice(item).toNumber();
  // Use backend-computed line_total for consistency with order total, fall back to local calculation
  const finalLineTotal = item.line_total ?? calculateItemTotal(item).toNumber();

  const handleQuantityChange = (e: React.MouseEvent, delta: number) => {
    e.stopPropagation();
    const newQty = item.quantity + delta;
    if (newQty >= 1) {
      onQuantityChange(item.instance_id, delta);
    }
  };

  // Use useLongPress to prevent scroll-clicks
  const clickHandlers = useLongPress(
    () => {}, // No long press action
    () => onClick(item),
    { delay: 500, isPreventDefault: false }
  );

  return (
    <div
      className={`flex justify-between items-center py-2 px-3 relative group cursor-pointer ${
        performanceMode ? 'hover:bg-gray-100' : 'hover:bg-gray-50'
      }`}
    >


      <div className="flex-1 min-w-0 pr-4" {...clickHandlers}>
        <div className="font-medium text-gray-800 text-lg truncate">
          {item.name}{item.selected_specification?.is_multi_spec ? ` (${item.selected_specification.name})` : ''}
        </div>

        {/* Selected Options */}
        {item.selected_options && item.selected_options.length > 0 && (
          <div className="text-xs text-gray-500 mt-1 space-y-1">
            {groupOptionsByAttribute(item.selected_options).map((group, idx) => (
              <div key={idx} className="flex items-center gap-1">
                <span>{group.attributeName}: {group.optionNames.join(', ')}</span>
                {group.totalPrice !== 0 && (
                  <span className={group.totalPrice > 0 ? 'text-orange-600' : 'text-green-600'}>
                    ({group.totalPrice > 0 ? '+' : ''}{formatCurrency(group.totalPrice)})
                  </span>
                )}
              </div>
            ))}
          </div>
        )}

        <div className="flex items-center gap-2 mt-1">
          {discountPercent > 0 ? (
            <>
              <span className="text-sm text-gray-400 line-through">{formatCurrency(baseUnitPrice)}</span>
              <span className="text-base text-[#FF5E5E] font-bold">{formatCurrency(finalUnitPrice)}</span>
              <span className="text-xs bg-orange-100 text-orange-700 px-1.5 py-0.5 rounded flex items-center">
                -{discountPercent}%
              </span>
            </>
          ) : (
            <div className="text-sm text-[#FF5E5E]">{formatCurrency(finalUnitPrice)}</div>
          )}
        </div>
      </div>

      <div className="flex flex-col items-end gap-2">
        <div className="font-bold text-gray-700 text-lg">
          {formatCurrency(finalLineTotal)}
        </div>
        <div className="flex items-center gap-4">
          {item.selected_specification?.external_id && (
            <div className="text-xs text-white bg-gray-900/85 font-bold font-mono px-2 py-0.5 rounded">
              {item.selected_specification.external_id}
            </div>
          )}
          <div className="flex items-center bg-gray-100 rounded-lg overflow-hidden" onClick={e => e.stopPropagation()}>
            <button
              onClick={(e) => handleQuantityChange(e, -1)}
              className="p-2 hover:bg-gray-200 text-gray-600 transition-colors"
              disabled={item.quantity <= 1}
            >
              <Minus size={16} strokeWidth={3} />
            </button>
            <span className="w-8 text-center font-semibold text-gray-700 text-base">{item.quantity}</span>
            <button
              onClick={(e) => handleQuantityChange(e, 1)}
              className="p-2 hover:bg-gray-200 text-gray-600 transition-colors"
            >
              <Plus size={16} strokeWidth={3} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
});

CartItem.displayName = 'CartItem';
