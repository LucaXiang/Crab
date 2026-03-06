import React, { useState, useMemo } from 'react';
import { 
  Clock, 
  Settings2, 
  Percent, 
  DollarSign, 
  Globe, 
  Layers, 
  Tag, 
  Package, 
  Check, 
  AlertCircle 
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreInfo } from '@/core/context/StoreInfoContext';
import { Wizard, WizardStep } from '@/shared/components/Wizard';
import { FormField, inputClass } from '@/shared/components/FormField';
import type { PriceRuleCreate, RuleType, ProductScope, AdjustmentType } from '@/core/types/store';

interface PriceRuleWizardProps {
  onFinish: (data: PriceRuleCreate) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

const DAY_INDICES = [1, 2, 3, 4, 5, 6, 0]; // Mon-Sun

export const PriceRuleWizard: React.FC<PriceRuleWizardProps> = ({ onFinish, onCancel, isSubmitting }) => {
  const { t } = useI18n();
  const { currencySymbol } = useStoreInfo();

  // ── Form State ──
  const [name, setName] = useState('');
  const [receiptName, setReceiptName] = useState('');
  const [description, setDescription] = useState('');
  
  const [ruleType, setRuleType] = useState<RuleType>('DISCOUNT');
  const [adjustmentType, setAdjustmentType] = useState<AdjustmentType>('PERCENTAGE');
  const [adjustmentValue, setAdjustmentValue] = useState<number>(0);

  const [productScope, setProductScope] = useState<ProductScope>('GLOBAL');
  const [targetId, setTargetId] = useState('');
  const [zoneScope, setZoneScope] = useState('');

  const [activeDays, setActiveDays] = useState<number[]>([]);
  const [activeStartTime, setActiveStartTime] = useState('');
  const [activeEndTime, setActiveEndTime] = useState('');
  const [validFrom, setValidFrom] = useState('');
  const [validUntil, setValidUntil] = useState('');
  
  const [isStackable, setIsStackable] = useState(false);
  const [isExclusive, setIsExclusive] = useState(false);

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
      receipt_name: receiptName.trim() || undefined,
      description: description.trim() || undefined,
      rule_type: ruleType,
      product_scope: productScope,
      target_id: productScope !== 'GLOBAL' && targetId ? Number(targetId) : undefined,
      zone_scope: zoneScope && zoneScope !== 'all' ? zoneScope : undefined,
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
    isValid: !!name.trim(),
    component: (
      <div className="space-y-5">
        <FormField label={t('settings.price_rule.name')} required>
          <input 
            value={name} 
            onChange={e => setName(e.target.value)} 
            className={inputClass} 
            autoFocus 
            placeholder="e.g. Happy Hour" 
          />
        </FormField>
        <FormField label={t('settings.price_rule.receipt_name')}>
          <input 
            value={receiptName} 
            onChange={e => setReceiptName(e.target.value)} 
            className={inputClass} 
            placeholder="e.g. DISC-HH" 
          />
        </FormField>
        <FormField label={t('settings.price_rule.description')}>
          <textarea 
            value={description} 
            onChange={e => setDescription(e.target.value)} 
            className={`${inputClass} resize-none`} 
            rows={3} 
            placeholder={t('settings.price_rule.description_placeholder') || "Optional description..."} 
          />
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
      <div className="space-y-8">
        {/* Rule Type Selection */}
        <div className="space-y-3">
          <label className="text-sm font-medium text-slate-700 block">
            {t('settings.price_rule.type')}
          </label>
          <div className="grid grid-cols-2 gap-4">
            <button
              type="button"
              onClick={() => setRuleType('DISCOUNT')}
              className={`relative p-4 rounded-xl border-2 transition-all text-left group ${
                ruleType === 'DISCOUNT'
                  ? 'border-amber-500 bg-amber-50'
                  : 'border-slate-100 bg-white hover:border-slate-200 hover:bg-slate-50'
              }`}
            >
              <div className={`w-10 h-10 rounded-lg flex items-center justify-center mb-3 transition-colors ${
                ruleType === 'DISCOUNT' ? 'bg-amber-100 text-amber-600' : 'bg-slate-100 text-slate-500 group-hover:bg-white'
              }`}>
                <Percent className="w-5 h-5" />
              </div>
              <div className="font-semibold text-slate-900 mb-1">{t('settings.price_rule.type_discount')}</div>
              <div className="text-xs text-slate-500">{t('settings.price_rule.type_discount_desc') || "Deduct from price"}</div>
              {ruleType === 'DISCOUNT' && (
                <div className="absolute top-4 right-4 text-amber-500">
                  <Check className="w-5 h-5" />
                </div>
              )}
            </button>

            <button
              type="button"
              onClick={() => setRuleType('SURCHARGE')}
              className={`relative p-4 rounded-xl border-2 transition-all text-left group ${
                ruleType === 'SURCHARGE'
                  ? 'border-purple-500 bg-purple-50'
                  : 'border-slate-100 bg-white hover:border-slate-200 hover:bg-slate-50'
              }`}
            >
              <div className={`w-10 h-10 rounded-lg flex items-center justify-center mb-3 transition-colors ${
                ruleType === 'SURCHARGE' ? 'bg-purple-100 text-purple-600' : 'bg-slate-100 text-slate-500 group-hover:bg-white'
              }`}>
                <DollarSign className="w-5 h-5" />
              </div>
              <div className="font-semibold text-slate-900 mb-1">{t('settings.price_rule.type_surcharge')}</div>
              <div className="text-xs text-slate-500">{t('settings.price_rule.type_surcharge_desc') || "Add to price"}</div>
              {ruleType === 'SURCHARGE' && (
                <div className="absolute top-4 right-4 text-purple-500">
                  <Check className="w-5 h-5" />
                </div>
              )}
            </button>
          </div>
        </div>
        
        {/* Adjustment Value */}
        <div className="p-5 bg-slate-50 rounded-xl border border-slate-200 space-y-4">
          <div className="flex items-center justify-between mb-2">
            <label className="text-sm font-medium text-slate-700">
              {t('settings.price_rule.adjustment_value')}
            </label>
            
            <div className="flex bg-white rounded-lg p-1 border border-slate-200">
              <button
                type="button"
                onClick={() => setAdjustmentType('PERCENTAGE')}
                className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all ${
                  adjustmentType === 'PERCENTAGE'
                    ? 'bg-primary-600 text-white shadow-sm'
                    : 'text-slate-600 hover:bg-slate-50'
                }`}
              >
                {t('settings.price_rule.adj_percentage')} (%)
              </button>
              <button
                type="button"
                onClick={() => setAdjustmentType('FIXED_AMOUNT')}
                className={`px-3 py-1.5 text-xs font-medium rounded-md transition-all ${
                  adjustmentType === 'FIXED_AMOUNT'
                    ? 'bg-primary-600 text-white shadow-sm'
                    : 'text-slate-600 hover:bg-slate-50'
                }`}
              >
                {t('settings.price_rule.adj_fixed')} ({currencySymbol})
              </button>
            </div>
          </div>

          <div className="relative">
            <input 
              type="number" 
              value={adjustmentValue} 
              onChange={e => setAdjustmentValue(Number(e.target.value))} 
              className={`w-full px-4 py-3 bg-white border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500 transition-all text-lg font-semibold ${
                ruleType === 'DISCOUNT' ? 'text-amber-600' : 'text-purple-600'
              }`}
              placeholder="0"
              step={adjustmentType === 'PERCENTAGE' ? '1' : '0.01'} 
              min={0} 
            />
            <div className="absolute right-4 top-1/2 -translate-y-1/2 text-slate-400 font-medium bg-slate-100 px-2 py-1 rounded">
              {adjustmentType === 'PERCENTAGE' ? '%' : currencySymbol}
            </div>
          </div>
          
          <p className="text-xs text-slate-500">
            {adjustmentType === 'PERCENTAGE' 
              ? t('settings.price_rule.hint_percentage') 
              : t('settings.price_rule.hint_fixed', { symbol: currencySymbol })}
          </p>
        </div>
      </div>
    ),
  };

  const SCOPES: { value: ProductScope; label: string; icon: React.ElementType }[] = [
    { value: 'GLOBAL', label: t('settings.price_rule.scope_global'), icon: Globe },
    { value: 'CATEGORY', label: t('settings.price_rule.scope_category'), icon: Layers },
    { value: 'TAG', label: t('settings.price_rule.scope_tag'), icon: Tag },
    { value: 'PRODUCT', label: t('settings.price_rule.scope_product'), icon: Package },
  ];

  const step3: WizardStep = {
    id: 'scope',
    title: t('settings.price_rule.step_scope'),
    description: t('settings.price_rule.step_scope_desc'),
    isValid: productScope === 'GLOBAL' || !!targetId,
    component: (
      <div className="space-y-6">
        <div className="grid grid-cols-2 gap-3">
          {SCOPES.map((scope) => {
            const Icon = scope.icon;
            const isSelected = productScope === scope.value;
            return (
              <button
                key={scope.value}
                type="button"
                onClick={() => setProductScope(scope.value)}
                className={`flex flex-col items-center justify-center p-4 rounded-xl border-2 transition-all ${
                  isSelected
                    ? 'border-primary-500 bg-primary-50 text-primary-700'
                    : 'border-slate-100 bg-white text-slate-600 hover:border-slate-200 hover:bg-slate-50'
                }`}
              >
                <Icon className={`w-6 h-6 mb-2 ${isSelected ? 'text-primary-600' : 'text-slate-400'}`} />
                <span className="text-sm font-medium">{scope.label}</span>
              </button>
            );
          })}
        </div>
        
        {productScope !== 'GLOBAL' && (
          <div className="animate-in fade-in slide-in-from-top-2 duration-200 p-4 bg-slate-50 rounded-xl border border-slate-200">
            <FormField label={t('settings.price_rule.target_id')} required>
              <input 
                type="number" 
                value={targetId} 
                onChange={e => setTargetId(e.target.value)} 
                className={inputClass} 
                placeholder={t('settings.price_rule.target_id_placeholder')} 
              />
              <div className="flex items-start gap-2 mt-2 text-xs text-slate-500">
                <AlertCircle className="w-4 h-4 text-slate-400 shrink-0" />
                <p>{t('settings.price_rule.target_id_hint')}</p>
              </div>
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

        <div className="pt-4 border-t border-slate-100">
          <label className="text-sm font-medium text-slate-700 flex items-center gap-2 mb-4">
            <Settings2 className="w-4 h-4 text-slate-400" />
            {t('settings.price_rule.behavior')}
          </label>
          
          <div className="grid grid-cols-1 gap-3">
            <label className={`flex items-start gap-3 p-3 rounded-xl border cursor-pointer transition-all ${
              isExclusive 
                ? 'bg-red-50 border-red-200' 
                : 'bg-white border-slate-200 hover:border-slate-300'
            }`}>
              <div className={`mt-0.5 w-5 h-5 rounded border flex items-center justify-center shrink-0 ${
                isExclusive ? 'bg-red-500 border-red-500' : 'bg-white border-slate-300'
              }`}>
                {isExclusive && <Check className="w-3.5 h-3.5 text-white" />}
              </div>
              <input 
                type="checkbox" 
                className="hidden" 
                checked={isExclusive} 
                onChange={e => {
                  setIsExclusive(e.target.checked);
                  if (e.target.checked) setIsStackable(false);
                }} 
              />
              <div>
                <div className={`text-sm font-semibold ${isExclusive ? 'text-red-700' : 'text-slate-900'}`}>
                  {t('settings.price_rule.is_exclusive')}
                </div>
                <div className="text-xs text-slate-500 mt-0.5">{t('settings.price_rule.is_exclusive_desc')}</div>
              </div>
            </label>

            <label className={`flex items-start gap-3 p-3 rounded-xl border cursor-pointer transition-all ${
              !isExclusive && isStackable
                ? 'bg-green-50 border-green-200' 
                : 'bg-white border-slate-200 hover:border-slate-300'
            }`}>
              <div className={`mt-0.5 w-5 h-5 rounded border flex items-center justify-center shrink-0 ${
                !isExclusive && isStackable ? 'bg-green-500 border-green-500' : 'bg-white border-slate-300'
              }`}>
                {!isExclusive && isStackable && <Check className="w-3.5 h-3.5 text-white" />}
              </div>
              <input 
                type="checkbox" 
                className="hidden" 
                checked={isStackable} 
                onChange={e => {
                  setIsStackable(e.target.checked);
                  if (e.target.checked) setIsExclusive(false);
                }}
                disabled={isExclusive}
              />
              <div className={isExclusive ? 'opacity-50' : ''}>
                <div className={`text-sm font-semibold ${!isExclusive && isStackable ? 'text-green-700' : 'text-slate-900'}`}>
                  {t('settings.price_rule.is_stackable')}
                </div>
                <div className="text-xs text-slate-500 mt-0.5">{t('settings.price_rule.is_stackable_desc')}</div>
              </div>
            </label>
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