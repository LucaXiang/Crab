import React, { useState, useMemo } from 'react';
import {
  X, Check, Calculator, Percent, DollarSign,
  Target, LayoutGrid, Tag, Package, Search, FileText,
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { Currency } from '@/utils/currency';
import { useCategoryStore, useTagStore, useProductStore } from '@/core/stores/resources';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { FormSection, FormField, inputClass } from '@/shared/components/FormField';
import { updateDiscountRule } from '../mutations';
import type { MgDiscountRule, MgDiscountRuleCreate, ProductScope, AdjustmentType } from '@/core/domain/types/api';

// ── Single-select card grid (from Step2Scope) ──

function CardGridSelector<T extends { id: number; name: string }>({
  items,
  selectedId,
  onSelect,
  searchPlaceholder,
  emptyText,
  renderExtra,
}: {
  items: T[];
  selectedId: number | null;
  onSelect: (id: number | null) => void;
  searchPlaceholder: string;
  emptyText: string;
  renderExtra?: (item: T) => React.ReactNode;
}) {
  const [search, setSearch] = useState('');

  const filtered = useMemo(() => {
    if (!search.trim()) return items;
    const lower = search.toLowerCase();
    return items.filter((item) => item.name.toLowerCase().includes(lower));
  }, [items, search]);

  return (
    <div className="space-y-3">
      <div className="relative">
        <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={searchPlaceholder}
          className="w-full pl-9 pr-3 py-2 text-sm border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-violet-500/20 focus:border-violet-500 bg-white"
        />
      </div>
      <div className="grid grid-cols-3 gap-2 max-h-[14rem] overflow-y-auto custom-scrollbar content-start">
        {filtered.length === 0 ? (
          <div className="col-span-3 text-center py-6 text-sm text-gray-400">{emptyText}</div>
        ) : (
          filtered.map((item) => {
            const isSelected = selectedId === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => onSelect(selectedId === item.id ? null : item.id)}
                className={`relative p-3 rounded-xl border-2 transition-all text-left flex flex-col items-start min-h-[3.5rem] justify-center ${
                  isSelected
                    ? 'border-violet-500 bg-violet-50 ring-2 ring-violet-200'
                    : 'bg-white text-gray-700 border-gray-200 hover:border-violet-300 hover:bg-violet-50/30'
                }`}
              >
                <span className={`text-xs font-bold leading-tight ${isSelected ? 'text-violet-800' : 'text-gray-900'}`}>
                  {item.name}
                </span>
                {renderExtra?.(item)}
                {isSelected && (
                  <div className="absolute top-1.5 right-1.5">
                    <div className="w-4 h-4 bg-violet-500 rounded-full flex items-center justify-center">
                      <Check size={10} className="text-white" strokeWidth={3} />
                    </div>
                  </div>
                )}
              </button>
            );
          })
        )}
      </div>
    </div>
  );
}

// ── Main Edit Modal ──

interface DiscountRuleEditModalProps {
  rule: MgDiscountRule;
  groupId: number;
  onClose: () => void;
  onSuccess: () => void;
}

