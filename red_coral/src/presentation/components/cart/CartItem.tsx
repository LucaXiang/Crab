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
  const discountPercent = item.discountPercent || 0;
  const optionsModifier = calculateOptionsModifier(item.selectedOptions).toNumber();
  const baseUnitPrice = (item.originalPrice ?? item.price) + optionsModifier;
  const finalUnitPrice = calculateItemFinalPrice(item).toNumber();
  const finalLineTotal = calculateItemTotal(item).toNumber();

  const handleQuantityChange = (e: React.MouseEvent, delta: number) => {
    e.stopPropagation();
    const newQty = item.quantity + delta;
    if (newQty >= 1) {
      onQuantityChange(item.instanceId || item.id, delta);
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
	      className={`flex justify-between items-center p-3 relative group cursor-pointer ${
	        performanceMode ? 'hover:bg-gray-100' : 'hover:bg-gray-50'
	      }`}
	    >


	      <div className="flex-1 min-w-0 pr-3" {...clickHandlers}>
        <div className="font-medium text-gray-800 text-sm truncate">
          {item.name} {item.selectedSpecification ? `(${item.selectedSpecification.name})` : ''}
        </div>

        {/* Selected Options */}
        {item.selectedOptions && item.selectedOptions.length > 0 && (
          <div className="text-[10px] text-gray-500 mt-0.5 space-y-0.5">
            {groupOptionsByAttribute(item.selectedOptions).map((group, idx) => (
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

        <div className="flex items-center justify-between mt-0.5">
          <div className="flex items-center gap-2">
            {discountPercent > 0 ? (
              <>
                <span className="text-xs text-gray-400 line-through">{formatCurrency(baseUnitPrice)}</span>
                <span className="text-xs text-[#FF5E5E] font-bold">{formatCurrency(finalUnitPrice)}</span>
                <span className="text-[10px] bg-orange-100 text-orange-700 px-1 rounded flex items-center">
                  -{discountPercent}%
                </span>
              </>
            ) : (
              <div className="text-xs text-[#FF5E5E]">{formatCurrency(finalUnitPrice)}</div>
            )}
          </div>
          {item.externalId && (
            <div className="text-[10px] text-white bg-gray-900/85 font-bold font-mono px-1.5 py-0.5 rounded backdrop-blur-[1px]">
              {item.externalId}
            </div>
          )}
        </div>
      </div>

      <div className="flex items-center gap-3">
        <div className="flex items-center bg-gray-100 rounded overflow-hidden" onClick={e => e.stopPropagation()}>
          <button
            onClick={(e) => handleQuantityChange(e, -1)}
            className="p-1.5 hover:bg-gray-200 text-gray-600 transition-colors"
            disabled={item.quantity <= 1}
          >
            <Minus size={12} strokeWidth={3} />
          </button>
          <span className="w-6 text-center font-semibold text-gray-700 text-sm">{item.quantity}</span>
          <button
            onClick={(e) => handleQuantityChange(e, 1)}
            className="p-1.5 hover:bg-gray-200 text-gray-600 transition-colors"
          >
            <Plus size={12} strokeWidth={3} />
          </button>
        </div>
	        <div className="w-16 text-right font-bold text-gray-700 text-sm">
	          {formatCurrency(finalLineTotal)}
	        </div>
      </div>
    </div>
  );
});

CartItem.displayName = 'CartItem';
