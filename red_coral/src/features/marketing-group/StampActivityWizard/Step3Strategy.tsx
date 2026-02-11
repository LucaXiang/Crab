import React from 'react';
import { Award, Coins, Crown, Package } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { StampWizardState } from './index';
import type { RewardStrategy } from '@/core/domain/types/api';
import { FormSection, FormField, selectClass } from '@/shared/components/FormField';
import { useProductStore } from '@/core/stores/resources';

interface Step3StrategyProps {
  state: StampWizardState;
  updateState: (updates: Partial<StampWizardState>) => void;
}

const strategyOptions: { value: RewardStrategy; icon: typeof Coins; variant: 'green' | 'amber' | 'blue' }[] = [
  { value: 'ECONOMIZADOR', icon: Coins, variant: 'green' },
  { value: 'GENEROSO', icon: Crown, variant: 'amber' },
  { value: 'DESIGNATED', icon: Package, variant: 'blue' },
];

const variantStyles = {
  green: {
    border: 'border-green-500', bg: 'bg-green-50', ring: 'ring-green-100',
    iconBg: 'bg-green-100', iconText: 'text-green-600', titleText: 'text-green-700',
    checkBg: 'bg-green-500',
  },
  amber: {
    border: 'border-amber-500', bg: 'bg-amber-50', ring: 'ring-amber-100',
    iconBg: 'bg-amber-100', iconText: 'text-amber-600', titleText: 'text-amber-700',
    checkBg: 'bg-amber-500',
  },
  blue: {
    border: 'border-blue-500', bg: 'bg-blue-50', ring: 'ring-blue-100',
    iconBg: 'bg-blue-100', iconText: 'text-blue-600', titleText: 'text-blue-700',
    checkBg: 'bg-blue-500',
  },
};

export const Step3Strategy: React.FC<Step3StrategyProps> = ({ state, updateState }) => {
  const { t } = useI18n();
  const products = useProductStore((s) => s.items);

  return (
    <FormSection title={t('settings.marketing_group.stamp_wizard.step3_section')} icon={Award}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.marketing_group.stamp_wizard.step3_desc')}
      </p>

      {/* Strategy Cards */}
      <div className="grid grid-cols-3 gap-4 mb-6">
        {strategyOptions.map(({ value, icon: Icon, variant }) => {
          const styles = variantStyles[variant];
          const isSelected = state.reward_strategy === value;
          return (
            <button
              key={value}
              type="button"
              onClick={() => updateState({ reward_strategy: value })}
              className={`relative flex flex-col items-center p-5 rounded-xl border-2 transition-all ${
                isSelected
                  ? `${styles.border} ${styles.bg} ring-4 ${styles.ring}`
                  : 'border-gray-200 bg-white hover:border-gray-300 hover:bg-gray-50'
              }`}
            >
              <div className={`w-14 h-14 rounded-full flex items-center justify-center mb-3 ${
                isSelected ? styles.iconBg : 'bg-gray-100'
              }`}>
                <Icon size={28} className={isSelected ? styles.iconText : 'text-gray-400'} />
              </div>
              <h4 className={`text-sm font-bold mb-1 ${isSelected ? styles.titleText : 'text-gray-700'}`}>
                {t(`settings.marketing_group.stamp.strategy.${value.toLowerCase()}`)}
              </h4>
              <p className="text-xs text-gray-500 text-center">
                {t(`settings.marketing_group.stamp_wizard.strategy_${value.toLowerCase()}_desc`)}
              </p>
              {isSelected && (
                <div className={`absolute top-2 right-2 w-5 h-5 rounded-full ${styles.checkBg} flex items-center justify-center`}>
                  <svg className="w-3 h-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                  </svg>
                </div>
              )}
            </button>
          );
        })}
      </div>

      {/* Designated Product Selector */}
      {state.reward_strategy === 'DESIGNATED' && (
        <FormField label={t('settings.marketing_group.stamp.designated_product')} required>
          <select
            value={state.designated_product_id ?? ''}
            onChange={(e) => updateState({ designated_product_id: e.target.value ? Number(e.target.value) : null })}
            className={selectClass}
          >
            <option value="">{t('common.hint.select')}</option>
            {products.map((p) => (
              <option key={p.id} value={p.id}>{p.name}</option>
            ))}
          </select>
        </FormField>
      )}
    </FormSection>
  );
};