export const DiscountRuleEditModal: React.FC<DiscountRuleEditModalProps> = ({
  rule,
  groupId,
  onClose,
  onSuccess,
}) => {
  const { t } = useI18n();
  const categories = useCategoryStore((s) => s.items);
  const tags = useTagStore((s) => s.items);
  const products = useProductStore((s) => s.items);
  const [saving, setSaving] = useState(false);

  // ── Form state ──
  const [adjustmentType, setAdjustmentType] = useState<AdjustmentType>(rule.adjustment_type);
  const [adjustmentValue, setAdjustmentValue] = useState(rule.adjustment_value);
  const [inputValue, setInputValue] = useState(String(rule.adjustment_value));
  const [productScope, setProductScope] = useState<ProductScope>(rule.product_scope);
  const [targetId, setTargetId] = useState<number | null>(rule.target_id ?? null);
  const [name, setName] = useState(rule.name);
  const [displayName, setDisplayName] = useState(rule.display_name);
  const [receiptName, setReceiptName] = useState(rule.receipt_name);

  const isPercentage = adjustmentType === 'PERCENTAGE';

  const categoryMap = useMemo(() => {
    const map = new Map<number, string>();
    categories.forEach((cat) => map.set(cat.id, cat.name));
    return map;
  }, [categories]);

  const handleTypeChange = (type: AdjustmentType) => {
    setAdjustmentType(type);
    const fallback = type === 'PERCENTAGE' ? 10 : 1;
    setAdjustmentValue(fallback);
    setInputValue(String(fallback));
  };

  const handleInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    if (value === '' || /^\d*\.?\d*$/.test(value)) {
      setInputValue(value);
    }
  };

  const handleBlur = () => {
    const parsed = parseFloat(inputValue);
    if (!isNaN(parsed) && parsed > 0) {
      const formatted = isPercentage ? Math.round(parsed) : Currency.round2(parsed).toNumber();
      setAdjustmentValue(formatted);
      setInputValue(String(formatted));
    } else {
      const fallback = adjustmentValue > 0 ? adjustmentValue : (isPercentage ? 10 : 1);
      setAdjustmentValue(fallback);
      setInputValue(String(fallback));
    }
  };

  const handleScopeChange = (scope: ProductScope) => {
    setProductScope(scope);
    setTargetId(null);
  };

  const canSave =
    adjustmentValue > 0 &&
    (adjustmentType !== 'PERCENTAGE' || adjustmentValue <= 100) &&
    (productScope === 'GLOBAL' || targetId != null) &&
    name.trim() !== '' &&
    displayName.trim() !== '' &&
    receiptName.trim() !== '';

  const handleSave = async () => {
    if (!canSave) return;
    setSaving(true);
    try {
      const payload: MgDiscountRuleCreate = {
        name: name.trim(),
        display_name: displayName.trim(),
        receipt_name: receiptName.trim(),
        product_scope: productScope,
        target_id: productScope === 'GLOBAL' ? null : targetId,
        adjustment_type: adjustmentType,
        adjustment_value: adjustmentValue,
      };
      await updateDiscountRule(groupId, rule.id, payload);
      toast.success(t('settings.marketing_group.message.rule_updated'));
      onSuccess();
    } catch (e) {
      logger.error('Failed to update MG discount rule', e);
      toast.error(t('common.message.save_failed'));
    } finally {
      setSaving(false);
    }
  };

  const scopeOptions: { value: ProductScope; label: string; icon: typeof LayoutGrid; desc: string }[] = [
    { value: 'GLOBAL', label: t('settings.price_rule.scope.global'), icon: LayoutGrid, desc: t('settings.price_rule.wizard.scope_global_desc') },
    { value: 'CATEGORY', label: t('settings.price_rule.scope.category'), icon: LayoutGrid, desc: t('settings.price_rule.wizard.scope_category_desc') },
    { value: 'TAG', label: t('settings.price_rule.scope.tag'), icon: Tag, desc: t('settings.price_rule.wizard.scope_tag_desc') },
    { value: 'PRODUCT', label: t('settings.price_rule.scope.product'), icon: Package, desc: t('settings.price_rule.wizard.scope_product_desc') },
  ];

  const renderTargetSelector = () => {
    if (productScope === 'CATEGORY') {
      return (
        <FormField label={t('settings.price_rule.wizard.select_category')} required>
          <CardGridSelector
            key="CATEGORY"
            items={categories}
            selectedId={targetId}
            onSelect={setTargetId}
            searchPlaceholder={t('common.action.search')}
            emptyText={t('common.empty.no_results')}
          />
        </FormField>
      );
    }
    if (productScope === 'TAG') {
      return (
        <FormField label={t('settings.price_rule.wizard.select_tag')} required>
          <CardGridSelector
            key="TAG"
            items={tags}
            selectedId={targetId}
            onSelect={setTargetId}
            searchPlaceholder={t('common.action.search')}
            emptyText={t('common.empty.no_results')}
          />
        </FormField>
      );
    }
    if (productScope === 'PRODUCT') {
      return (
        <FormField label={t('settings.price_rule.wizard.select_product')} required>
          <CardGridSelector
            key="PRODUCT"
            items={products}
            selectedId={targetId}
            onSelect={setTargetId}
            searchPlaceholder={t('common.action.search')}
            emptyText={t('common.empty.no_results')}
            renderExtra={(prod) => {
              const catName = categoryMap.get(prod.category_id);
              return catName ? (
                <span className="text-[0.625rem] text-gray-400 mt-0.5 leading-tight">{catName}</span>
              ) : null;
            }}
          />
        </FormField>
      );
    }
    return null;
  };

  return (
    <div className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[90vh] overflow-hidden animate-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="shrink-0 px-6 py-4 border-b border-gray-100 bg-gradient-to-r from-violet-50 to-white flex items-center justify-between">
          <h2 className="text-lg font-bold text-gray-900">
            {t('settings.marketing_group.edit_rule')}
          </h2>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-xl transition-colors">
            <X size={18} className="text-gray-500" />
          </button>
        </div>

        {/* Scrollable content - all sections flat */}
        <div className="flex-1 overflow-y-auto p-6 space-y-8">

          {/* ── Section 1: Naming ── */}
          <FormSection title={t('settings.marketing_group.rule_wizard.step3_section')} icon={FileText}>
            <div className="grid grid-cols-2 gap-4 mb-4">
              <FormField label={t('settings.marketing_group.field.name')} required>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className={inputClass}
                  placeholder="mg_coffee_10off"
                />
                <p className="mt-1 text-xs text-gray-500">
                  {t('settings.marketing_group.rule_wizard.name_hint')}
                </p>
              </FormField>
              <FormField label={t('settings.marketing_group.field.display_name')} required>
                <input
                  type="text"
                  value={displayName}
                  onChange={(e) => setDisplayName(e.target.value)}
                  className={inputClass}
                  placeholder={t('settings.marketing_group.rule_wizard.display_name_placeholder')}
                />
              </FormField>
            </div>
            <FormField label={t('settings.marketing_group.rule_wizard.receipt_name')} required>
              <input
                type="text"
                value={receiptName}
                onChange={(e) => setReceiptName(e.target.value)}
                className={inputClass}
                placeholder={t('settings.marketing_group.rule_wizard.receipt_name_placeholder')}
              />
              <p className="mt-1 text-xs text-gray-500">
                {t('settings.marketing_group.rule_wizard.receipt_name_hint')}
              </p>
            </FormField>
          </FormSection>

          {/* ── Section 2: Adjustment ── */}
          <FormSection title={t('settings.marketing_group.rule_wizard.step1_section')} icon={Calculator}>
            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 mb-3">
                {t('settings.price_rule.wizard.adjustment_type')}
              </label>
              <div className="grid grid-cols-2 gap-3">
                <button
                  type="button"
                  onClick={() => handleTypeChange('PERCENTAGE')}
                  className={`flex items-center justify-center gap-2 p-4 rounded-xl border-2 transition-all ${
                    isPercentage
                      ? 'border-violet-500 bg-violet-50 text-violet-700'
                      : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300'
                  }`}
                >
                  <Percent size={20} />
                  <span className="font-medium">{t('settings.price_rule.adjustment.percentage')}</span>
                </button>
                <button
                  type="button"
                  onClick={() => handleTypeChange('FIXED_AMOUNT')}
                  className={`flex items-center justify-center gap-2 p-4 rounded-xl border-2 transition-all ${
                    !isPercentage
                      ? 'border-violet-500 bg-violet-50 text-violet-700'
                      : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300'
                  }`}
                >
                  <DollarSign size={20} />
                  <span className="font-medium">{t('settings.price_rule.adjustment.fixed')}</span>
                </button>
              </div>
            </div>

            <FormField label={t('settings.marketing_group.rule_wizard.value_label')} required>
              <div className="relative">
                <input
                  type="text"
                  inputMode="decimal"
                  value={inputValue}
                  onChange={handleInputChange}
                  onBlur={handleBlur}
                  className={`${inputClass} pr-12`}
                  placeholder={isPercentage ? '10' : '5.00'}
                />
                <span className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-400 font-medium">
                  {isPercentage ? '%' : '€'}
                </span>
              </div>
            </FormField>

            <div className="mt-4 p-4 bg-gray-50 rounded-xl">
              <p className="text-sm text-gray-600">
                <span className="font-medium">{t('settings.price_rule.wizard.preview')}: </span>
                {isPercentage
                  ? t('settings.price_rule.wizard.preview_discount_percent', { value: adjustmentValue })
                  : t('settings.price_rule.wizard.preview_discount_fixed', { value: adjustmentValue.toFixed(2) })}
              </p>
            </div>
          </FormSection>

          {/* ── Section 3: Scope ── */}
          <FormSection title={t('settings.marketing_group.rule_wizard.step2_section')} icon={Target}>
            <div className="mb-4">
              <label className="block text-sm font-medium text-gray-700 mb-3">
                {t('settings.price_rule.wizard.product_scope')}
              </label>
              <div className="grid grid-cols-2 gap-3">
                {scopeOptions.map((option) => {
                  const Icon = option.icon;
                  const isSelected = productScope === option.value;
                  return (
                    <button
                      key={option.value}
                      type="button"
                      onClick={() => handleScopeChange(option.value)}
                      className={`flex items-start gap-3 p-4 rounded-xl border-2 text-left transition-all ${
                        isSelected
                          ? 'border-violet-500 bg-violet-50'
                          : 'border-gray-200 bg-white hover:border-gray-300'
                      }`}
                    >
                      <div className={`w-10 h-10 rounded-lg flex items-center justify-center shrink-0 ${
                        isSelected ? 'bg-violet-100 text-violet-600' : 'bg-gray-100 text-gray-400'
                      }`}>
                        <Icon size={20} />
                      </div>
                      <div>
                        <span className={`font-medium block ${isSelected ? 'text-violet-700' : 'text-gray-700'}`}>
                          {option.label}
                        </span>
                        <span className="text-xs text-gray-500">{option.desc}</span>
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>

            {productScope !== 'GLOBAL' && renderTargetSelector()}
          </FormSection>
        </div>

        {/* Footer */}
        <div className="shrink-0 px-6 py-4 border-t border-gray-100 bg-gray-50/50 flex justify-end gap-3">
          <button
            onClick={onClose}
            className="px-5 py-2.5 text-gray-600 hover:bg-gray-100 rounded-xl text-sm font-medium transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          <button
            onClick={handleSave}
            disabled={saving || !canSave}
            className="flex items-center gap-2 px-5 py-2.5 bg-violet-600 text-white rounded-xl text-sm font-semibold hover:bg-violet-700 transition-colors shadow-lg shadow-violet-600/20 disabled:opacity-50"
          >
            {saving ? (
              <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
            ) : (
              <Check size={18} />
            )}
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
