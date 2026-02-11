import React from 'react';
import { Settings, RefreshCw } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { StampWizardState } from './index';
import { FormSection, FormField, inputClass } from '@/shared/components/FormField';

interface Step2ConfigProps {
  state: StampWizardState;
  updateState: (updates: Partial<StampWizardState>) => void;
}

export const Step2Config: React.FC<Step2ConfigProps> = ({ state, updateState }) => {
  const { t } = useI18n();

  return (
    <FormSection title={t('settings.marketing_group.stamp_wizard.step2_section')} icon={Settings}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.marketing_group.stamp_wizard.step2_desc')}
      </p>

      <div className="space-y-6">
        {/* Stamps Required */}
        <FormField label={t('settings.marketing_group.stamp.stamps_required')} required>
          <input
            type="number"
            value={state.stamps_required}
            onChange={(e) => updateState({ stamps_required: Number(e.target.value) || 0 })}
            min={1}
            className={inputClass}
          />
          <p className="mt-1 text-xs text-gray-500">
            {t('settings.marketing_group.stamp_wizard.stamps_required_hint')}
          </p>
        </FormField>

        {/* Reward Quantity */}
        <FormField label={t('settings.marketing_group.stamp.reward_quantity')} required>
          <input
            type="number"
            value={state.reward_quantity}
            onChange={(e) => updateState({ reward_quantity: Number(e.target.value) || 0 })}
            min={1}
            className={inputClass}
          />
          <p className="mt-1 text-xs text-gray-500">
            {t('settings.marketing_group.stamp_wizard.reward_quantity_hint')}
          </p>
        </FormField>

        {/* Cyclic Toggle */}
        <div className="flex items-center justify-between p-4 bg-gray-50 rounded-xl border border-gray-100">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 rounded-lg bg-teal-100 flex items-center justify-center">
              <RefreshCw size={20} className="text-teal-600" />
            </div>
            <div>
              <span className="text-sm font-medium text-gray-700 block">
                {t('settings.marketing_group.stamp.is_cyclic')}
              </span>
              <p className="text-xs text-gray-400 mt-0.5">
                {t('settings.marketing_group.stamp.cyclic_hint')}
              </p>
            </div>
          </div>
          <button
            type="button"
            onClick={() => updateState({ is_cyclic: !state.is_cyclic })}
            className={`px-4 py-2 rounded-full text-sm font-medium transition-colors ${
              state.is_cyclic
                ? 'bg-teal-100 text-teal-700'
                : 'bg-gray-200 text-gray-500'
            }`}
          >
            {state.is_cyclic ? t('common.status.enabled') : t('common.status.disabled')}
          </button>
        </div>

        {/* Preview */}
        <div className="p-4 bg-violet-50 rounded-xl border border-violet-100">
          <p className="text-sm text-violet-700">
            <span className="font-medium">{t('settings.price_rule.wizard.preview')}: </span>
            {t('settings.marketing_group.stamp_wizard.config_preview', {
              stamps: state.stamps_required,
              reward: state.reward_quantity,
              cyclic: state.is_cyclic ? t('settings.marketing_group.stamp.cyclic') : t('settings.marketing_group.stamp_wizard.one_time'),
            })}
          </p>
        </div>
      </div>
    </FormSection>
  );
};
