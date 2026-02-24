import React, { useState, useEffect } from 'react';
import { Calculator, Percent, DollarSign } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { Currency } from '@/utils/currency';
import type { RuleWizardState } from './index';
import { FormSection, FormField, inputClass } from '@/shared/components/FormField';

interface Step1AdjustmentProps {
  state: RuleWizardState;
  updateState: (updates: Partial<RuleWizardState>) => void;
}

export const Step1Adjustment: React.FC<Step1AdjustmentProps> = ({ state, updateState }) => {
  const { t } = useI18n();
  const [inputValue, setInputValue] = useState(() =>
    state.adjustment_value > 0 ? String(state.adjustment_value) : ''
  );

  const isPercentage = state.adjustment_type === 'PERCENTAGE';

  useEffect(() => {
    setInputValue(state.adjustment_value > 0 ? String(state.adjustment_value) : '');
  }, [state.adjustment_type]);

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    if (value === '' || /^\d*\.?\d*$/.test(value)) {
      setInputValue(value);
    }
  };

  const handleBlur = () => {
    const parsed = parseFloat(inputValue);
    if (!isNaN(parsed) && parsed > 0) {
      const formatted = isPercentage ? Math.round(parsed) : Currency.round2(parsed).toNumber();
      updateState({ adjustment_value: formatted });
      setInputValue(String(formatted));
    } else {
      const fallback = state.adjustment_value > 0 ? state.adjustment_value : (isPercentage ? 10 : 1);
      updateState({ adjustment_value: fallback });
      setInputValue(String(fallback));
    }
  };

  return (
    <FormSection title={t('settings.marketing_group.rule_wizard.step1_section')} icon={Calculator}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.marketing_group.rule_wizard.step1_desc')}
      </p>

      {/* Adjustment Type */}
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
                ? 'border-violet-500 bg-violet-50 text-violet-700'
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
                ? 'border-violet-500 bg-violet-50 text-violet-700'
                : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300'
            }`}
          >
            <DollarSign size={20} />
            <span className="font-medium">{t('settings.price_rule.adjustment.fixed')}</span>
          </button>
        </div>
      </div>

      {/* Value Input */}
      <FormField label={t('settings.marketing_group.rule_wizard.value_label')} required>
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
            {isPercentage ? '%' : '€'}
          </span>
        </div>
      </FormField>

      {/* Preview */}
      <div className="mt-6 p-4 bg-gray-50 rounded-xl">
        <p className="text-sm text-gray-600">
          <span className="font-medium">{t('settings.price_rule.wizard.preview')}: </span>
          {isPercentage
            ? t('settings.price_rule.wizard.preview_discount_percent', { value: state.adjustment_value })
            : t('settings.price_rule.wizard.preview_discount_fixed', { value: state.adjustment_value.toFixed(2) })}
        </p>
      </div>
    </FormSection>
  );
};
