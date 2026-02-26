import React from 'react';
import { FileText, Info } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { WizardState } from './index';
import { FormSection, FormField, inputClass } from '@/shared/components/FormField';
import { MAX_NAME_LEN, MAX_RECEIPT_NAME_LEN, MAX_NOTE_LEN } from '@/shared/constants/validation';

interface Step5NamingProps {
  state: WizardState;
  updateState: (updates: Partial<WizardState>) => void;
}

export const Step5Naming: React.FC<Step5NamingProps> = ({ state, updateState }) => {
  const { t } = useI18n();

  // Generate suggested name from rule type and value
  const suggestName = () => {
    const type = state.rule_type === 'DISCOUNT' ? 'discount' : 'surcharge';
    const value = state.adjustment_type === 'PERCENTAGE'
      ? `${state.adjustment_value}pct`
      : `${state.adjustment_value}`;
    return `${type}_${value}`;
  };

  // Auto-fill name if empty
  React.useEffect(() => {
    if (!state.name && state.adjustment_value > 0) {
      updateState({ name: suggestName() });
    }
  }, [state.rule_type, state.adjustment_type, state.adjustment_value]);

  // Auto-fill receipt_name based on display_name
  React.useEffect(() => {
    if (state.display_name && !state.receipt_name) {
      updateState({ receipt_name: state.display_name.slice(0, MAX_RECEIPT_NAME_LEN) });
    }
  }, [state.display_name]);

  // Build summary text
  const buildSummary = () => {
    const parts: string[] = [];

    // Type
    parts.push(
      state.rule_type === 'DISCOUNT'
        ? t('settings.price_rule.type.discount')
        : t('settings.price_rule.type.surcharge')
    );

    // Value
    parts.push(
      state.adjustment_type === 'PERCENTAGE'
        ? `${state.adjustment_value}%`
        : `€${state.adjustment_value.toFixed(2)}`
    );

    // Scope
    const scopeLabels: Record<string, string> = {
      GLOBAL: t('settings.price_rule.scope.global'),
      CATEGORY: t('settings.price_rule.scope.category'),
      TAG: t('settings.price_rule.scope.tag'),
      PRODUCT: t('settings.price_rule.scope.product'),
    };
    parts.push(scopeLabels[state.product_scope]);

    // Time
    const timeLabels: Record<string, string> = {
      ALWAYS: t('settings.price_rule.time.always'),
      SCHEDULE: t('settings.price_rule.time.schedule'),
      ONETIME: t('settings.price_rule.time.onetime'),
    };
    parts.push(timeLabels[state.time_mode]);

    return parts.join(' · ');
  };

  return (
    <FormSection title={t('settings.price_rule.wizard.step5_section')} icon={FileText}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.price_rule.wizard.step5_desc')}
      </p>

      <div className="space-y-4">
        <FormField label={t('settings.price_rule.form.name')} required>
          <input
            type="text"
            value={state.name}
            onChange={(e) => updateState({ name: e.target.value })}
            placeholder="lunch_discount"
            maxLength={MAX_NAME_LEN}
            className={inputClass}
          />
          <p className="mt-1 text-xs text-gray-500">
            {t('settings.price_rule.wizard.name_hint')}
          </p>
        </FormField>

        <FormField label={t('settings.price_rule.form.display_name')} required>
          <input
            type="text"
            value={state.display_name}
            onChange={(e) => updateState({ display_name: e.target.value })}
            placeholder={t('settings.price_rule.wizard.display_name_placeholder')}
            maxLength={MAX_NAME_LEN}
            className={inputClass}
          />
          <p className="mt-1 text-xs text-gray-500">
            {t('settings.price_rule.wizard.display_name_hint')}
          </p>
        </FormField>

        <FormField label={t('settings.price_rule.form.receipt_name')} required>
          <input
            type="text"
            value={state.receipt_name}
            onChange={(e) => updateState({ receipt_name: e.target.value })}
            placeholder={t('settings.price_rule.wizard.receipt_name_placeholder')}
            maxLength={MAX_RECEIPT_NAME_LEN}
            className={inputClass}
          />
          <p className="mt-1 text-xs text-gray-500">
            {t('settings.price_rule.wizard.receipt_name_hint')}
          </p>
        </FormField>

        <FormField label={t('settings.price_rule.form.description')}>
          <textarea
            value={state.description}
            onChange={(e) => updateState({ description: e.target.value })}
            placeholder={t('settings.price_rule.wizard.description_placeholder')}
            maxLength={MAX_NOTE_LEN}
            rows={2}
            className={`${inputClass} resize-none`}
          />
        </FormField>
      </div>

      {/* Summary */}
      <div className="mt-6 p-4 bg-teal-50 rounded-xl border border-teal-100">
        <div className="flex items-start gap-3">
          <div className="w-8 h-8 rounded-lg bg-teal-100 flex items-center justify-center shrink-0">
            <Info size={16} className="text-teal-600" />
          </div>
          <div>
            <h4 className="font-medium text-teal-800 mb-1">
              {t('settings.price_rule.wizard.summary_title')}
            </h4>
            <p className="text-sm text-teal-700">{buildSummary()}</p>
          </div>
        </div>
      </div>
    </FormSection>
  );
};
