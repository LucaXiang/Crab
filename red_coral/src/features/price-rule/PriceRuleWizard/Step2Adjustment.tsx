import React, { useState, useEffect } from 'react';
import { Calculator, Percent, DollarSign } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { WizardState } from './index';
import { FormSection, FormField, inputClass } from '@/shared/components/FormField';

interface Step2AdjustmentProps {
  state: WizardState;
  updateState: (updates: Partial<WizardState>) => void;
}

export const Step2Adjustment: React.FC<Step2AdjustmentProps> = ({ state, updateState }) => {
  const { t } = useI18n();

  // Local string state for free text input
  const [inputValue, setInputValue] = useState(() =>
    state.adjustment_value > 0 ? String(state.adjustment_value) : ''
  );

  const isDiscount = state.rule_type === 'DISCOUNT';
  const isPercentage = state.adjustment_type === 'PERCENTAGE';

  // Sync from parent state when adjustment_type changes
  useEffect(() => {
    setInputValue(state.adjustment_value > 0 ? String(state.adjustment_value) : '');
  }, [state.adjustment_type]);

  // Handle input change - allow free typing
  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    // Allow empty, digits, and one decimal point
    if (value === '' || /^\d*\.?\d*$/.test(value)) {
      setInputValue(value);
    }
  };

  // Handle blur - validate and update parent state
  const handleBlur = () => {
    const parsed = parseFloat(inputValue);
    if (!isNaN(parsed) && parsed > 0) {
      // Format: percentage as integer, fixed as 2 decimal places
      const formatted = isPercentage
        ? Math.round(parsed)
        : Math.round(parsed * 100) / 100;
      updateState({ adjustment_value: formatted });
      setInputValue(String(formatted));
    } else {
      // Reset to previous valid value or default
      const fallback = state.adjustment_value > 0 ? state.adjustment_value : (isPercentage ? 10 : 1);
      updateState({ adjustment_value: fallback });
      setInputValue(String(fallback));
    }
  };

  return (
    <FormSection title={t('settings.price_rule.wizard.step2_section')} icon={Calculator}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.price_rule.wizard.step2_desc')}
      </p>

      {/* Adjustment Type Selection */}
      <div className="mb-6">
        <label className="block text-sm font-medium text-gray-700 mb-3">
          {t('settings.price_rule.wizard.adjustment_type')}
        </label>
        <div className="grid grid-cols-2 gap-3">
          <button
            type="button"
            onClick={() => updateState({ adjustment_type: 'PERCENTAGE' })}
            className={`flex items-center justify-center gap-2 p-4 rounded-xl border-2 transition-all ${
              isPercentage
                ? 'border-teal-500 bg-teal-50 text-teal-700'
                : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300'
            }`}
          >
            <Percent size={20} />
            <span className="font-medium">{t('settings.price_rule.adjustment.percentage')}</span>
          </button>
          <button
            type="button"
            onClick={() => updateState({ adjustment_type: 'FIXED_AMOUNT' })}
            className={`flex items-center justify-center gap-2 p-4 rounded-xl border-2 transition-all ${
              !isPercentage
                ? 'border-teal-500 bg-teal-50 text-teal-700'
                : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300'
            }`}
          >
            <DollarSign size={20} />
            <span className="font-medium">{t('settings.price_rule.adjustment.fixed')}</span>
          </button>
        </div>
      </div>

      {/* Value Input */}
      <FormField
        label={isDiscount
          ? t('settings.price_rule.wizard.discount_value')
          : t('settings.price_rule.wizard.surcharge_value')}
        required
      >
        <div className="relative">
          <input
            type="text"
            inputMode="decimal"
            value={inputValue}
            onChange={handleInputChange}
            onBlur={handleBlur}
            className={`${inputClass} pr-12`}
            placeholder={isPercentage ? '10' : '5.00'}
          />
          <span className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 font-medium">
            {isPercentage ? '%' : '¥'}
          </span>
        </div>
        <p className="mt-1.5 text-xs text-gray-500">
          {isPercentage
            ? t('settings.price_rule.wizard.percentage_hint')
            : t('settings.price_rule.wizard.fixed_hint')}
        </p>
      </FormField>

      {/* Preview */}
      <div className="mt-6 p-4 bg-gray-50 rounded-xl">
        <p className="text-sm text-gray-600">
          <span className="font-medium">{t('settings.price_rule.wizard.preview')}: </span>
          {isDiscount ? (
            isPercentage
              ? t('settings.price_rule.wizard.preview_discount_percent', { value: state.adjustment_value })
              : t('settings.price_rule.wizard.preview_discount_fixed', { value: state.adjustment_value.toFixed(2) })
          ) : (
            isPercentage
              ? t('settings.price_rule.wizard.preview_surcharge_percent', { value: state.adjustment_value })
              : t('settings.price_rule.wizard.preview_surcharge_fixed', { value: state.adjustment_value.toFixed(2) })
          )}
        </p>
      </div>
    </FormSection>
  );
};
