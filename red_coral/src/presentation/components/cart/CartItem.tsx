import React from 'react';
import { Minus, Plus } from 'lucide-react';
import { CartItem as CartItemType } from '@/core/domain/types';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { formatCurrency, Currency } from '@/utils/currency';
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
  
  // Calculate options modifier for display
  const optionsModifier = (item.selected_options ?? []).reduce((sum, opt) => sum + (opt.price_modifier ?? 0), 0);
  const basePrice = item.original_price ?? item.price;
  const baseUnitPrice = basePrice + optionsModifier;

  // Use server-computed unit_price, fallback to local calculation
  let finalUnitPrice = item.unit_price;
  
  if (finalUnitPrice === undefined || finalUnitPrice === null) {
    if (discountPercent > 0) {
      // Calculate discounted price: (base + options) * (1 - discount%)
      const discountFactor = Currency.sub(1, Currency.div(discountPercent, 100));
      finalUnitPrice = Currency.floor2(
        Currency.mul(baseUnitPrice, discountFactor)
      ).toNumber();
    } else {
      finalUnitPrice = baseUnitPrice;
    }
  }

  // Use server-computed line_total, fallback to local calculation
  const finalLineTotal = item.line_total ?? Currency.floor2(Currency.mul(finalUnitPrice, item.quantity)).toNumber();

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

  const hasMultiSpec = item.selected_specification?.is_multi_spec;
  const hasOptions = item.selected_options && item.selected_options.length > 0;
  const hasNote = item.note && item.note.trim().length > 0;

  return (
    <div
      className={`flex justify-between items-start py-2 px-3 relative group cursor-pointer ${
        performanceMode ? 'hover:bg-gray-100' : 'hover:bg-gray-50'
      }`}
    >
      <div className="flex-1 min-w-0 pr-4" {...clickHandlers}>
        {/* Line 1: Product Name */}
        <div className="font-medium text-gray-800 text-lg truncate">
          {item.name}
        </div>

        {/* Line 2: Specification (if multi-spec) */}
        {hasMultiSpec && (
          <div className="text-sm text-gray-600 mt-0.5">
            {item.selected_specification!.name}
          </div>
        )}

        {/* Line 3: Attribute Tags */}
        {hasOptions && (
          <div className="flex flex-wrap gap-1 mt-1">
            {item.selected_options!.map((opt, idx) => (
              <span
                key={idx}
                className="text-xs bg-gray-100 text-gray-600 px-1.5 py-0.5 rounded"
              >
                {opt.attribute_name}:{opt.option_name}
                {opt.price_modifier != null && opt.price_modifier !== 0 && (
                  <span className={opt.price_modifier > 0 ? 'text-orange-600 ml-0.5' : 'text-green-600 ml-0.5'}>
                    {opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)}
                  </span>
                )}
              </span>
            ))}
          </div>
        )}

        {/* Line 4: Note */}
        {hasNote && (
          <div className="text-xs text-blue-600 mt-1 flex items-center gap-1">
            <span>📝</span>
            <span className="truncate">{item.note}</span>
          </div>
        )}

        {/* Line 5: Unit Price */}
        <div className="flex items-center gap-2 mt-1">
          {discountPercent > 0 ? (
            <>
              <span className="text-sm text-gray-400 line-through">{formatCurrency(baseUnitPrice)}</span>
              <span className="text-base text-[#FF5E5E] font-bold">{formatCurrency(finalUnitPrice)}</span>
            </>
          ) : (
            <div className="text-sm text-[#FF5E5E]">{formatCurrency(finalUnitPrice)}</div>
          )}
        </div>
      </div>

      {/* Right Column */}
      <div className="flex flex-col items-end gap-2 shrink-0">
        {/* Line Total + Discount Badge */}
        <div className="flex items-center gap-2">
          {discountPercent > 0 && (
            <span className="text-xs bg-orange-100 text-orange-700 px-1.5 py-0.5 rounded">
              -{discountPercent}%
            </span>
          )}
          <div className="font-bold text-gray-700 text-lg">
            {formatCurrency(finalLineTotal)}
          </div>
        </div>

        {/* External ID + Quantity Control */}
        <div className="flex items-center gap-2">
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
