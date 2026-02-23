import React, { useState } from 'react';
import { Minus, Plus, Percent } from 'lucide-react';
import { CartItem as CartItemType } from '@/core/domain/types';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { useProductStore } from '@/core/stores/resources';
import { formatCurrency, Currency } from '@/utils/currency';
import { calculateOptionsModifier, getSpecName } from '@/utils/pricing';
import { useLongPress } from '@/hooks/useLongPress';
import { t } from '@/infrastructure/i18n';
import { GroupedOptionsList } from '@/shared/components';

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
  const [isHoveringControl, setIsHoveringControl] = useState(false);
  const performanceMode = useSettingsStore(state => state.performanceMode);
  const externalId = useProductStore(state => state.items.find(p => p.id === item.id)?.external_id);
  const discountPercent = item.manual_discount_percent || 0;
  
  // Calculate options modifier for display (considering option quantity)
  const optionsModifier = calculateOptionsModifier(item.selected_options);
  const basePrice = item.original_price || item.price;
  const baseUnitPrice = basePrice + optionsModifier;

  // Use server-computed unit_price (always present)
  const finalUnitPrice = item.unit_price;

  // Use server-computed line_total (always present)
  const finalLineTotal = item.line_total;

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

  const specName = getSpecName(item.selected_specification);
  const hasOptions = item.selected_options && item.selected_options.length > 0;
  const hasNote = item.note && item.note.trim().length > 0;
  
  // Applied price rules (filter out skipped ones for display)
  const activeRules = item.applied_rules.filter(r => !r.skipped);
  const hasActiveRules = activeRules.length > 0;

  return (
    <div
      className={`flex justify-between items-stretch py-2 px-3 relative group cursor-pointer antialiased ${
        !isHoveringControl ? (performanceMode ? 'hover:bg-gray-100' : 'hover:bg-gray-50') : ''
      }`}
      {...clickHandlers}
    >
      <div className="flex-1 min-w-0 pr-4 flex flex-col justify-between">
        <div>
          {/* Line 1: Product Name */}
          <div className="font-medium text-gray-800 text-lg truncate">
            {item.name}
          </div>

          {/* Line 2: Specification (if multi-spec and name is non-empty) */}
          {specName && (
            <div className="text-sm text-gray-600 mt-0.5">
              {t('pos.cart.spec')}: {specName}
            </div>
          )}

          {/* Line 3: Attribute Tags (grouped by attribute, one per line) */}
          {hasOptions && <GroupedOptionsList options={item.selected_options!} />}

          {/* Line 4: Note */}
          {hasNote && (
            <div className="text-xs text-blue-600 mt-1 flex items-center gap-1">
              <span>📝</span>
              <span className="truncate">{item.note}</span>
            </div>
          )}

          {/* Line 5: Applied Price Rules (max 2 displayed) */}
          {hasActiveRules && (
            <div className="flex flex-wrap gap-1 mt-1">
              {activeRules.slice(0, 2).map((rule) => (
                <span
                  key={rule.rule_id}
                  className={`text-xs px-1.5 py-0.5 rounded flex items-center gap-0.5 ${
                    rule.rule_type === 'DISCOUNT'
                      ? 'bg-amber-100 text-amber-700'
                      : 'bg-purple-100 text-purple-700'
                  }`}
                  title={`${rule.display_name}: ${rule.adjustment_type === 'PERCENTAGE' ? `${rule.adjustment_value}%` : formatCurrency(rule.adjustment_value)}`}
                >
                  <Percent size={10} />
                  <span>{rule.receipt_name || rule.display_name}</span>
                  <span className="font-medium">
                    {rule.rule_type === 'DISCOUNT' ? '-' : '+'}
                    {formatCurrency(Math.abs(rule.calculated_amount))}
                  </span>
                </span>
              ))}
              {activeRules.length > 2 && (
                <span className="text-xs px-1.5 py-0.5 rounded bg-gray-100 text-gray-600">
                  +{activeRules.length - 2}
                </span>
              )}
            </div>
          )}
        </div>

        {/* Line 6: Unit Price */}
        <div className="flex items-center gap-2 mt-1">
          {discountPercent > 0 ? (
            <>
              <span className="text-sm text-gray-400 line-through">{formatCurrency(baseUnitPrice)}</span>
              <span className="text-base text-primary-500 font-bold">{formatCurrency(finalUnitPrice)}</span>
            </>
          ) : (
            <div className="text-sm text-primary-500">{formatCurrency(finalUnitPrice)}</div>
          )}
        </div>
      </div>

      {/* Right Column */}
      <div className="flex flex-col items-end justify-between gap-2 shrink-0">
        {/* Line Total + Discount Badge */}
        <div className="flex items-center gap-2">
          {discountPercent > 0 && (
            <span className="text-xs bg-orange-100 text-orange-700 px-1.5 py-0.5 rounded">
              -{discountPercent}%
            </span>
          )}
          {item.mg_discount_amount > 0 && (
            <span className="text-xs bg-red-100 text-red-700 px-1.5 py-0.5 rounded font-medium">
              -{formatCurrency(item.mg_discount_amount)}
            </span>
          )}
          <div className="font-bold text-gray-700 text-lg">
            {formatCurrency(finalLineTotal)}
          </div>
        </div>

        {/* External ID + Quantity Control */}
        <div className="flex items-end gap-2">
          {externalId != null && (
            <div className="text-xs text-white bg-gray-900/85 font-bold font-mono px-2 py-0.5 rounded">
              {externalId}
            </div>
          )}
          {/* Quantity Controls */}
          <div
            className="flex items-center bg-gray-100 rounded-lg overflow-hidden border border-gray-200 shadow-sm"
            onClick={e => e.stopPropagation()}
            onMouseDown={e => e.stopPropagation()}
            onMouseUp={e => e.stopPropagation()}
            onTouchStart={e => e.stopPropagation()}
            onTouchEnd={e => e.stopPropagation()}
            onMouseEnter={() => setIsHoveringControl(true)}
            onMouseLeave={() => setIsHoveringControl(false)}
          >
            <button
              onClick={(e) => handleQuantityChange(e, -1)}
              className="p-3 min-w-[2.75rem] min-h-[2.75rem] flex items-center justify-center hover:bg-gray-200 active:bg-gray-300 text-gray-600 transition-colors"
              disabled={item.quantity <= 1}
            >
              <Minus size={18} strokeWidth={2.5} />
            </button>
            
            <span className="w-10 text-center font-bold text-gray-900 text-lg tabular-nums select-none">
              {item.quantity}
            </span>

            <button
              onClick={(e) => handleQuantityChange(e, 1)}
              className="p-3 min-w-[2.75rem] min-h-[2.75rem] flex items-center justify-center hover:bg-gray-200 active:bg-gray-300 text-gray-600 transition-colors"
            >
              <Plus size={18} strokeWidth={2.5} />
            </button>
          </div>
        </div>
      </div>
    </div>
  );
});

CartItem.displayName = 'CartItem';
