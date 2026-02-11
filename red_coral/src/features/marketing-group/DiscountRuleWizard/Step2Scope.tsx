import React, { useState, useMemo } from 'react';
import { Target, LayoutGrid, Tag, Package, Search, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { RuleWizardState } from './index';
import { FormSection, FormField } from '@/shared/components/FormField';
import { useCategoryStore, useTagStore, useProductStore } from '@/core/stores/resources';
import type { ProductScope } from '@/core/domain/types/api';

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
      <div className="grid grid-cols-3 gap-2 max-h-[16rem] overflow-y-auto custom-scrollbar content-start">
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

interface Step2ScopeProps {
  state: RuleWizardState;
  updateState: (updates: Partial<RuleWizardState>) => void;
}

export const Step2Scope: React.FC<Step2ScopeProps> = ({ state, updateState }) => {
  const { t } = useI18n();
  const categories = useCategoryStore((s) => s.items);
  const tags = useTagStore((s) => s.items);
  const products = useProductStore((s) => s.items);

  const categoryMap = useMemo(() => {
    const map = new Map<number, string>();
    categories.forEach((cat) => map.set(cat.id, cat.name));
    return map;
  }, [categories]);

  const scopeOptions: { value: ProductScope; label: string; icon: typeof LayoutGrid; desc: string }[] = [
    { value: 'GLOBAL', label: t('settings.price_rule.scope.global'), icon: LayoutGrid, desc: t('settings.price_rule.wizard.scope_global_desc') },
    { value: 'CATEGORY', label: t('settings.price_rule.scope.category'), icon: LayoutGrid, desc: t('settings.price_rule.wizard.scope_category_desc') },
    { value: 'TAG', label: t('settings.price_rule.scope.tag'), icon: Tag, desc: t('settings.price_rule.wizard.scope_tag_desc') },
    { value: 'PRODUCT', label: t('settings.price_rule.scope.product'), icon: Package, desc: t('settings.price_rule.wizard.scope_product_desc') },
  ];

  const handleScopeChange = (scope: ProductScope) => {
    updateState({ product_scope: scope, target_id: null });
  };

  const renderTargetSelector = () => {
    switch (state.product_scope) {
      case 'CATEGORY':
        return (
          <FormField label={t('settings.price_rule.wizard.select_category')} required>
            <CardGridSelector
              key="category"
              items={categories}
              selectedId={state.target_id}
              onSelect={(id) => updateState({ target_id: id })}
              searchPlaceholder={t('common.action.search')}
              emptyText={t('common.empty.no_results')}
            />
          </FormField>
        );
      case 'TAG':
        return (
          <FormField label={t('settings.price_rule.wizard.select_tag')} required>
            <CardGridSelector
              key="tag"
              items={tags}
              selectedId={state.target_id}
              onSelect={(id) => updateState({ target_id: id })}
              searchPlaceholder={t('common.action.search')}
              emptyText={t('common.empty.no_results')}
            />
          </FormField>
        );
      case 'PRODUCT':
        return (
          <FormField label={t('settings.price_rule.wizard.select_product')} required>
            <CardGridSelector
              key="product"
              items={products}
              selectedId={state.target_id}
              onSelect={(id) => updateState({ target_id: id })}
              searchPlaceholder={t('common.action.search')}
              emptyText={t('common.empty.no_results')}
              renderExtra={(prod) => {
                const catName = 'category_id' in prod ? categoryMap.get(prod.category_id as number) : undefined;
                return catName ? (
                  <span className="text-[0.625rem] text-gray-400 mt-0.5 leading-tight">{catName}</span>
                ) : null;
              }}
            />
          </FormField>
        );
      default:
        return null;
    }
  };

  return (
    <FormSection title={t('settings.marketing_group.rule_wizard.step2_section')} icon={Target}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.marketing_group.rule_wizard.step2_desc')}
      </p>

      <div className="mb-6">
        <label className="block text-sm font-medium text-gray-700 mb-3">
          {t('settings.price_rule.wizard.product_scope')}
        </label>
        <div className="grid grid-cols-2 gap-3">
          {scopeOptions.map((option) => {
            const Icon = option.icon;
            const isSelected = state.product_scope === option.value;
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

      {state.product_scope !== 'GLOBAL' && (
        <div className="mb-6">{renderTargetSelector()}</div>
      )}
    </FormSection>
  );
};
