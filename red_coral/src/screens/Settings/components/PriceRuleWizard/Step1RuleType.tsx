import React from 'react';
import { Percent, Plus } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { WizardState } from './index';
import { FormSection } from '../../forms/FormField';

interface TypeCardProps {
  icon: React.ReactNode;
  title: string;
  description: string;
  selected: boolean;
  onClick: () => void;
  color: string;
}

const TypeCard: React.FC<TypeCardProps> = ({
  icon,
  title,
  description,
  selected,
  onClick,
  color,
}) => (
  <button
    type="button"
    onClick={onClick}
    className={`relative flex flex-col items-center p-6 rounded-xl border-2 transition-all ${
      selected
        ? `border-${color}-500 bg-${color}-50 ring-4 ring-${color}-100`
        : 'border-gray-200 bg-white hover:border-gray-300 hover:bg-gray-50'
    }`}
  >
    <div
      className={`w-16 h-16 rounded-full flex items-center justify-center mb-4 ${
        selected ? `bg-${color}-100` : 'bg-gray-100'
      }`}
    >
      <span className={selected ? `text-${color}-600` : 'text-gray-400'}>{icon}</span>
    </div>
    <h4 className={`text-lg font-bold mb-1 ${selected ? `text-${color}-700` : 'text-gray-700'}`}>
      {title}
    </h4>
    <p className="text-sm text-gray-500 text-center">{description}</p>
    {selected && (
      <div className={`absolute top-3 right-3 w-6 h-6 rounded-full bg-${color}-500 flex items-center justify-center`}>
        <svg className="w-4 h-4 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
        </svg>
      </div>
    )}
  </button>
);

interface Step1RuleTypeProps {
  state: WizardState;
  updateState: (updates: Partial<WizardState>) => void;
}

export const Step1RuleType: React.FC<Step1RuleTypeProps> = ({ state, updateState }) => {
  const { t } = useI18n();

  return (
    <FormSection title={t('settings.price_rule.wizard.step1_section')} icon={Percent}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.price_rule.wizard.step1_desc')}
      </p>

      <div className="grid grid-cols-2 gap-4">
        <TypeCard
          icon={<Percent size={32} />}
          title={t('settings.price_rule.type.discount')}
          description={t('settings.price_rule.wizard.discount_desc')}
          selected={state.rule_type === 'DISCOUNT'}
          onClick={() => updateState({ rule_type: 'DISCOUNT' })}
          color="green"
        />
        <TypeCard
          icon={<Plus size={32} />}
          title={t('settings.price_rule.type.surcharge')}
          description={t('settings.price_rule.wizard.surcharge_desc')}
          selected={state.rule_type === 'SURCHARGE'}
          onClick={() => updateState({ rule_type: 'SURCHARGE' })}
          color="red"
        />
      </div>
    </FormSection>
  );
};
