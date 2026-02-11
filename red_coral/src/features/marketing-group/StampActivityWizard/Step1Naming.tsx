import React from 'react';
import { FileText } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { StampWizardState } from './index';
import { FormSection, FormField, inputClass } from '@/shared/components/FormField';

interface Step1NamingProps {
  state: StampWizardState;
  updateState: (updates: Partial<StampWizardState>) => void;
}

export const Step1Naming: React.FC<Step1NamingProps> = ({ state, updateState }) => {
  const { t } = useI18n();

  return (
    <FormSection title={t('settings.marketing_group.stamp_wizard.step1_section')} icon={FileText}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.marketing_group.stamp_wizard.step1_desc')}
      </p>

      <div className="space-y-5">
        <FormField label={t('settings.marketing_group.field.name')} required>
          <input
            type="text"
            value={state.name}
            onChange={(e) => updateState({ name: e.target.value })}
            className={inputClass}
            placeholder="coffee_stamp"
          />
          <p className="mt-1 text-xs text-gray-500">
            {t('settings.marketing_group.stamp_wizard.name_hint')}
          </p>
        </FormField>

        <FormField label={t('settings.marketing_group.field.display_name')} required>
          <input
            type="text"
            value={state.display_name}
            onChange={(e) => updateState({ display_name: e.target.value })}
            className={inputClass}
            placeholder={t('settings.marketing_group.stamp.display_name_placeholder')}
          />
        </FormField>
      </div>
    </FormSection>
  );
};
