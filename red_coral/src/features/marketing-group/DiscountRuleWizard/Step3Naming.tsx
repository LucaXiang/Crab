import React from 'react';
import { FileText } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { RuleWizardState } from './index';
import { FormSection, FormField, inputClass } from '@/shared/components/FormField';
import { MAX_NAME_LEN, MAX_RECEIPT_NAME_LEN } from '@/shared/constants/validation';

interface Step3NamingProps {
  state: RuleWizardState;
  updateState: (updates: Partial<RuleWizardState>) => void;
}

export const Step3Naming: React.FC<Step3NamingProps> = ({ state, updateState }) => {
  const { t } = useI18n();

  return (
    <FormSection title={t('settings.marketing_group.rule_wizard.step3_section')} icon={FileText}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.marketing_group.rule_wizard.step3_desc')}
      </p>

      <div className="space-y-5">
        <FormField label={t('settings.marketing_group.field.name')} required>
          <input
            type="text"
            value={state.name}
            onChange={(e) => updateState({ name: e.target.value })}
            className={inputClass}
            maxLength={MAX_NAME_LEN}
            placeholder="mg_coffee_10off"
          />
          <p className="mt-1 text-xs text-gray-500">
            {t('settings.marketing_group.rule_wizard.name_hint')}
          </p>
        </FormField>

        <FormField label={t('settings.marketing_group.rule_wizard.receipt_name')}>
          <input
            type="text"
            value={state.receipt_name}
            onChange={(e) => updateState({ receipt_name: e.target.value })}
            className={inputClass}
            maxLength={MAX_RECEIPT_NAME_LEN}
            placeholder={t('settings.marketing_group.rule_wizard.receipt_name_placeholder')}
          />
          <p className="mt-1 text-xs text-gray-500">
            {t('settings.marketing_group.rule_wizard.receipt_name_hint')}
          </p>
        </FormField>
      </div>
    </FormSection>
  );
};
