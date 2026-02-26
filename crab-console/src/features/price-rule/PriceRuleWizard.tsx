import React, { useState, useMemo } from 'react';
import { Tag, Clock, Settings2, FileText, Percent } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { Wizard, WizardStep } from '@/shared/components/Wizard';
import { FormField, CheckboxField, inputClass } from '@/shared/components/FormField';
import { SelectField } from '@/shared/components/FormField/SelectField';
import type { PriceRuleCreate, RuleType, ProductScope, AdjustmentType } from '@/core/types/store';

interface PriceRuleWizardProps {
  onFinish: (data: PriceRuleCreate) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

const DAY_INDICES = [1, 2, 3, 4, 5, 6, 0]; // Mon-Sun

export const PriceRuleWizard: React.FC<PriceRuleWizardProps> = ({ onFinish, onCancel, isSubmitting }) => {
  const { t } = useI18n();

  // ── Form State ──
  const [name, setName] = useState('');
  const [displayName, setDisplayName] = useState('');
  const [receiptName, setReceiptName] = useState('');
  const [description, setDescription] = useState('');
  
  const [ruleType, setRuleType] = useState<RuleType>('DISCOUNT');
  const [adjustmentType, setAdjustmentType] = useState<AdjustmentType>('PERCENTAGE');
  const [adjustmentValue, setAdjustmentValue] = useState<number>(0);

  const [productScope, setProductScope] = useState<ProductScope>('GLOBAL');
  const [targetId, setTargetId] = useState('');
  const [zoneScope, setZoneScope] = useState('all');

  const [activeDays, setActiveDays] = useState<number[]>([]);
  const [activeStartTime, setActiveStartTime] = useState('');
  const [activeEndTime, setActiveEndTime] = useState('');
  const [validFrom, setValidFrom] = useState('');
  const [validUntil, setValidUntil] = useState('');
  
  const [isStackable, setIsStackable] = useState(false);
  const [isExclusive, setIsExclusive] = useState(false);

  // ── Options ──
  const ruleTypeOptions = useMemo(() => [
    { value: 'DISCOUNT' as RuleType, label: t('settings.price_rule.type_discount') },
    { value: 'SURCHARGE' as RuleType, label: t('settings.price_rule.type_surcharge') },
  ], [t]);

  const adjustmentTypeOptions = useMemo(() => [
    { value: 'PERCENTAGE' as AdjustmentType, label: t('settings.price_rule.adjustment_percentage') },
    { value: 'FIXED_AMOUNT' as AdjustmentType, label: t('settings.price_rule.adjustment_fixed') },
  ], [t]);

  const productScopeOptions = useMemo(() => [
    { value: 'GLOBAL' as ProductScope, label: t('settings.price_rule.scope_global') },
    { value: 'CATEGORY' as ProductScope, label: t('settings.price_rule.scope_category') },
    { value: 'TAG' as ProductScope, label: t('settings.price_rule.scope_tag') },
    { value: 'PRODUCT' as ProductScope, label: t('settings.price_rule.scope_product') },
  ], [t]);

  // ── Helpers ──
  const toggleDay = (day: number) => {
    setActiveDays(prev => prev.includes(day) ? prev.filter(d => d !== day) : [...prev, day]);
  };

  const dateStrToTs = (s: string): number | undefined => {
    if (!s) return undefined;
    return new Date(s + 'T00:00:00Z').getTime();
  };

  const handleFinish = () => {
    onFinish({
      name: name.trim(),
      display_name: displayName.trim(),
      receipt_name: receiptName.trim(),
      description: description.trim() || undefined,
      rule_type: ruleType,
      product_scope: productScope,
      target_id: productScope !== 'GLOBAL' && targetId ? Number(targetId) : undefined,
      zone_scope: zoneScope !== 'all' ? zoneScope : undefined,
      adjustment_type: adjustmentType,
      adjustment_value: adjustmentValue,
      is_stackable: isStackable,
      is_exclusive: isExclusive,
      active_days: activeDays.length > 0 ? activeDays : undefined,
      active_start_time: activeStartTime || undefined,
      active_end_time: activeEndTime || undefined,
      valid_from: dateStrToTs(validFrom),
      valid_until: dateStrToTs(validUntil),
    });
  };

  // ── Steps ──

  const step1: WizardStep = {
    id: 'basics',
    title: t('settings.price_rule.step_basics'),
    description: t('settings.price_rule.step_basics_desc'),
    isValid: !!name.trim() && !!displayName.trim() && !!receiptName.trim(),
    component: (
      <div className="space-y-4">
        <FormField label={t('settings.price_rule.name')} required>
          <input value={name} onChange={e => setName(e.target.value)} className={inputClass} autoFocus placeholder="e.g. Happy Hour" />
        </FormField>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <FormField label={t('settings.price_rule.display_name')} required>
            <input value={displayName} onChange={e => setDisplayName(e.target.value)} className={inputClass} placeholder="e.g. -10% Off" />
          </FormField>
          <FormField label={t('settings.price_rule.receipt_name')} required>
            <input value={receiptName} onChange={e => setReceiptName(e.target.value)} className={inputClass} placeholder="e.g. DISC-HH" />
          </FormField>
        </div>
        <FormField label={t('settings.price_rule.description')}>
          <textarea value={description} onChange={e => setDescription(e.target.value)} className={`${inputClass} resize-none`} rows={3} placeholder="Optional description..." />
        </FormField>
      </div>
    ),
  };

  const step2: WizardStep = {
    id: 'config',
    title: t('settings.price_rule.step_config'),
    description: t('settings.price_rule.step_config_desc'),
    isValid: adjustmentValue >= 0,
    component: (
      <div className="space-y-6">
        <SelectField 
          label={t('settings.price_rule.type')} 
          value={ruleType} 
          onChange={v => setRuleType(v as RuleType)} 
          options={ruleTypeOptions} 
          required 
        />
        
        <div className="p-4 bg-slate-50 rounded-xl border border-slate-200 space-y-4">
          <SelectField 
            label={t('settings.price_rule.adjustment_type')} 
            value={adjustmentType} 
            onChange={v => setAdjustmentType(v as AdjustmentType)} 
            options={adjustmentTypeOptions} 
            required 
          />
          <FormField label={t('settings.price_rule.adjustment_value')} required>
            <div className="relative">
              <input 
                type="number" 
                value={adjustmentValue} 
                onChange={e => setAdjustmentValue(Number(e.target.value))} 
                className={`${inputClass} pr-12`} 
                step={adjustmentType === 'PERCENTAGE' ? '1' : '0.01'} 
                min={0} 
              />
              <div className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 font-medium">
                {adjustmentType === 'PERCENTAGE' ? '%' : '€'}
              </div>
            </div>
          </FormField>
        </div>
      </div>
    ),
  };

  const step3: WizardStep = {
    id: 'scope',
    title: t('settings.price_rule.step_scope'),
    description: t('settings.price_rule.step_scope_desc'),
    isValid: productScope === 'GLOBAL' || !!targetId,
    component: (
      <div className="space-y-6">
        <SelectField 
          label={t('settings.price_rule.scope')} 
          value={productScope} 
          onChange={v => setProductScope(v as ProductScope)} 
          options={productScopeOptions} 
          required 
        />
        
        {productScope !== 'GLOBAL' && (
          <div className="animate-in fade-in slide-in-from-top-2 duration-200">
            <FormField label={t('settings.price_rule.target_id')} required>
              <input 
                type="number" 
                value={targetId} 
                onChange={e => setTargetId(e.target.value)} 
                className={inputClass} 
                placeholder={t('settings.price_rule.target_id_placeholder')} 
              />
              <p className="text-xs text-slate-400 mt-1.5">
                {t('settings.price_rule.target_id_hint')}
              </p>
            </FormField>
          </div>
        )}

        <div className="pt-4 border-t border-slate-100">
           <FormField label={t('settings.price_rule.zone_scope')}>
            <input 
              value={zoneScope} 
              onChange={e => setZoneScope(e.target.value)} 
              className={inputClass} 
              placeholder={t('settings.price_rule.zone_all')} 
            />
            <p className="text-xs text-slate-400 mt-1.5">
              {t('settings.price_rule.zone_hint')}
            </p>
          </FormField>
        </div>
      </div>
    ),
  };

  const step4: WizardStep = {
    id: 'constraints',
    title: t('settings.price_rule.step_constraints'),
    description: t('settings.price_rule.step_constraints_desc'),
    isValid: true,
    component: (
      <div className="space-y-6">
        {/* Time Constraints */}
        <div className="space-y-4">
          <label className="text-sm font-medium text-slate-700 flex items-center gap-2">
            <Clock className="w-4 h-4 text-slate-400" />
            {t('settings.price_rule.active_days')}
          </label>
          <div className="flex flex-wrap gap-2">
            {DAY_INDICES.map(day => (
              <button key={day} type="button" onClick={() => toggleDay(day)}
                className={`w-10 h-10 rounded-full text-xs font-bold transition-all ${
                  activeDays.includes(day) 
                    ? 'bg-primary-600 text-white shadow-md scale-105' 
                    : 'bg-white text-slate-500 border border-slate-200 hover:border-primary-300 hover:text-primary-600'
                }`}>
                {t(`settings.price_rule.day_${day}_short`)}
              </button>
            ))}
          </div>

          <div className="grid grid-cols-2 gap-4">
            <FormField label={t('settings.price_rule.active_start_time')}>
              <input type="time" value={activeStartTime} onChange={e => setActiveStartTime(e.target.value)} className={inputClass} />
            </FormField>
            <FormField label={t('settings.price_rule.active_end_time')}>
              <input type="time" value={activeEndTime} onChange={e => setActiveEndTime(e.target.value)} className={inputClass} />
            </FormField>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <FormField label={t('settings.price_rule.valid_from')}>
              <input type="date" value={validFrom} onChange={e => setValidFrom(e.target.value)} className={inputClass} />
            </FormField>
            <FormField label={t('settings.price_rule.valid_until')}>
              <input type="date" value={validUntil} onChange={e => setValidUntil(e.target.value)} className={inputClass} />
            </FormField>
          </div>
        </div>

        {/* Flags */}
        <div className="pt-6 border-t border-slate-100 space-y-4">
          <label className="text-sm font-medium text-slate-700 flex items-center gap-2">
            <Settings2 className="w-4 h-4 text-slate-400" />
            {t('settings.price_rule.advanced')}
          </label>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div className="p-3 border border-slate-200 rounded-xl hover:border-primary-200 transition-colors bg-slate-50/50">
              <CheckboxField 
                id="is_stackable" 
                label={t('settings.price_rule.is_stackable')} 
                description={t('settings.price_rule.is_stackable_desc')} 
                checked={isStackable} 
                onChange={setIsStackable} 
              />
            </div>
            <div className="p-3 border border-slate-200 rounded-xl hover:border-primary-200 transition-colors bg-slate-50/50">
              <CheckboxField 
                id="is_exclusive" 
                label={t('settings.price_rule.is_exclusive')} 
                description={t('settings.price_rule.is_exclusive_desc')} 
                checked={isExclusive} 
                onChange={setIsExclusive} 
              />
            </div>
          </div>
        </div>
      </div>
    ),
  };

  const steps = [step1, step2, step3, step4];

  return (
    <Wizard
      steps={steps}
      onFinish={handleFinish}
      onCancel={onCancel}
      isSubmitting={isSubmitting}
      title={t('settings.price_rule.create_title')}
      finishLabel={t('common.action.create')}
    />
  );
};
