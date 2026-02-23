import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Plus, Percent, X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { listPriceRules, createPriceRule, updatePriceRule, deletePriceRule } from '@/infrastructure/api/catalog';
import { ApiError } from '@/infrastructure/api/client';
import { DataTable, type Column } from '@/shared/components/DataTable';
import { FilterBar } from '@/shared/components/FilterBar/FilterBar';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog/ConfirmDialog';
import { FormField, inputClass, CheckboxField } from '@/shared/components/FormField/FormField';
import { SelectField } from '@/shared/components/FormField/SelectField';
import { formatCurrency } from '@/utils/format';
import type {
  PriceRule, PriceRuleCreate, PriceRuleUpdate,
  RuleType, ProductScope, AdjustmentType,
} from '@/core/types/catalog';

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

export const PriceRuleManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);

  const ruleTypeOptions = useRuleTypeOptions(t);
  const productScopeOptions = useProductScopeOptions(t);
  const adjustmentTypeOptions = useAdjustmentTypeOptions(t);

  const [rules, setRules] = useState<PriceRule[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [searchQuery, setSearchQuery] = useState('');

  // Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<PriceRule | null>(null);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  // Form fields
  const [formName, setFormName] = useState('');
  const [formDisplayName, setFormDisplayName] = useState('');
  const [formReceiptName, setFormReceiptName] = useState('');
  const [formRuleType, setFormRuleType] = useState<RuleType>('DISCOUNT');
  const [formProductScope, setFormProductScope] = useState<ProductScope>('GLOBAL');
  const [formAdjustmentType, setFormAdjustmentType] = useState<AdjustmentType>('PERCENTAGE');
  const [formAdjustmentValue, setFormAdjustmentValue] = useState<number>(0);
  const [formIsStackable, setFormIsStackable] = useState(false);
  const [formIsExclusive, setFormIsExclusive] = useState(false);

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<PriceRule | null>(null);

  const loadData = useCallback(async () => {
    if (!token) return;
    try {
      setLoading(true);
      const data = await listPriceRules(token, storeId);
      setRules(data);
      setError('');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  }, [token, storeId, t]);

  useEffect(() => { loadData(); }, [loadData]);

  const filtered = useMemo(() => {
    if (!searchQuery.trim()) return rules;
    const q = searchQuery.toLowerCase();
    return rules.filter(r =>
      r.name.toLowerCase().includes(q) ||
      r.display_name.toLowerCase().includes(q)
    );
  }, [rules, searchQuery]);

  const openCreate = () => {
    setEditing(null);
    setFormName('');
    setFormDisplayName('');
    setFormReceiptName('');
    setFormRuleType('DISCOUNT');
    setFormProductScope('GLOBAL');
    setFormAdjustmentType('PERCENTAGE');
    setFormAdjustmentValue(0);
    setFormIsStackable(false);
    setFormIsExclusive(false);
    setFormError('');
    setModalOpen(true);
  };

  const openEdit = (rule: PriceRule) => {
    setEditing(rule);
    setFormName(rule.name);
    setFormDisplayName(rule.display_name);
    setFormReceiptName(rule.receipt_name);
    setFormRuleType(rule.rule_type);
    setFormProductScope(rule.product_scope);
    setFormAdjustmentType(rule.adjustment_type);
    setFormAdjustmentValue(rule.adjustment_value);
    setFormIsStackable(rule.is_stackable);
    setFormIsExclusive(rule.is_exclusive);
    setFormError('');
    setModalOpen(true);
  };

  const handleSave = async () => {
    if (!token) return;
    if (!formName.trim() || !formDisplayName.trim() || !formReceiptName.trim()) {
      setFormError(t('settings.common.required_field'));
      return;
    }

    setSaving(true);
    setFormError('');
    try {
      if (editing) {
        const payload: PriceRuleUpdate = {
          name: formName.trim(),
          display_name: formDisplayName.trim(),
          receipt_name: formReceiptName.trim(),
          rule_type: formRuleType,
          product_scope: formProductScope,
          adjustment_type: formAdjustmentType,
          adjustment_value: formAdjustmentValue,
          is_stackable: formIsStackable,
          is_exclusive: formIsExclusive,
        };
        await updatePriceRule(token, storeId, editing.source_id, payload);
      } else {
        const payload: PriceRuleCreate = {
          name: formName.trim(),
          display_name: formDisplayName.trim(),
          receipt_name: formReceiptName.trim(),
          rule_type: formRuleType,
          product_scope: formProductScope,
          adjustment_type: formAdjustmentType,
          adjustment_value: formAdjustmentValue,
          is_stackable: formIsStackable,
          is_exclusive: formIsExclusive,
        };
        await createPriceRule(token, storeId, payload);
      }
      setModalOpen(false);
      await loadData();
    } catch (err) {
      setFormError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!token || !deleteTarget) return;
    try {
      await deletePriceRule(token, storeId, deleteTarget.source_id);
      setDeleteTarget(null);
      await loadData();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      setDeleteTarget(null);
    }
  };

  const columns: Column<PriceRule>[] = [
    {
      key: 'name',
      header: t('settings.common.name'),
      render: (r) => (
        <div>
          <div className="font-medium text-gray-900">{r.name}</div>
          {r.display_name !== r.name && (
            <div className="text-xs text-gray-500">{r.display_name}</div>
          )}
        </div>
      ),
    },
    {
      key: 'type',
      header: t('settings.price_rule.type'),
      width: '120px',
      render: (r) => (
        <span className={`inline-flex px-2.5 py-0.5 rounded-full text-xs font-medium border ${
          r.rule_type === 'DISCOUNT'
            ? 'bg-orange-50 text-orange-700 border-orange-200'
            : 'bg-purple-50 text-purple-700 border-purple-200'
        }`}>
          {r.rule_type}
        </span>
      ),
    },
    {
      key: 'adjustment',
      header: t('settings.price_rule.adjustment'),
      width: '120px',
      render: (r) => (
        <span className="text-sm font-medium text-gray-900">
          {formatAdjustment(r.adjustment_type, r.adjustment_value)}
        </span>
      ),
    },
    {
      key: 'scope',
      header: t('settings.price_rule.scope'),
      width: '100px',
      render: (r) => (
        <span className="text-sm text-gray-600">{r.product_scope}</span>
      ),
    },
    {
      key: 'status',
      header: t('settings.common.status'),
      width: '100px',
      render: (r) => (
        <span className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium ${
          r.is_active ? 'bg-green-50 text-green-700' : 'bg-gray-100 text-gray-500'
        }`}>
          {r.is_active ? t('settings.common.active') : t('settings.common.inactive')}
        </span>
      ),
    },
  ];

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-orange-100 rounded-xl flex items-center justify-center">
            <Percent size={20} className="text-orange-600" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-gray-900">{t('settings.price_rule.title')}</h2>
            <p className="text-sm text-gray-500">{t('settings.price_rule.subtitle')}</p>
          </div>
        </div>
        <button
          onClick={openCreate}
          className="inline-flex items-center gap-2 px-4 py-2.5 bg-orange-600 text-white rounded-xl text-sm font-medium hover:bg-orange-700 transition-colors shadow-sm"
        >
          <Plus size={16} />
          {t('common.action.add')}
        </button>
      </div>

      {error && (
        <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{error}</div>
      )}

      {/* Filter */}
      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        totalCount={filtered.length}
        countUnit={t('settings.price_rule.unit')}
        themeColor="orange"
      />

      {/* Table */}
      <DataTable
        data={filtered}
        columns={columns}
        loading={loading}
        onEdit={openEdit}
        onDelete={(r) => setDeleteTarget(r)}
        getRowKey={(r) => r.source_id}
        themeColor="orange"
      />

      {/* Modal */}
      {modalOpen && (
        <div
          className="fixed inset-0 z-50 flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm"
          onClick={(e) => { if (e.target === e.currentTarget) setModalOpen(false); }}
        >
          <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-lg overflow-hidden max-h-[90vh] flex flex-col" style={{ animation: 'slideUp 0.25s ease-out' }}>
            {/* Modal Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100 shrink-0">
              <h3 className="text-lg font-bold text-gray-900">
                {editing ? t('common.action.edit') : t('common.action.add')} {t('settings.price_rule.title')}
              </h3>
              <button onClick={() => setModalOpen(false)} className="p-1 hover:bg-gray-100 rounded-lg transition-colors">
                <X size={20} className="text-gray-400" />
              </button>
            </div>

            {/* Modal Body */}
            <div className="px-6 py-5 space-y-4 overflow-y-auto">
              {formError && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{formError}</div>
              )}

              <FormField label={t('settings.price_rule.name')} required>
                <input
                  type="text"
                  value={formName}
                  onChange={(e) => setFormName(e.target.value)}
                  className={inputClass}
                  placeholder={t('settings.price_rule.name')}
                />
              </FormField>

              <FormField label={t('settings.price_rule.display_name')} required>
                <input
                  type="text"
                  value={formDisplayName}
                  onChange={(e) => setFormDisplayName(e.target.value)}
                  className={inputClass}
                  placeholder={t('settings.price_rule.display_name')}
                />
              </FormField>

              <FormField label={t('settings.price_rule.receipt_name')} required>
                <input
                  type="text"
                  value={formReceiptName}
                  onChange={(e) => setFormReceiptName(e.target.value)}
                  className={inputClass}
                  placeholder={t('settings.price_rule.receipt_name')}
                />
              </FormField>

              <SelectField
                label={t('settings.price_rule.type')}
                value={formRuleType}
                onChange={(v) => setFormRuleType(v as RuleType)}
                options={ruleTypeOptions}
                required
              />

              <SelectField
                label={t('settings.price_rule.scope')}
                value={formProductScope}
                onChange={(v) => setFormProductScope(v as ProductScope)}
                options={productScopeOptions}
                required
              />

              <SelectField
                label={t('settings.price_rule.adjustment_type')}
                value={formAdjustmentType}
                onChange={(v) => setFormAdjustmentType(v as AdjustmentType)}
                options={adjustmentTypeOptions}
                required
              />

              <FormField label={t('settings.price_rule.adjustment_value')} required>
                <input
                  type="number"
                  value={formAdjustmentValue}
                  onChange={(e) => setFormAdjustmentValue(Number(e.target.value))}
                  className={inputClass}
                  placeholder="0"
                  step={formAdjustmentType === 'PERCENTAGE' ? '1' : '0.01'}
                  min={0}
                />
              </FormField>

              <div className="space-y-2">
                <CheckboxField
                  id="is_stackable"
                  label={t('settings.price_rule.is_stackable')}
                  description={t('settings.price_rule.is_stackable_desc')}
                  checked={formIsStackable}
                  onChange={setFormIsStackable}
                />
                <CheckboxField
                  id="is_exclusive"
                  label={t('settings.price_rule.is_exclusive')}
                  description={t('settings.price_rule.is_exclusive_desc')}
                  checked={formIsExclusive}
                  onChange={setFormIsExclusive}
                />
              </div>
            </div>

            {/* Modal Footer */}
            <div className="px-6 py-4 border-t border-gray-100 flex justify-end gap-3 shrink-0">
              <button
                onClick={() => setModalOpen(false)}
                className="px-4 py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-medium hover:bg-gray-200 transition-colors"
              >
                {t('common.action.cancel')}
              </button>
              <button
                onClick={handleSave}
                disabled={saving}
                className="px-4 py-2.5 bg-orange-600 text-white rounded-xl text-sm font-medium hover:bg-orange-700 transition-colors disabled:opacity-50"
              >
                {saving ? t('auth.loading') : t('common.action.save')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirmation */}
      <ConfirmDialog
        isOpen={!!deleteTarget}
        title={t('common.dialog.confirm_delete')}
        description={t('settings.price_rule.confirm.delete')}
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
        variant="danger"
      />
    </div>
  );
};
