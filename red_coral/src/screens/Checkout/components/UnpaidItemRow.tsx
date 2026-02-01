import React from 'react';
import { CartItem, CheckoutMode } from '@/core/domain/types';
import { useLongPress } from '@/hooks/useLongPress';
import { formatCurrency } from '@/utils/currency';
import { t } from '@/infrastructure/i18n';
import { GroupedOptionsList } from '@/shared/components';

interface UnpaidItemRowProps {
  item: CartItem;
  remainingQty: number;
  originalIndex: number;
  mode: CheckoutMode;
  selectedQuantities: Record<number, number>;
  onUpdateSelectedQty: (index: number, delta: number) => void;
  onEditItem: (item: CartItem) => void;
  surchargeExempt?: boolean;
}

export const UnpaidItemRow: React.FC<UnpaidItemRowProps> = ({
  item,
  remainingQty,
  originalIndex,
  mode,
  selectedQuantities,
  onUpdateSelectedQty,
  onEditItem,
  surchargeExempt,
}) => {
  const isSelected = selectedQuantities[originalIndex] > 0;
  const currentQty = selectedQuantities[originalIndex] || 0;

  // Price calculations
  const optionsModifier = (item.selected_options ?? []).reduce((sum, opt) => sum + (opt.price_modifier ?? 0), 0);
  const basePrice = (item.original_price ?? item.price) + optionsModifier;
  const unitPrice = surchargeExempt
    ? (item.unit_price ?? item.price) - (item.surcharge ?? 0)
    : (item.unit_price ?? item.price);
  const discountPercent = item.manual_discount_percent || 0;
  const hasDiscount = discountPercent > 0 || basePrice !== unitPrice;
  const isSelectMode = mode === 'SELECT';

  const hasMultiSpec = item.selected_specification?.is_multi_spec;
  const hasOptions = item.selected_options && item.selected_options.length > 0;
  const hasNote = item.note && item.note.trim().length > 0;

  const clickHandlers = useLongPress(
    () => {},
    () => {
      if (isSelectMode) {
        onEditItem(item);
      }
    },
    { delay: 500, isPreventDefault: false }
  );

  return (
    <div
      {...clickHandlers}
      className={`
        group relative bg-white border rounded-xl p-4 transition-all duration-200 select-none
        ${isSelected
          ? 'border-blue-500 ring-1 ring-blue-500 shadow-md bg-blue-50/5'
          : 'border-gray-200 hover:border-blue-300 hover:shadow-md'
        }
        ${isSelectMode ? 'cursor-pointer hover:bg-gray-50' : ''}
      `}
    >
      <div className="flex items-start justify-between gap-4">
        {/* Left: Item Info */}
        <div className="flex-1 min-w-0">
          {/* Line 1: Product Name */}
          <div className="font-bold text-gray-900 text-lg truncate">
            {item.name}
          </div>

          {/* Line 2: Specification (if multi-spec) */}
          {hasMultiSpec && (
            <div className="text-sm text-gray-600 mt-0.5">
              {t('pos.cart.spec')}: {item.selected_specification!.name}
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

          {/* Line 5: Instance ID + Quantity × Unit Price */}
          <div className="flex items-center gap-2 mt-2 text-sm text-gray-500 tabular-nums">
            {item.instance_id && (
              <span className="px-1.5 py-0.5 rounded text-[0.625rem] font-bold font-mono border bg-blue-100 text-blue-600 border-blue-200">
                #{item.instance_id.slice(-5)}
              </span>
            )}
            <span className="font-medium">x{remainingQty}</span>
            <span className="w-1 h-1 bg-gray-300 rounded-full" />
            {hasDiscount ? (
              <>
                <span className="line-through text-gray-400">{formatCurrency(basePrice)}</span>
                <span className="font-semibold text-gray-700">{formatCurrency(unitPrice)}</span>
              </>
            ) : (
              <span className="font-semibold text-gray-700">{formatCurrency(unitPrice)}</span>
            )}
          </div>
        </div>

        {/* Right: Total + Controls */}
        <div className="flex flex-col items-end justify-between shrink-0 self-stretch">
          {/* Line Total + Badges */}
          <div className="flex items-center gap-2">
            {discountPercent > 0 && (
              <span className="text-xs bg-orange-100 text-orange-700 px-1.5 py-0.5 rounded">
                -{discountPercent}%
              </span>
            )}
            {!surchargeExempt && item.surcharge ? (
              <span className="text-xs bg-purple-100 text-purple-600 px-1.5 py-0.5 rounded">
                +{formatCurrency(item.surcharge)}
              </span>
            ) : null}
            <div className={`font-bold text-xl tabular-nums ${isSelected ? 'text-blue-600' : 'text-gray-900'}`}>
              {formatCurrency(unitPrice * remainingQty)}
            </div>
          </div>

          {/* External ID + Quantity Selector */}
          <div className="flex items-center gap-2">
            {item.selected_specification?.external_id && (
              <div className="text-xs text-white bg-gray-900/85 font-bold font-mono px-2 py-0.5 rounded">
                {item.selected_specification.external_id}
              </div>
            )}
            {!isSelectMode && (
              <div
                className={`
                  flex items-center bg-gray-50 rounded-lg p-1 border transition-colors
                  ${isSelected ? 'border-blue-200 bg-blue-50' : 'border-gray-200'}
                `}
                onClick={(e) => e.stopPropagation()}
              >
                <button
                  onClick={() => onUpdateSelectedQty(originalIndex, -1)}
                  className={`
                    w-8 h-8 flex items-center justify-center rounded-md transition-all font-bold
                    ${currentQty > 0
                      ? 'bg-white shadow-sm text-gray-700 hover:text-blue-600'
                      : 'text-gray-300 cursor-not-allowed'}
                  `}
                  disabled={currentQty <= 0}
                >
                  -
                </button>
                <div className="w-14 text-center">
                  <span className={`font-bold ${currentQty > 0 ? 'text-blue-600' : 'text-gray-800'}`}>
                    {currentQty}
                  </span>
                  <span className="text-gray-400 text-sm mx-0.5">/</span>
                  <span className="text-gray-500 text-sm">{remainingQty}</span>
                </div>
                <button
                  onClick={() => onUpdateSelectedQty(originalIndex, 1)}
                  className={`
                    w-8 h-8 flex items-center justify-center rounded-md transition-all font-bold
                    ${currentQty < remainingQty
                      ? 'bg-white shadow-sm text-gray-700 hover:text-blue-600'
                      : 'text-gray-300 cursor-not-allowed'}
                  `}
                  disabled={currentQty >= remainingQty}
                >
                  +
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
