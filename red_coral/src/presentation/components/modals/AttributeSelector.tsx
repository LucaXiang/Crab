import React from 'react';
import { Check } from 'lucide-react';
import { AttributeTemplate, AttributeOption } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/formatCurrency';

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

  const isSingleChoice = attribute.type_.startsWith('SINGLE');
  const isRequired = attribute.type_.includes('REQUIRED');

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
    const labels: Record<string, string> = {
      SINGLE_REQUIRED: t('settings.attribute.type.singleRequired'),
      SINGLE_OPTIONAL: t('settings.attribute.type.singleOptional'),
      MULTI_REQUIRED: t('settings.attribute.type.multiRequired'),
      MULTI_OPTIONAL: t('settings.attribute.type.multiOptional'),
    };
    return labels[attribute.type_] || attribute.type_;
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
            <span className="text-[10px] bg-red-50 text-red-600 px-1.5 py-0.5 rounded border border-red-100 font-medium">
              {t('common.required')}
            </span>
          )}
        </h3>
        <span className="text-xs text-gray-400">
          {getAttributeTypeLabel()}
        </span>
      </div>

      {/* Options Grid - Card Style (Unified with Specifications) */}
      <div className="grid grid-cols-3 sm:grid-cols-4 gap-2">
        {activeOptions.map((option) => {
          const optionIdStr = String(option.id);
          const isSelected = selectedOptionIds.includes(optionIdStr);
          const isDefault = defaultOptionIds.includes(optionIdStr);

          return (
            <button
              key={option.id}
              onClick={() => {
                isSingleChoice ? handleSingleSelect(optionIdStr) : handleMultiSelect(optionIdStr);
              }}
              className={`
                relative p-2 rounded-xl border-2 transition-all text-left flex flex-col items-start min-h-[60px] justify-center
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
                <span className={`text-[10px] font-medium ${option.price_modifier > 0 ? 'text-orange-600' : 'text-green-600'}`}>
                  {option.price_modifier > 0 ? '+' : ''}{formatCurrency(option.price_modifier)}
                </span>
              ) : (
                // Placeholder to keep height consistent if needed, or just let it collapse
                <span className="text-[10px] text-gray-400 opacity-50">
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
                <span className="absolute top-1.5 right-1.5 w-1.5 h-1.5 bg-blue-500 rounded-full border border-white" title={t('common.default')} />
              )}
            </button>
          );
        })}
      </div>
    </div>
  );
});
