import React from 'react';
import { Check, Minus, Plus } from 'lucide-react';
import { Attribute, AttributeOption } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency/formatCurrency';

interface AttributeSelectorProps {
  attribute: Attribute;
  options: AttributeOption[];
  /** Map of optionIdx -> quantity (quantity > 0 means selected) */
  selectedOptions: Map<string, number>;
  defaultOptionIds?: string[];
  onSelect: (options: Map<string, number>) => void;
}

export const AttributeSelector: React.FC<AttributeSelectorProps> = React.memo(({
  attribute,
  options,
  selectedOptions,
  defaultOptionIds = [],
  onSelect,
}) => {
  const { t } = useI18n();

  // is_multi_select=false means single select, is_multi_select=true means multi select
  const isSingleChoice = !attribute.is_multi_select;

  // Filter only active options
  const activeOptions = options.filter(opt => opt.is_active);

  // Count selected options (quantity > 0)
  const selectedCount = Array.from(selectedOptions.values()).filter(q => q > 0).length;

  const maxSel = attribute.max_selections;
  const isAtLimit = !!(maxSel && selectedCount >= maxSel);

  const handleOptionToggle = (optionIdx: string, option: AttributeOption) => {
    const newSelections = new Map(selectedOptions);
    const currentQty = newSelections.get(optionIdx) || 0;

    if (!option.enable_quantity) {
      // Traditional toggle logic (no quantity control)
      if (currentQty > 0) {
        // Deselect
        newSelections.delete(optionIdx);
      } else {
        // Select
        if (isSingleChoice) {
          // Clear all other selections for single choice
          newSelections.clear();
        } else if (isAtLimit) {
          return; // Can't add more for multi-select at limit
        }
        newSelections.set(optionIdx, 1);
      }
    } else {
      // Quantity control enabled: first click sets to 1
      if (currentQty === 0) {
        if (isSingleChoice) {
          newSelections.clear();
        } else if (isAtLimit) {
          return;
        }
        newSelections.set(optionIdx, 1);
      }
      // If already selected, clicking the main area does nothing (use +/- buttons)
    }
    onSelect(newSelections);
  };

  const handleQuantityChange = (optionIdx: string, delta: number, option: AttributeOption) => {
    const newSelections = new Map(selectedOptions);
    const currentQty = newSelections.get(optionIdx) || 0;
    const newQty = Math.max(0, currentQty + delta);

    // Apply max_quantity limit
    const maxQty = option.max_quantity || 99;
    const finalQty = Math.min(newQty, maxQty);

    if (finalQty === 0) {
      newSelections.delete(optionIdx);
    } else {
      newSelections.set(optionIdx, finalQty);
    }
    onSelect(newSelections);
  };

  const getAttributeTypeLabel = () => {
    const base = attribute.is_multi_select
      ? t('settings.attribute.type.multi_optional')
      : t('settings.attribute.type.single_optional');
    if (attribute.is_multi_select && maxSel) {
      return `${base} (${selectedCount}/${maxSel})`;
    }
    return base;
  };

  if (activeOptions.length === 0) {
    return null;
  }

  return (
    <div className="space-y-3">
      {/* Attribute Header */}
      <div className="flex items-center justify-between">
        <h3 className="font-semibold text-gray-800 flex items-center gap-2">
          {attribute.name}
        </h3>
        <span className={`text-xs ${isAtLimit ? 'text-orange-500 font-medium' : 'text-gray-400'}`}>
          {getAttributeTypeLabel()}
        </span>
      </div>

      {/* Options Grid - Card Style */}
      <div className="grid grid-cols-3 sm:grid-cols-4 gap-2">
        {activeOptions.map((option, optionIdx) => {
          const optionIdStr = String(optionIdx);
          const quantity = selectedOptions.get(optionIdStr) || 0;
          const isSelected = quantity > 0;
          const isDefault = defaultOptionIds.includes(optionIdStr);
          const isDisabled = isAtLimit && !isSelected && !isSingleChoice;
          const hasQuantityControl = option.enable_quantity;
          const maxQty = option.max_quantity || 99;

          return (
            <div
              key={`${option.name}-${optionIdx}`}
              className={`
                relative p-2 rounded-xl border-2 transition-all flex flex-col
                ${isSelected
                  ? 'border-orange-500 bg-orange-50 ring-2 ring-orange-200'
                  : isDisabled
                    ? 'bg-gray-50 text-gray-300 border-gray-100'
                    : 'bg-white text-gray-700 border-gray-200 hover:border-orange-300 hover:bg-orange-50/30'
                }
              `}
            >
              {/* Main clickable area */}
              <button
                onClick={() => {
                  if (isDisabled) return;
                  handleOptionToggle(optionIdStr, option);
                }}
                disabled={isDisabled}
                className="flex flex-col items-start text-left w-full min-h-[2.5rem]"
              >
                <span className={`text-xs font-bold mb-0.5 leading-tight ${isSelected ? 'text-orange-900' : 'text-gray-900'}`}>
                  {option.name}
                </span>

                {option.price_modifier !== 0 ? (
                  <span className={`text-[0.625rem] font-medium ${option.price_modifier > 0 ? 'text-orange-600' : 'text-green-600'}`}>
                    {option.price_modifier > 0 ? '+' : ''}{formatCurrency(option.price_modifier)}
                    {hasQuantityControl && quantity > 1 && ` ×${quantity}`}
                  </span>
                ) : (
                  <span className="text-[0.625rem] text-gray-400 opacity-50">
                    {formatCurrency(0)}
                    {hasQuantityControl && quantity > 1 && <span className="text-orange-600 opacity-100"> ×{quantity}</span>}
                  </span>
                )}
              </button>

              {/* Quantity control buttons (only when enabled and selected) */}
              {hasQuantityControl && isSelected && (
                <div className="flex items-center justify-center gap-1 mt-1.5 pt-1.5 border-t border-orange-200">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleQuantityChange(optionIdStr, -1, option);
                    }}
                    className="w-6 h-6 rounded-lg bg-gray-200 hover:bg-gray-300 flex items-center justify-center transition-colors"
                  >
                    <Minus size={12} className="text-gray-600" />
                  </button>
                  <span className="text-xs font-bold w-6 text-center text-gray-800">{quantity}</span>
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleQuantityChange(optionIdStr, 1, option);
                    }}
                    disabled={quantity >= maxQty}
                    className="w-6 h-6 rounded-lg bg-orange-500 hover:bg-orange-600 text-white flex items-center justify-center transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <Plus size={12} />
                  </button>
                </div>
              )}

              {/* Selection Checkmark */}
              {isSelected && !hasQuantityControl && (
                <div className="absolute top-1.5 right-1.5">
                  <div className="w-3.5 h-3.5 bg-orange-500 rounded-full flex items-center justify-center">
                    <Check size={9} className="text-white" strokeWidth={3} />
                  </div>
                </div>
              )}

              {/* Quantity badge (for quantity-enabled selected options) */}
              {isSelected && hasQuantityControl && (
                <div className="absolute top-1.5 right-1.5">
                  <div className="min-w-[1.25rem] h-5 bg-orange-500 rounded-full flex items-center justify-center px-1">
                    <span className="text-[0.625rem] text-white font-bold">{quantity}</span>
                  </div>
                </div>
              )}

              {/* Default Badge */}
              {isDefault && !isSelected && (
                <span className="absolute top-1.5 right-1.5 w-1.5 h-1.5 bg-blue-500 rounded-full border border-white" />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
});
