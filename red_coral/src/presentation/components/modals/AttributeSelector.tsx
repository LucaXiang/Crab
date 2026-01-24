import React from 'react';
import { Check } from 'lucide-react';
import { AttributeTemplate, AttributeOption } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency/formatCurrency';

interface AttributeSelectorProps {
  attribute: AttributeTemplate;
  options: AttributeOption[];
  selectedOptionIds: string[];
  defaultOptionIds?: string[];
  onSelect: (optionIds: string[]) => void;
}

export const AttributeSelector: React.FC<AttributeSelectorProps> = React.memo(({
  attribute,
  options,
  selectedOptionIds,
  defaultOptionIds = [],
  onSelect,
}) => {
  const { t } = useI18n();

  // is_multi_select=false means single select, is_multi_select=true means multi select
  const isSingleChoice = !attribute.is_multi_select;
  // Note: Required is no longer part of attribute type, check via binding's is_required if needed
  const isRequired = false;

  // Filter only active options
  const activeOptions = options.filter(opt => opt.is_active);

  const handleSingleSelect = (optionId: string) => {
    // For single choice: toggle if optional, or just select if required
    if (selectedOptionIds.includes(optionId) && !isRequired) {
      onSelect([]);
    } else {
      onSelect([optionId]);
    }
  };

  const handleMultiSelect = (optionId: string) => {
    if (selectedOptionIds.includes(optionId)) {
      onSelect(selectedOptionIds.filter(id => id !== optionId));
    } else {
      onSelect([...selectedOptionIds, optionId]);
    }
  };

  const getAttributeTypeLabel = () => {
    return attribute.is_multi_select
      ? t('settings.attribute.type.multi_optional')
      : t('settings.attribute.type.single_optional');
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
          {isRequired && (
            <span className="text-[0.625rem] bg-red-50 text-red-600 px-1.5 py-0.5 rounded border border-red-100 font-medium">
              {t('common.label.required')}
            </span>
          )}
        </h3>
        <span className="text-xs text-gray-400">
          {getAttributeTypeLabel()}
        </span>
      </div>

      {/* Options Grid - Card Style (Unified with Specifications) */}
      <div className="grid grid-cols-3 sm:grid-cols-4 gap-2">
        {activeOptions.map((option, optionIdx) => {
          // Use index as ID since AttributeOption doesn't have id field
          const optionIdStr = String(optionIdx);
          const isSelected = selectedOptionIds.includes(optionIdStr);
          const isDefault = defaultOptionIds.includes(optionIdStr);

          return (
            <button
              key={`${option.name}-${optionIdx}`}
              onClick={() => {
                isSingleChoice ? handleSingleSelect(optionIdStr) : handleMultiSelect(optionIdStr);
              }}
              className={`
                relative p-2 rounded-xl border-2 transition-all text-left flex flex-col items-start min-h-[3.75rem] justify-center
                ${isSelected 
                  ? 'border-orange-500 bg-orange-50 ring-2 ring-orange-200' 
                  : 'bg-white text-gray-700 border-gray-200 hover:border-orange-300 hover:bg-orange-50/30'
                }
              `}
            >
              <span className={`text-xs font-bold mb-0.5 leading-tight ${isSelected ? 'text-orange-900' : 'text-gray-900'}`}>
                {option.name}
              </span>
              
              {option.price_modifier !== 0 ? (
                <span className={`text-[0.625rem] font-medium ${option.price_modifier > 0 ? 'text-orange-600' : 'text-green-600'}`}>
                  {option.price_modifier > 0 ? '+' : ''}{formatCurrency(option.price_modifier)}
                </span>
              ) : (
                // Placeholder to keep height consistent if needed, or just let it collapse
                <span className="text-[0.625rem] text-gray-400 opacity-50">
                   {formatCurrency(0)}
                </span>
              )}

              {/* Selection Checkmark */}
              {isSelected && (
                <div className="absolute top-1.5 right-1.5">
                  <div className="w-3.5 h-3.5 bg-orange-500 rounded-full flex items-center justify-center">
                    <Check size={9} className="text-white" strokeWidth={3} />
                  </div>
                </div>
              )}

              {/* Default Badge */}
              {isDefault && !isSelected && (
                <span className="absolute top-1.5 right-1.5 w-1.5 h-1.5 bg-blue-500 rounded-full border border-white" title={t('common.label.default')} />
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
});
