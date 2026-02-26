import React, { useState, useCallback } from 'react';
import { X, ChevronLeft, ChevronRight, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import type { PriceRule, PriceRuleCreate } from '@/core/domain/types';

// Step components
import { Step1RuleType } from './Step1RuleType';
import { Step2Adjustment } from './Step2Adjustment';
import { Step3Scope } from './Step3Scope';
import { Step4Time } from './Step4Time';
import { Step5Naming } from './Step5Naming';
import { Step6Advanced } from './Step6Advanced';
import { WizardProgress } from './WizardProgress';

const getApi = () => createTauriClient();

/** 将 Unix millis 转为 datetime-local input 所需的本地时间字符串 "YYYY-MM-DDTHH:mm" */
function toLocalDatetimeString(millis: number): string {
  const d = new Date(millis);
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

export interface WizardState {
  // Step 1
  rule_type: 'DISCOUNT' | 'SURCHARGE';
  // Step 2
  adjustment_type: 'PERCENTAGE' | 'FIXED_AMOUNT';
  adjustment_value: number;
  // Step 3
  product_scope: 'GLOBAL' | 'CATEGORY' | 'TAG' | 'PRODUCT';
  target_id: number | null;
  /** Zone scope: "all", "retail", or specific zone ID */
  zone_scope: string;
  // Step 4 - UI-only mode for form control
  time_mode: 'ALWAYS' | 'SCHEDULE' | 'ONETIME';
  // Step 4 - SCHEDULE mode fields
  active_days: number[];
  active_start_time: string;
  active_end_time: string;
  // Step 4 - ONETIME mode fields (ISO datetime string for input)
  valid_from: string;
  valid_until: string;
  // Step 5
  name: string;
  receipt_name: string;
  description: string;
  // Step 6
  is_stackable: boolean;
  is_exclusive: boolean;
}

const getInitialState = (rule?: PriceRule | null): WizardState => {
  if (rule) {
    // Determine time_mode based on which fields are set
    let time_mode: WizardState['time_mode'] = 'ALWAYS';
    if (rule.valid_from || rule.valid_until) {
      time_mode = 'ONETIME';
    } else if (rule.active_days?.length || rule.active_start_time || rule.active_end_time) {
      time_mode = 'SCHEDULE';
    }

    return {
      rule_type: rule.rule_type as 'DISCOUNT' | 'SURCHARGE',
      adjustment_type: rule.adjustment_type as 'PERCENTAGE' | 'FIXED_AMOUNT',
      adjustment_value: rule.adjustment_value,
      product_scope: rule.product_scope as WizardState['product_scope'],
      target_id: rule.target_id,
      zone_scope: rule.zone_scope,
      time_mode,
      active_days: rule.active_days || [1, 2, 3, 4, 5],
      active_start_time: rule.active_start_time || '09:00',
      active_end_time: rule.active_end_time || '18:00',
      valid_from: rule.valid_from ? toLocalDatetimeString(rule.valid_from) : '',
      valid_until: rule.valid_until ? toLocalDatetimeString(rule.valid_until) : '',
      name: rule.name,
      receipt_name: rule.receipt_name ?? '',
      description: rule.description || '',
      is_stackable: rule.is_stackable,
      is_exclusive: rule.is_exclusive,
    };
  }
  return {
    rule_type: 'DISCOUNT',
    adjustment_type: 'PERCENTAGE',
    adjustment_value: 10,
    product_scope: 'GLOBAL',
    target_id: null,
    zone_scope: 'all',
    time_mode: 'ALWAYS',
    active_days: [1, 2, 3, 4, 5],
    active_start_time: '09:00',
    active_end_time: '18:00',
    valid_from: '',
    valid_until: '',
    name: '',
    receipt_name: '',
    description: '',
    is_stackable: true,
    is_exclusive: false,
  };
};

interface PriceRuleWizardProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: () => void;
  editingRule?: PriceRule | null;
}

export const PriceRuleWizard: React.FC<PriceRuleWizardProps> = ({
  isOpen,
  onClose,
  onSuccess,
  editingRule,
}) => {
  const { t } = useI18n();
  const [currentStep, setCurrentStep] = useState(1);
  const [state, setState] = useState<WizardState>(() => getInitialState(editingRule));
  const [saving, setSaving] = useState(false);

  const totalSteps = 6;
  const isEditing = !!editingRule;

  const updateState = useCallback((updates: Partial<WizardState>) => {
    setState((prev) => ({ ...prev, ...updates }));
  }, []);

  React.useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onClose]);

  const canProceed = useCallback((): boolean => {
    switch (currentStep) {
      case 1:
        return !!state.rule_type;
      case 2:
        return state.adjustment_value > 0 && (state.adjustment_type !== 'PERCENTAGE' || state.adjustment_value <= 100);
      case 3:
        return state.product_scope === 'GLOBAL' || state.target_id != null;
      case 4:
        if (state.time_mode === 'SCHEDULE') {
          return state.active_days.length > 0 && !!state.active_start_time && !!state.active_end_time;
        }
        if (state.time_mode === 'ONETIME') {
          return !!state.valid_from && !!state.valid_until;
        }
        return true;
      case 5:
        return !!state.name.trim();
      case 6:
        return true;
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

  const buildPayload = (): PriceRuleCreate => {
    const payload: PriceRuleCreate = {
      name: state.name.trim(),
      receipt_name: state.receipt_name.trim() || undefined,
      description: state.description.trim() || undefined,
      rule_type: state.rule_type,
      product_scope: state.product_scope,
      target_id: state.target_id ?? undefined,
      zone_scope: state.zone_scope,
      adjustment_type: state.adjustment_type,
      adjustment_value: state.adjustment_value,
      is_stackable: state.is_stackable,
      is_exclusive: state.is_exclusive,
    };

    if (state.time_mode === 'SCHEDULE') {
      payload.active_days = state.active_days;
      payload.active_start_time = state.active_start_time;
      payload.active_end_time = state.active_end_time;
    } else if (state.time_mode === 'ONETIME') {
      // Convert local datetime string to Unix millis for backend
      payload.valid_from = state.valid_from ? new Date(state.valid_from).getTime() : undefined;
      payload.valid_until = state.valid_until ? new Date(state.valid_until).getTime() : undefined;
    }

    return payload;
  };

  const handleSave = async () => {
    if (!canProceed()) return;

    setSaving(true);
    try {
      const payload = buildPayload();

      if (isEditing && editingRule?.id) {
        await getApi().updatePriceRule(editingRule.id, payload);
        toast.success(t('settings.price_rule.message.updated'));
      } else {
        await getApi().createPriceRule(payload);
        toast.success(t('settings.price_rule.message.created'));
      }
      onSuccess();
    } catch (e) {
      logger.error('Failed to save price rule', e);
      toast.error(t('common.message.save_failed'));
    } finally {
      setSaving(false);
    }
  };

  if (!isOpen) return null;

  const stepTitles = [
    t('settings.price_rule.wizard.step1_title'),
    t('settings.price_rule.wizard.step2_title'),
    t('settings.price_rule.wizard.step3_title'),
    t('settings.price_rule.wizard.step4_title'),
    t('settings.price_rule.wizard.step5_title'),
    t('settings.price_rule.wizard.step6_title'),
  ];

  return (
    <div
      className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4"
    >
      <div
        className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[90vh] overflow-hidden animate-in zoom-in-95 duration-200"
      >
        {/* Header */}
        <div className="shrink-0 px-6 py-4 border-b border-gray-100 bg-gradient-to-r from-teal-50 to-white">
          <div className="flex items-center justify-between mb-4">
            <h2 className="text-lg font-bold text-gray-900">
              {isEditing ? t('settings.price_rule.edit_rule') : t('settings.price_rule.add_rule')}
            </h2>
            <button
              onClick={onClose}
              className="p-2 hover:bg-gray-100 rounded-xl transition-colors"
            >
              <X size={18} className="text-gray-500" />
            </button>
          </div>
          <WizardProgress currentStep={currentStep} totalSteps={totalSteps} titles={stepTitles} />
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">
          {currentStep === 1 && <Step1RuleType state={state} updateState={updateState} />}
          {currentStep === 2 && <Step2Adjustment state={state} updateState={updateState} />}
          {currentStep === 3 && <Step3Scope state={state} updateState={updateState} />}
          {currentStep === 4 && <Step4Time state={state} updateState={updateState} />}
          {currentStep === 5 && <Step5Naming state={state} updateState={updateState} />}
          {currentStep === 6 && <Step6Advanced state={state} updateState={updateState} />}
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
                {isEditing ? t('settings.price_rule.wizard.save') : t('settings.price_rule.wizard.finish')}
              </button>
            ) : currentStep === 5 ? (
              <>
                <button
                  onClick={handleSave}
                  disabled={saving || !canProceed()}
                  className="px-5 py-2.5 bg-white border border-gray-200 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-50 transition-colors disabled:opacity-50"
                >
                  {t('settings.price_rule.wizard.finish')}
                </button>
                <button
                  onClick={handleNext}
                  disabled={!canProceed()}
                  className="flex items-center gap-1 px-5 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-semibold hover:bg-teal-700 transition-colors shadow-lg shadow-teal-600/20 disabled:opacity-50"
                >
                  {t('settings.price_rule.wizard.next')}
                  <ChevronRight size={18} />
                </button>
              </>
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
