import React, { useState, useCallback } from 'react';
import { X, ChevronLeft, ChevronRight, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import type { MgDiscountRule, MgDiscountRuleCreate, ProductScope, AdjustmentType } from '@/core/domain/types/api';
import { createDiscountRule, updateDiscountRule } from '../mutations';
import { WizardProgress } from '@/features/price-rule/PriceRuleWizard/WizardProgress';
import { Step1Adjustment } from './Step1Adjustment';
import { Step2Scope } from './Step2Scope';
import { Step3Naming } from './Step3Naming';

export interface RuleWizardState {
  adjustment_type: AdjustmentType;
  adjustment_value: number;
  product_scope: ProductScope;
  target_id: number | null;
  name: string;
  receipt_name: string;
}

const getInitialState = (rule?: MgDiscountRule | null): RuleWizardState => {
  if (rule) {
    return {
      adjustment_type: rule.adjustment_type,
      adjustment_value: rule.adjustment_value,
      product_scope: rule.product_scope,
      target_id: rule.target_id ?? null,
      name: rule.name,
      receipt_name: rule.receipt_name ?? '',
    };
  }
  return {
    adjustment_type: 'PERCENTAGE',
    adjustment_value: 10,
    product_scope: 'GLOBAL',
    target_id: null,
    name: '',
    receipt_name: '',
  };
};

interface DiscountRuleWizardProps {
  isOpen: boolean;
  groupId: number;
  onClose: () => void;
  onSuccess: () => void;
  editingRule?: MgDiscountRule | null;
}

export const DiscountRuleWizard: React.FC<DiscountRuleWizardProps> = ({
  isOpen,
  groupId,
  onClose,
  onSuccess,
  editingRule,
}) => {
  const { t } = useI18n();
  const [currentStep, setCurrentStep] = useState(1);
  const [state, setState] = useState<RuleWizardState>(() => getInitialState(editingRule));
  const [saving, setSaving] = useState(false);

  const totalSteps = 3;
  const isEditing = !!editingRule;

  const updateState = useCallback((updates: Partial<RuleWizardState>) => {
    setState((prev) => ({ ...prev, ...updates }));
  }, []);

  const canProceed = useCallback((): boolean => {
    switch (currentStep) {
      case 1:
        return state.adjustment_value > 0 && (state.adjustment_type !== 'PERCENTAGE' || state.adjustment_value <= 100);
      case 2:
        return state.product_scope === 'GLOBAL' || state.target_id != null;
      case 3:
        return !!state.name.trim();
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
      const payload: MgDiscountRuleCreate = {
        name: state.name.trim(),
        receipt_name: state.receipt_name.trim() || undefined,
        product_scope: state.product_scope,
        target_id: state.product_scope === 'GLOBAL' ? null : state.target_id,
        adjustment_type: state.adjustment_type,
        adjustment_value: state.adjustment_value,
      };

      if (isEditing && editingRule) {
        await updateDiscountRule(groupId, editingRule.id, payload);
        toast.success(t('settings.marketing_group.message.rule_updated'));
      } else {
        await createDiscountRule(groupId, payload);
        toast.success(t('settings.marketing_group.message.rule_created'));
      }
      onSuccess();
    } catch (e) {
      logger.error('Failed to save MG discount rule', e);
      toast.error(t('common.message.save_failed'));
    } finally {
      setSaving(false);
    }
  };

  if (!isOpen) return null;

  const stepTitles = [
    t('settings.marketing_group.rule_wizard.step1_title'),
    t('settings.marketing_group.rule_wizard.step2_title'),
    t('settings.marketing_group.rule_wizard.step3_title'),
  ];

  return (
    <div className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[90vh] overflow-hidden animate-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="shrink-0 px-6 py-4 border-b border-gray-100 bg-gradient-to-r from-violet-50 to-white">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-bold text-gray-900">
              {isEditing ? t('settings.marketing_group.edit_rule') : t('settings.marketing_group.add_rule')}
            </h2>
            <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-xl transition-colors">
              <X size={18} className="text-gray-500" />
            </button>
          </div>
          <WizardProgress currentStep={currentStep} totalSteps={totalSteps} titles={stepTitles} />
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">
          {currentStep === 1 && <Step1Adjustment state={state} updateState={updateState} />}
          {currentStep === 2 && <Step2Scope state={state} updateState={updateState} />}
          {currentStep === 3 && <Step3Naming state={state} updateState={updateState} />}
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
                className="flex items-center gap-2 px-5 py-2.5 bg-violet-600 text-white rounded-xl text-sm font-semibold hover:bg-violet-700 transition-colors shadow-lg shadow-violet-600/20 disabled:opacity-50"
              >
                {saving ? (
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                ) : (
                  <Check size={18} />
                )}
                {isEditing ? t('settings.price_rule.wizard.save') : t('settings.price_rule.wizard.finish')}
              </button>
            ) : (
              <button
                onClick={handleNext}
                disabled={!canProceed()}
                className="flex items-center gap-1 px-5 py-2.5 bg-violet-600 text-white rounded-xl text-sm font-semibold hover:bg-violet-700 transition-colors shadow-lg shadow-violet-600/20 disabled:opacity-50"
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
