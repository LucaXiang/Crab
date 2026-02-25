import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Percent, Tag, Settings2, Clock } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { listPriceRules, createPriceRule, updatePriceRule, deletePriceRule } from '@/infrastructure/api/store';
import { ApiError } from '@/infrastructure/api/client';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, FormSection, inputClass, CheckboxField } from '@/shared/components/FormField';
import { SelectField } from '@/shared/components/FormField/SelectField';
import { formatCurrency } from '@/utils/format';
import type {
  PriceRule, PriceRuleCreate, PriceRuleUpdate,
  RuleType, ProductScope, AdjustmentType,
} from '@/core/types/store';

function useRuleTypeOptions(t: (key: string) => string) {
  return [
    { value: 'DISCOUNT' as RuleType, label: t('settings.price_rule.type_discount') },
    { value: 'SURCHARGE' as RuleType, label: t('settings.price_rule.type_surcharge') },
  ];
}

function useProductScopeOptions(t: (key: string) => string) {
  return [
    { value: 'GLOBAL' as ProductScope, label: t('settings.price_rule.scope_global') },
    { value: 'CATEGORY' as ProductScope, label: t('settings.price_rule.scope_category') },
    { value: 'TAG' as ProductScope, label: t('settings.price_rule.scope_tag') },
    { value: 'PRODUCT' as ProductScope, label: t('settings.price_rule.scope_product') },
  ];
}

function useAdjustmentTypeOptions(t: (key: string) => string) {
  return [
    { value: 'PERCENTAGE' as AdjustmentType, label: t('settings.price_rule.adjustment_percentage') },
    { value: 'FIXED_AMOUNT' as AdjustmentType, label: t('settings.price_rule.adjustment_fixed') },
  ];
}

