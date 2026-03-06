import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { 
  Percent, 
  Tag, 
  Settings2, 
  Clock, 
  DollarSign, 
  Globe, 
  Layers, 
  Package, 
  Check 
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useStoreName } from '@/hooks/useStoreName';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { useStoreInfo } from '@/core/context/StoreInfoContext';
import { listPriceRules, createPriceRule, updatePriceRule, deletePriceRule } from '@/infrastructure/api/store';
import { ApiError } from '@/infrastructure/api/client';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, FormSection, inputClass, CheckboxField } from '@/shared/components/FormField';
import { formatCurrency } from '@/utils/format';
import { PriceRuleWizard } from './PriceRuleWizard';
import type {
  PriceRule, PriceRuleCreate, PriceRuleUpdate,
  RuleType, ProductScope, AdjustmentType,
} from '@/core/types/store';

function formatAdjustment(type: AdjustmentType, value: number, currencyCode: string): string {
  if (type === 'PERCENTAGE') return `${value}%`;
  return formatCurrency(value, { currency: currencyCode });
}

const DAY_INDICES = [1, 2, 3, 4, 5, 6, 0]; // Mon-Sun display order

type PanelState =
  | { type: 'closed' }
  | { type: 'create' }
  | { type: 'edit'; item: PriceRule }
  | { type: 'delete'; item: PriceRule };

