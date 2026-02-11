import React, { useState, useCallback } from 'react';
import { X, ChevronLeft, ChevronRight, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import type {
  StampActivityCreate,
  StampTargetInput,
  RewardStrategy,
} from '@/core/domain/types/api';
import { createStampActivity } from '../mutations';
import { WizardProgress } from '@/features/price-rule/PriceRuleWizard/WizardProgress';
import { Step1Naming } from './Step1Naming';
import { Step2Config } from './Step2Config';
import { Step3Strategy } from './Step3Strategy';
import { Step4Targets } from './Step4Targets';

export interface StampWizardState {
  name: string;
  display_name: string;
  stamps_required: number;
  reward_quantity: number;
  is_cyclic: boolean;
  reward_strategy: RewardStrategy;
  designated_product_id: number | null;
  stamp_targets: StampTargetInput[];
  reward_targets: StampTargetInput[];
}

const INITIAL_STATE: StampWizardState = {
  name: '',
  display_name: '',
  stamps_required: 10,
  reward_quantity: 1,
  is_cyclic: true,
  reward_strategy: 'ECONOMIZADOR',
  designated_product_id: null,
  stamp_targets: [],
  reward_targets: [],
};

interface StampActivityWizardProps {
  isOpen: boolean;
  groupId: number;
  onClose: () => void;
  onSuccess: () => void;
}

export const StampActivityWizard: React.FC<StampActivityWizardProps> = ({
  isOpen,
  groupId,
  onClose,
  onSuccess,
}) => {
  const { t } = useI18n();
  const [currentStep, setCurrentStep] = useState(1);
  const [state, setState] = useState<StampWizardState>(() => ({ ...INITIAL_STATE }));
  const [saving, setSaving] = useState(false);

  const totalSteps = 4;

  const updateState = useCallback((updates: Partial<StampWizardState>) => {
    setState((prev) => ({ ...prev, ...updates }));
  }, []);

  const canProceed = useCallback((): boolean => {
    switch (currentStep) {
      case 1:
        return !!state.name.trim() && !!state.display_name.trim();
      case 2:
        return state.stamps_required > 0 && state.reward_quantity > 0;
      case 3:
        return state.reward_strategy !== 'DESIGNATED' || state.designated_product_id != null;
      case 4:
        return state.stamp_targets.length > 0;
      default:
        return true;
    }
  }, [currentStep, state]);

  const handleNext = () => {
    if (canProceed() && currentStep < totalSteps) {
      setCurrentStep((prev) => prev + 1);
    }
  };

  const handlePrev = () => {
    if (currentStep > 1) {
      setCurrentStep((prev) => prev - 1);
    }
  };

  const handleSave = async () => {
    if (!canProceed()) return;

    setSaving(true);
    try {
      const payload: StampActivityCreate = {
        name: state.name.trim(),
        display_name: state.display_name.trim(),
        stamps_required: state.stamps_required,
        reward_quantity: state.reward_quantity,
        reward_strategy: state.reward_strategy,
        designated_product_id: state.reward_strategy === 'DESIGNATED' ? state.designated_product_id : null,
        is_cyclic: state.is_cyclic,
        stamp_targets: state.stamp_targets,
        reward_targets: state.reward_strategy === 'DESIGNATED' ? [] : state.reward_targets,
      };

      await createStampActivity(groupId, payload);
      toast.success(t('settings.marketing_group.message.stamp_created'));
      onSuccess();
    } catch (e) {
      logger.error('Failed to create stamp activity', e);
      toast.error(t('common.message.save_failed'));
    } finally {
      setSaving(false);
    }
  };

  if (!isOpen) return null;

  const stepTitles = [
    t('settings.marketing_group.stamp_wizard.step1_title'),
    t('settings.marketing_group.stamp_wizard.step2_title'),
    t('settings.marketing_group.stamp_wizard.step3_title'),
    t('settings.marketing_group.stamp_wizard.step4_title'),
  ];

  return (
    <div className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[90vh] overflow-hidden animate-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="shrink-0 px-6 py-4 border-b border-gray-100 bg-gradient-to-r from-teal-50 to-white">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-bold text-gray-900">
              {t('settings.marketing_group.add_stamp')}
            </h2>
            <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-xl transition-colors">
              <X size={18} className="text-gray-500" />
            </button>
          </div>
          <WizardProgress currentStep={currentStep} totalSteps={totalSteps} titles={stepTitles} />
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">
          {currentStep === 1 && <Step1Naming state={state} updateState={updateState} />}
          {currentStep === 2 && <Step2Config state={state} updateState={updateState} />}
          {currentStep === 3 && <Step3Strategy state={state} updateState={updateState} />}
          {currentStep === 4 && <Step4Targets state={state} updateState={updateState} />}
        </div>

        {/* Footer */}
        <div className="shrink-0 px-6 py-4 border-t border-gray-100 bg-gray-50/50 flex justify-between">
          <button
            onClick={handlePrev}
            disabled={currentStep === 1}
            className="flex items-center gap-1 px-4 py-2.5 text-gray-600 hover:bg-gray-100 rounded-xl text-sm font-medium transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
          >
            <ChevronLeft size={18} />
            {t('settings.price_rule.wizard.prev')}
          </button>

          <div className="flex gap-3">
            {currentStep === totalSteps ? (
              <button
                onClick={handleSave}
                disabled={saving || !canProceed()}
                className="flex items-center gap-2 px-5 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-semibold hover:bg-teal-700 transition-colors shadow-lg shadow-teal-600/20 disabled:opacity-50"
              >
                {saving ? (
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                ) : (
                  <Check size={18} />
                )}
                {t('settings.price_rule.wizard.finish')}
              </button>
            ) : (
              <button
                onClick={handleNext}
                disabled={!canProceed()}
                className="flex items-center gap-1 px-5 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-semibold hover:bg-teal-700 transition-colors shadow-lg shadow-teal-600/20 disabled:opacity-50"
              >
                {t('settings.price_rule.wizard.next')}
                <ChevronRight size={18} />
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