function formatAdjustment(type: AdjustmentType, value: number): string {
  if (type === 'PERCENTAGE') return `${value}%`;
  return formatCurrency(value);
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
  const token = useAuthStore(s => s.token);

  const ruleTypeOptions = useRuleTypeOptions(t);
  const productScopeOptions = useProductScopeOptions(t);
  const adjustmentTypeOptions = useAdjustmentTypeOptions(t);

  const [rules, setRules] = useState<PriceRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  // Form fields — basic
  const [formName, setFormName] = useState('');
  const [formDisplayName, setFormDisplayName] = useState('');
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
    return rules.filter(r => r.name.toLowerCase().includes(q) || r.display_name.toLowerCase().includes(q));
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
    setFormName(''); setFormDisplayName(''); setFormReceiptName(''); setFormDescription('');
    setFormRuleType('DISCOUNT'); setFormProductScope('GLOBAL'); setFormTargetId('');
    setFormZoneScope('all'); setFormAdjustmentType('PERCENTAGE'); setFormAdjustmentValue(0);
    setFormIsStackable(false); setFormIsExclusive(false); setFormIsActive(true);
    setFormActiveDays([]); setFormActiveStartTime(''); setFormActiveEndTime('');
    setFormValidFrom(''); setFormValidUntil(''); setFormError('');
    setPanel({ type: 'create' });
  };

  const openEdit = (rule: PriceRule) => {
    setFormName(rule.name); setFormDisplayName(rule.display_name);
    setFormReceiptName(rule.receipt_name); setFormDescription(rule.description ?? '');
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

  const handleSave = async () => {
    if (!token || saving) return;
    if (!formName.trim() || !formDisplayName.trim() || !formReceiptName.trim()) {
      setFormError(t('settings.common.required_field')); return;
    }

    setSaving(true); setFormError('');
    try {
      const common = {
        name: formName.trim(), display_name: formDisplayName.trim(),
        receipt_name: formReceiptName.trim(),
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
      };

      if (panel.type === 'edit') {
        const payload: PriceRuleUpdate = { ...common, is_active: formIsActive };
        await updatePriceRule(token, storeId, panel.item.source_id, payload);
      } else if (panel.type === 'create') {
        const payload: PriceRuleCreate = common;
        await createPriceRule(token, storeId, payload);
      }
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

  const renderItem = (rule: PriceRule, isSelected: boolean) => (
    <div className={`px-4 py-3.5 ${isSelected ? 'font-medium' : ''}`}>
      <div className="flex items-center justify-between">
        <span className={`text-sm truncate ${rule.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
          {rule.name}
        </span>
        <span className="text-xs font-semibold tabular-nums shrink-0 ml-2 text-gray-500">
          {formatAdjustment(rule.adjustment_type, rule.adjustment_value)}
        </span>
      </div>
      <div className="flex items-center gap-1.5 mt-1">
        <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium ${
          rule.rule_type === 'DISCOUNT' ? 'bg-orange-100 text-orange-700' : 'bg-purple-100 text-purple-700'
        }`}>
          {rule.rule_type === 'DISCOUNT' ? t('settings.price_rule.type_discount') : t('settings.price_rule.type_surcharge')}
        </span>
        <span className="text-[10px] px-1.5 py-0.5 rounded bg-gray-100 text-gray-500">
          {productScopeOptions.find(o => o.value === rule.product_scope)?.label ?? rule.product_scope}
        </span>
      </div>
    </div>
  );

  const formContent = (
    <>
      {formError && (
        <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{formError}</div>
      )}

      <FormField label={t('settings.price_rule.name')} required>
        <input value={formName} onChange={e => setFormName(e.target.value)} className={inputClass} autoFocus />
      </FormField>
      <FormField label={t('settings.price_rule.display_name')} required>
        <input value={formDisplayName} onChange={e => setFormDisplayName(e.target.value)} className={inputClass} />
      </FormField>
      <FormField label={t('settings.price_rule.receipt_name')} required>
        <input value={formReceiptName} onChange={e => setFormReceiptName(e.target.value)} className={inputClass} />
      </FormField>
      <FormField label={t('settings.price_rule.description')}>
        <textarea value={formDescription} onChange={e => setFormDescription(e.target.value)} className={`${inputClass} resize-none`} rows={2} />
      </FormField>

      <SelectField label={t('settings.price_rule.type')} value={formRuleType} onChange={v => setFormRuleType(v as RuleType)} options={ruleTypeOptions} required />
      <SelectField label={t('settings.price_rule.adjustment_type')} value={formAdjustmentType} onChange={v => setFormAdjustmentType(v as AdjustmentType)} options={adjustmentTypeOptions} required />

      <FormField label={t('settings.price_rule.adjustment_value')} required>
        <input type="number" value={formAdjustmentValue} onChange={e => setFormAdjustmentValue(Number(e.target.value))} className={inputClass} step={formAdjustmentType === 'PERCENTAGE' ? '1' : '0.01'} min={0} />
      </FormField>

      <FormSection title={t('settings.price_rule.scope')} icon={Tag}>
        <SelectField label={t('settings.price_rule.scope')} value={formProductScope} onChange={v => setFormProductScope(v as ProductScope)} options={productScopeOptions} required />
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
                className={`px-3 py-1.5 text-xs font-medium rounded-lg border transition-colors ${
                  formActiveDays.includes(day) ? 'bg-orange-50 text-orange-700 border-orange-300' : 'bg-white text-gray-600 border-gray-200 hover:border-gray-300'
                }`}>
                {t(`settings.price_rule.day_${day}`)}
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

      <FormSection title={t('settings.price_rule.adjustment')} icon={Settings2}>
        <CheckboxField id="is_stackable" label={t('settings.price_rule.is_stackable')} description={t('settings.price_rule.is_stackable_desc')} checked={formIsStackable} onChange={setFormIsStackable} />
        <CheckboxField id="is_exclusive" label={t('settings.price_rule.is_exclusive')} description={t('settings.price_rule.is_exclusive_desc')} checked={formIsExclusive} onChange={setFormIsExclusive} />
        {panel.type === 'edit' && (
          <CheckboxField id="is_active" label={t('settings.common.active')} checked={formIsActive} onChange={setFormIsActive} />
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
          <p className="text-xs text-gray-400">{t('settings.price_rule.subtitle')}</p>
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
          {(panel.type === 'create' || panel.type === 'edit') && (
            <DetailPanel
              title={panel.type === 'create' ? `${t('common.action.add')} ${t('settings.price_rule.title')}` : `${t('common.action.edit')} ${t('settings.price_rule.title')}`}
              isCreating={panel.type === 'create'}
              onClose={() => setPanel({ type: 'closed' })}
              onSave={handleSave}
              onDelete={panel.type === 'edit' ? () => setPanel({ type: 'delete', item: panel.item }) : undefined}
              saving={saving}
              saveDisabled={!formName.trim() || !formDisplayName.trim() || !formReceiptName.trim()}
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