export const PriceRuleManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const storeName = useStoreName();
  const { currencySymbol, currencyCode } = useStoreInfo();
  const token = useAuthStore(s => s.token);

  const [rules, setRules] = useState<PriceRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  // Form fields — basic
  const [formName, setFormName] = useState('');
  const [formReceiptName, setFormReceiptName] = useState('');
  const [formDescription, setFormDescription] = useState('');
  const [formRuleType, setFormRuleType] = useState<RuleType>('DISCOUNT');
  const [formProductScope, setFormProductScope] = useState<ProductScope>('GLOBAL');
  const [formTargetId, setFormTargetId] = useState('');
  const [formZoneScope, setFormZoneScope] = useState('all');
  const [formAdjustmentType, setFormAdjustmentType] = useState<AdjustmentType>('PERCENTAGE');
  const [formAdjustmentValue, setFormAdjustmentValue] = useState<number>(0);
  const [formIsStackable, setFormIsStackable] = useState(false);
  const [formIsExclusive, setFormIsExclusive] = useState(false);
  const [formIsActive, setFormIsActive] = useState(true);

  // Form fields — time
  const [formActiveDays, setFormActiveDays] = useState<number[]>([]);
  const [formActiveStartTime, setFormActiveStartTime] = useState('');
  const [formActiveEndTime, setFormActiveEndTime] = useState('');
  const [formValidFrom, setFormValidFrom] = useState('');
  const [formValidUntil, setFormValidUntil] = useState('');

  const handleError = useCallback((err: unknown) => {
    alert(err instanceof ApiError ? err.message : t('auth.error_generic'));
  }, [t]);

  const load = useCallback(async () => {
    if (!token) return;
    try {
      setRules(await listPriceRules(token, storeId));
    } catch (err) { handleError(err); }
    finally { setLoading(false); }
  }, [token, storeId, handleError]);

  useEffect(() => { load(); }, [load]);

  const filtered = useMemo(() => {
    if (!search.trim()) return rules;
    const q = search.toLowerCase();
    return rules.filter(r => r.name.toLowerCase().includes(q));
  }, [rules, search]);

  const selectedId = panel.type === 'edit' ? panel.item.source_id : null;

  const tsToDateStr = (ts: number | null): string => {
    if (!ts) return '';
    return new Date(ts).toISOString().slice(0, 10);
  };
  const dateStrToTs = (s: string): number | undefined => {
    if (!s) return undefined;
    return new Date(s + 'T00:00:00Z').getTime();
  };

  const toggleDay = (day: number) => {
    setFormActiveDays(prev =>
      prev.includes(day) ? prev.filter(d => d !== day) : [...prev, day]
    );
  };

  const openCreate = () => {
    setPanel({ type: 'create' });
  };

  const openEdit = (rule: PriceRule) => {
    setFormName(rule.name);
    setFormReceiptName(rule.receipt_name ?? ''); setFormDescription(rule.description ?? '');
    setFormRuleType(rule.rule_type); setFormProductScope(rule.product_scope);
    setFormTargetId(rule.target_id != null ? String(rule.target_id) : '');
    setFormZoneScope(rule.zone_scope || 'all');
    setFormAdjustmentType(rule.adjustment_type); setFormAdjustmentValue(rule.adjustment_value);
    setFormIsStackable(rule.is_stackable); setFormIsExclusive(rule.is_exclusive);
    setFormIsActive(rule.is_active);
    setFormActiveDays(rule.active_days ?? []);
    setFormActiveStartTime(rule.active_start_time ?? '');
    setFormActiveEndTime(rule.active_end_time ?? '');
    setFormValidFrom(tsToDateStr(rule.valid_from));
    setFormValidUntil(tsToDateStr(rule.valid_until));
    setFormError('');
    setPanel({ type: 'edit', item: rule });
  };

  const handleCreate = async (data: PriceRuleCreate) => {
    if (!token || saving) return;
    setSaving(true);
    try {
      await createPriceRule(token, storeId, data);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) {
      handleError(err);
    } finally {
      setSaving(false);
    }
  };

  const handleUpdate = async () => {
    if (!token || saving || panel.type !== 'edit') return;
    if (!formName.trim()) {
      setFormError(t('settings.common.required_field')); return;
    }

    setSaving(true); setFormError('');
    try {
      const payload: PriceRuleUpdate = {
        name: formName.trim(),
        receipt_name: formReceiptName.trim() || undefined,
        description: formDescription.trim() || undefined,
        rule_type: formRuleType, product_scope: formProductScope,
        target_id: formProductScope !== 'GLOBAL' && formTargetId ? Number(formTargetId) : undefined,
        zone_scope: formZoneScope !== 'all' ? formZoneScope : undefined,
        adjustment_type: formAdjustmentType, adjustment_value: formAdjustmentValue,
        is_stackable: formIsStackable, is_exclusive: formIsExclusive,
        active_days: formActiveDays.length > 0 ? formActiveDays : undefined,
        active_start_time: formActiveStartTime || undefined,
        active_end_time: formActiveEndTime || undefined,
        valid_from: dateStrToTs(formValidFrom), valid_until: dateStrToTs(formValidUntil),
        is_active: formIsActive,
      };

      await updatePriceRule(token, storeId, panel.item.source_id, payload);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) {
      setFormError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally { setSaving(false); }
  };

  const handleDelete = async () => {
    if (!token || panel.type !== 'delete') return;
    setSaving(true);
    try {
      await deletePriceRule(token, storeId, panel.item.source_id);
      setPanel({ type: 'closed' });
      await load();
    } catch (err) { handleError(err); }
    finally { setSaving(false); }
  };

  const SCOPES = [
    { value: 'GLOBAL' as ProductScope, label: t('settings.price_rule.scope_global'), icon: Globe },
    { value: 'CATEGORY' as ProductScope, label: t('settings.price_rule.scope_category'), icon: Layers },
    { value: 'TAG' as ProductScope, label: t('settings.price_rule.scope_tag'), icon: Tag },
    { value: 'PRODUCT' as ProductScope, label: t('settings.price_rule.scope_product'), icon: Package },
  ];

  const renderItem = (rule: PriceRule, isSelected: boolean) => (
    <div className={`px-4 py-3.5 ${isSelected ? 'font-medium' : ''}`}>
      <div className="flex items-center justify-between">
        <span className={`text-sm truncate ${rule.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
          {rule.name}
        </span>
        <span className="text-xs font-semibold tabular-nums shrink-0 ml-2 text-gray-500">
          {formatAdjustment(rule.adjustment_type, rule.adjustment_value, currencyCode)}
        </span>
      </div>
      <div className="flex items-center gap-1.5 mt-1">
        <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${
          rule.rule_type === 'DISCOUNT' ? 'bg-orange-100 text-orange-700' : 'bg-purple-100 text-purple-700'
        }`}>
          {rule.rule_type === 'DISCOUNT' ? t('settings.price_rule.type_discount') : t('settings.price_rule.type_surcharge')}
        </span>
        <span className="text-[10px] px-1.5 py-0.5 rounded bg-gray-100 text-gray-500">
          {SCOPES.find(s => s.value === rule.product_scope)?.label ?? rule.product_scope}
        </span>
      </div>
    </div>
  );

  const formContent = (
    <>
      {formError && (
        <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{formError}</div>
      )}

      {/* Rule Type Selector */}
      <div className="grid grid-cols-2 gap-3 mb-6">
        <button
          type="button"
          onClick={() => setFormRuleType('DISCOUNT')}
          className={`relative p-3 rounded-xl border-2 transition-all text-left group ${
            formRuleType === 'DISCOUNT'
              ? 'border-amber-500 bg-amber-50'
              : 'border-slate-100 bg-white hover:border-slate-200 hover:bg-slate-50'
          }`}
        >
          <div className="flex items-center gap-2 mb-1">
            <div className={`w-6 h-6 rounded flex items-center justify-center ${
              formRuleType === 'DISCOUNT' ? 'bg-amber-100 text-amber-600' : 'bg-slate-100 text-slate-500'
            }`}>
              <Percent className="w-3.5 h-3.5" />
            </div>
            <span className="font-semibold text-sm text-slate-900">{t('settings.price_rule.type_discount')}</span>
          </div>
          {formRuleType === 'DISCOUNT' && <Check className="absolute top-3 right-3 w-4 h-4 text-amber-500" />}
        </button>

        <button
          type="button"
          onClick={() => setFormRuleType('SURCHARGE')}
          className={`relative p-3 rounded-xl border-2 transition-all text-left group ${
            formRuleType === 'SURCHARGE'
              ? 'border-purple-500 bg-purple-50'
              : 'border-slate-100 bg-white hover:border-slate-200 hover:bg-slate-50'
          }`}
        >
          <div className="flex items-center gap-2 mb-1">
            <div className={`w-6 h-6 rounded flex items-center justify-center ${
              formRuleType === 'SURCHARGE' ? 'bg-purple-100 text-purple-600' : 'bg-slate-100 text-slate-500'
            }`}>
              <DollarSign className="w-3.5 h-3.5" />
            </div>
            <span className="font-semibold text-sm text-slate-900">{t('settings.price_rule.type_surcharge')}</span>
          </div>
          {formRuleType === 'SURCHARGE' && <Check className="absolute top-3 right-3 w-4 h-4 text-purple-500" />}
        </button>
      </div>

      <FormSection title={t('settings.price_rule.section_basics')} icon={Settings2}>
        <FormField label={t('settings.price_rule.name')} required>
          <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} />
        </FormField>
        <FormField label={t('settings.price_rule.receipt_name')}>
          <input value={formReceiptName} onChange={e => setFormReceiptName(e.target.value)} className={inputClass} />
        </FormField>
        <FormField label={t('settings.price_rule.description')}>
          <textarea value={formDescription} onChange={e => setFormDescription(e.target.value)} className={`${inputClass} resize-none`} rows={2} />
        </FormField>
      </FormSection>

      <FormSection title={t('settings.price_rule.adjustment')} icon={DollarSign}>
        <div className="flex items-center justify-between mb-2">
          <label className="text-xs font-medium text-slate-500 uppercase tracking-wider">
            {t('settings.price_rule.adjustment_value')}
          </label>
          <div className="flex bg-slate-100 rounded-lg p-0.5">
            <button
              type="button"
              onClick={() => setFormAdjustmentType('PERCENTAGE')}
              className={`px-3 py-1 text-xs font-medium rounded-md transition-all ${
                formAdjustmentType === 'PERCENTAGE'
                  ? 'bg-white text-slate-900 shadow-sm'
                  : 'text-slate-500 hover:text-slate-700'
              }`}
            >
              %
            </button>
            <button
              type="button"
              onClick={() => setFormAdjustmentType('FIXED_AMOUNT')}
              className={`px-3 py-1 text-xs font-medium rounded-md transition-all ${
                formAdjustmentType === 'FIXED_AMOUNT'
                  ? 'bg-white text-slate-900 shadow-sm'
                  : 'text-slate-500 hover:text-slate-700'
              }`}
            >
              {currencySymbol}
            </button>
          </div>
        </div>
        
        <div className="relative">
          <input 
            type="number" 
            value={formAdjustmentValue} 
            onChange={e => setFormAdjustmentValue(Number(e.target.value))} 
            className={`w-full px-4 py-3 bg-white border border-slate-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500 transition-all text-lg font-semibold ${
              formRuleType === 'DISCOUNT' ? 'text-amber-600' : 'text-purple-600'
            }`}
            step={formAdjustmentType === 'PERCENTAGE' ? '1' : '0.01'} 
            min={0} 
          />
          <div className="absolute right-4 top-1/2 -translate-y-1/2 text-slate-400 font-medium">
            {formAdjustmentType === 'PERCENTAGE' ? '%' : currencySymbol}
          </div>
        </div>
      </FormSection>

      <FormSection title={t('settings.price_rule.scope')} icon={Tag}>
        <div className="grid grid-cols-4 gap-2 mb-4">
          {SCOPES.map((scope) => {
            const Icon = scope.icon;
            const isSelected = formProductScope === scope.value;
            return (
              <button
                key={scope.value}
                type="button"
                onClick={() => setFormProductScope(scope.value)}
                className={`flex flex-col items-center justify-center p-2 rounded-lg border transition-all ${
                  isSelected
                    ? 'border-primary-500 bg-primary-50 text-primary-700'
                    : 'border-slate-200 bg-white text-slate-500 hover:border-slate-300 hover:bg-slate-50'
                }`}
                title={scope.label}
              >
                <Icon className={`w-4 h-4 mb-1 ${isSelected ? 'text-primary-600' : 'text-slate-400'}`} />
                <span className="text-[10px] font-medium truncate w-full text-center">{scope.label}</span>
              </button>
            );
          })}
        </div>
        
        {formProductScope !== 'GLOBAL' && (
          <FormField label={t('settings.price_rule.target_id')}>
            <input type="number" value={formTargetId} onChange={e => setFormTargetId(e.target.value)} className={inputClass} placeholder={t('settings.price_rule.target_id_placeholder')} />
          </FormField>
        )}
        <FormField label={t('settings.price_rule.zone_scope')}>
          <input value={formZoneScope} onChange={e => setFormZoneScope(e.target.value)} className={inputClass} placeholder={t('settings.price_rule.zone_all')} />
        </FormField>
      </FormSection>

      <FormSection title={t('settings.price_rule.section_time')} icon={Clock} defaultCollapsed>
        <FormField label={t('settings.price_rule.active_days')}>
          <div className="flex flex-wrap gap-2">
            {DAY_INDICES.map(day => (
              <button key={day} type="button" onClick={() => toggleDay(day)}
                className={`w-8 h-8 rounded-full text-xs font-bold transition-all ${
                  formActiveDays.includes(day) 
                    ? 'bg-primary-600 text-white shadow-sm' 
                    : 'bg-white text-slate-500 border border-slate-200 hover:border-primary-300 hover:text-primary-600'
                }`}>
                {t(`settings.price_rule.day_${day}_short`)}
              </button>
            ))}
          </div>
        </FormField>
        <div className="grid grid-cols-2 gap-3">
          <FormField label={t('settings.price_rule.active_start_time')}>
            <input type="time" value={formActiveStartTime} onChange={e => setFormActiveStartTime(e.target.value)} className={inputClass} />
          </FormField>
          <FormField label={t('settings.price_rule.active_end_time')}>
            <input type="time" value={formActiveEndTime} onChange={e => setFormActiveEndTime(e.target.value)} className={inputClass} />
          </FormField>
        </div>
        <div className="grid grid-cols-2 gap-3">
          <FormField label={t('settings.price_rule.valid_from')}>
            <input type="date" value={formValidFrom} onChange={e => setFormValidFrom(e.target.value)} className={inputClass} />
          </FormField>
          <FormField label={t('settings.price_rule.valid_until')}>
            <input type="date" value={formValidUntil} onChange={e => setFormValidUntil(e.target.value)} className={inputClass} />
          </FormField>
        </div>
      </FormSection>

      <FormSection title={t('settings.price_rule.behavior')} icon={Settings2}>
        <CheckboxField id="is_stackable" label={t('settings.price_rule.is_stackable')} description={t('settings.price_rule.is_stackable_desc')} checked={formIsStackable} onChange={setFormIsStackable} />
        <CheckboxField id="is_exclusive" label={t('settings.price_rule.is_exclusive')} description={t('settings.price_rule.is_exclusive_desc')} checked={formIsExclusive} onChange={setFormIsExclusive} />
        {panel.type === 'edit' && (
          <div className="pt-4 mt-4 border-t border-slate-100">
            <CheckboxField id="is_active" label={t('settings.common.active')} checked={formIsActive} onChange={setFormIsActive} />
          </div>
        )}
      </FormSection>
    </>
  );

  return (
    <div className="h-full flex flex-col p-4 lg:p-6">
      <div className="flex items-center gap-3 mb-4 shrink-0">
        <div className="w-10 h-10 bg-orange-100 rounded-xl flex items-center justify-center">
          <Percent className="w-5 h-5 text-orange-600" />
        </div>
        <div>
          <h1 className="text-xl font-bold text-slate-900">{t('settings.price_rule.title')}</h1>
          <p className="text-xs text-gray-400">{storeName}</p>
        </div>
      </div>

      <div className="flex-1 min-h-0">
        <MasterDetail
          items={filtered}
          getItemId={(r) => r.source_id}
          renderItem={renderItem}
          selectedId={selectedId}
          onSelect={openEdit}
          onDeselect={() => setPanel({ type: 'closed' })}
          searchQuery={search}
          onSearchChange={setSearch}
          totalCount={filtered.length}
          countUnit={t('settings.price_rule.unit')}
          onCreateNew={openCreate}
          createLabel={t('common.action.add')}
          isCreating={panel.type === 'create'}
          themeColor="orange"
          loading={loading}
        >
          {panel.type === 'create' && (
            <div className="h-full p-4 lg:p-8 bg-slate-50/50">
              <PriceRuleWizard 
                onFinish={handleCreate} 
                onCancel={() => setPanel({ type: 'closed' })}
                isSubmitting={saving}
              />
            </div>
          )}
          {panel.type === 'edit' && (
            <DetailPanel
              title={`${t('common.action.edit')} ${t('settings.price_rule.title')}`}
              isCreating={false}
              onClose={() => setPanel({ type: 'closed' })}
              onSave={handleUpdate}
              onDelete={() => setPanel({ type: 'delete', item: panel.item })}
              saving={saving}
              saveDisabled={!formName.trim()}
            >
              {formContent}
            </DetailPanel>
          )}
        </MasterDetail>
      </div>

      <ConfirmDialog
        isOpen={panel.type === 'delete'}
        title={t('common.dialog.confirm_delete')}
        description={t('settings.price_rule.confirm.delete')}
        onConfirm={handleDelete}
        onCancel={() => setPanel({ type: 'closed' })}
        variant="danger"
      />
    </div>
  );
};
