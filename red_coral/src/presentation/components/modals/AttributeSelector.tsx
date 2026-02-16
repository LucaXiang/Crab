import React from 'react';
import { Check, Minus, Plus } from 'lucide-react';
import { Attribute, AttributeOption } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency/formatCurrency';

interface AttributeSelectorProps {
  attribute: Attribute;
  options: AttributeOption[];
  /** Map of optionId -> quantity (quantity > 0 means selected) */
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

  // All options are now active (is_active field removed)
  const activeOptions = options;

  // Count selected options (quantity > 0)
  const selectedCount = Array.from(selectedOptions.values()).filter(q => q > 0).length;

  const maxSel = attribute.max_selections;
  const isAtLimit = !!(maxSel && selectedCount >= maxSel);

  const handleOptionToggle = (optionIdStr: string, option: AttributeOption) => {
    const newSelections = new Map(selectedOptions);
    const currentQty = newSelections.get(optionIdStr) || 0;

    if (!option.enable_quantity) {
      // Traditional toggle logic (no quantity control)
      if (currentQty > 0) {
        // Deselect
        newSelections.delete(optionIdStr);
      } else {
        // Select
        if (isSingleChoice) {
          // Clear all other selections for single choice
          newSelections.clear();
        } else if (isAtLimit) {
          return; // Can't add more for multi-select at limit
        }
        newSelections.set(optionIdStr, 1);
      }
    } else {
      // Quantity control enabled: first click sets to 1
      if (currentQty === 0) {
        if (isSingleChoice) {
          newSelections.clear();
        } else if (isAtLimit) {
          return;
        }
        newSelections.set(optionIdStr, 1);
      }
      // If already selected, clicking the main area does nothing (use +/- buttons)
    }
    onSelect(newSelections);
  };

  const handleQuantityChange = (optionIdStr: string, delta: number, option: AttributeOption) => {
    const newSelections = new Map(selectedOptions);
    const currentQty = newSelections.get(optionIdStr) || 0;
    const newQty = Math.max(0, currentQty + delta);

    // Apply max_quantity limit
    const maxQty = option.max_quantity || 99;
    const finalQty = Math.min(newQty, maxQty);

    if (finalQty === 0) {
      newSelections.delete(optionIdStr);
    } else {
      newSelections.set(optionIdStr, finalQty);
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

      {/* Options */}
      <div className="flex flex-wrap gap-2">
        {activeOptions.map((option) => {
          const optionIdStr = String(option.id);
          const quantity = selectedOptions.get(optionIdStr) || 0;
          const isSelected = quantity > 0;
          const isDefault = defaultOptionIds.includes(optionIdStr);
          const isDisabled = isAtLimit && !isSelected && !isSingleChoice;
          const hasQuantityControl = option.enable_quantity;
          const maxQty = option.max_quantity || 99;

          // Options with quantity control: selected shows stepper, unselected looks normal
          if (hasQuantityControl) {
            if (isSelected) {
              return (
                <div
                  key={`${option.name}-${option.id}`}
                  className="relative rounded-full bg-orange-500 text-white shadow-md shadow-orange-200 transition-all duration-150 flex items-center"
                >
                  {/* Label + price: click to +1 */}
                  <button
                    onClick={() => {
                      if (quantity >= maxQty) return;
                      handleQuantityChange(optionIdStr, 1, option);
                    }}
                    disabled={quantity >= maxQty}
                    className="flex items-center gap-1.5 pl-4 pr-1 py-2 disabled:cursor-not-allowed"
                  >
                    <span className="text-sm font-medium whitespace-nowrap">{option.name}</span>
                    {option.price_modifier !== 0 && (
                      <span className="text-xs font-medium whitespace-nowrap text-white/80">
                        {option.price_modifier > 0 ? '+' : ''}{formatCurrency(option.price_modifier)}
                      </span>
                    )}
                  </button>

                  {/* Quantity stepper */}
                  <div className="flex items-center gap-0.5 pr-1.5">
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleQuantityChange(optionIdStr, -1, option);
                      }}
                      className="w-8 h-8 rounded-full bg-white/20 hover:bg-white/30 active:bg-white/40 flex items-center justify-center transition-colors"
                    >
                      <Minus size={14} strokeWidth={2.5} />
                    </button>
                    <span className="text-sm font-bold w-6 text-center tabular-nums">{quantity}</span>
                    <button
                      onClick={(e) => {
                        e.stopPropagation();
                        handleQuantityChange(optionIdStr, 1, option);
                      }}
                      disabled={quantity >= maxQty}
                      className="w-8 h-8 rounded-full bg-white/20 hover:bg-white/30 active:bg-white/40 flex items-center justify-center transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
                    >
                      <Plus size={14} strokeWidth={2.5} />
                    </button>
                  </div>
                </div>
              );
            }

            // Unselected: normal pill, click to +1
            return (
              <div
                key={`${option.name}-${option.id}`}
                className={`
                  relative rounded-full transition-all duration-150
                  ${isDisabled
                    ? 'bg-gray-100 text-gray-300'
                    : 'bg-gray-100 text-gray-700 hover:bg-orange-100 hover:text-orange-700'
                  }
                `}
              >
                <button
                  onClick={() => {
                    if (isDisabled) return;
                    handleQuantityChange(optionIdStr, 1, option);
                  }}
                  disabled={isDisabled}
                  className="flex items-center gap-1.5 px-4 py-2"
                >
                  <span className="text-sm font-medium whitespace-nowrap">{option.name}</span>
                  {option.price_modifier !== 0 && (
                    <span className={`text-xs font-medium whitespace-nowrap ${
                      option.price_modifier > 0 ? 'text-orange-500' : 'text-green-600'
                    }`}>
                      {option.price_modifier > 0 ? '+' : ''}{formatCurrency(option.price_modifier)}
                    </span>
                  )}
                </button>
                {isDefault && (
                  <span className="absolute -top-0.5 -right-0.5 w-2 h-2 bg-blue-500 rounded-full border-2 border-white" />
                )}
              </div>
            );
          }

          return (
            <div
              key={`${option.name}-${option.id}`}
              className={`
                relative rounded-full transition-all duration-150
                ${isSelected
                  ? 'bg-orange-500 text-white shadow-md shadow-orange-200'
                  : isDisabled
                    ? 'bg-gray-100 text-gray-300'
                    : 'bg-gray-100 text-gray-700 hover:bg-orange-100 hover:text-orange-700'
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
                className="flex items-center gap-1.5 px-4 py-2"
              >
                {isSelected && (
                  <Check size={14} strokeWidth={2.5} />
                )}
                <span className="text-sm font-medium whitespace-nowrap">
                  {option.name}
                </span>
                {option.price_modifier !== 0 && (
                  <span className={`text-xs font-medium whitespace-nowrap ${
                    isSelected
                      ? 'text-white/80'
                      : option.price_modifier > 0 ? 'text-orange-500' : 'text-green-600'
                  }`}>
                    {option.price_modifier > 0 ? '+' : ''}{formatCurrency(option.price_modifier)}
                  </span>
                )}
              </button>

              {/* Default dot */}
              {isDefault && !isSelected && (
                <span className="absolute -top-0.5 -right-0.5 w-2 h-2 bg-blue-500 rounded-full border-2 border-white" />
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
});
