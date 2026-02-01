import React from 'react';
import { Settings2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { WizardState } from './index';
import { FormSection, FormField, CheckboxField, inputClass } from '@/shared/components/FormField';

interface Step6AdvancedProps {
  state: WizardState;
  updateState: (updates: Partial<WizardState>) => void;
}

export const Step6Advanced: React.FC<Step6AdvancedProps> = ({ state, updateState }) => {
  const { t } = useI18n();

  return (
    <FormSection title={t('settings.price_rule.wizard.step6_section')} icon={Settings2}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.price_rule.wizard.step6_desc')}
      </p>

      <div className="space-y-5">
        <CheckboxField
          id="is_stackable"
          label={t('settings.price_rule.form.is_stackable')}
          description={t('settings.price_rule.wizard.stackable_hint')}
          checked={state.is_stackable}
          onChange={(checked) => updateState({ is_stackable: checked })}
        />

        <CheckboxField
          id="is_exclusive"
          label={t('settings.price_rule.form.is_exclusive')}
          description={t('settings.price_rule.wizard.exclusive_hint')}
          checked={state.is_exclusive}
          onChange={(checked) => updateState({ is_exclusive: checked })}
        />

        {/* Info box */}
        <div className="p-4 bg-amber-50 rounded-xl border border-amber-100">
          <p className="text-sm text-amber-800">
            <strong>{t('common.hint.tip')}:</strong> {t('settings.price_rule.wizard.advanced_tip')}
          </p>
        </div>
      </div>
    </FormSection>
  );
};
