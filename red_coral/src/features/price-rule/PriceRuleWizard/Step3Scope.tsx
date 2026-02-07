import React, { useEffect, useState, useMemo } from 'react';
import { Target, LayoutGrid, Tag, Package, MapPin, Search, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { WizardState } from './index';
import { FormSection, FormField, selectClass } from '@/shared/components/FormField';
import { useCategoryStore, useTagStore, useProductStore, useZoneStore } from '@/core/stores/resources';

interface Step3ScopeProps {
  state: WizardState;
  updateState: (updates: Partial<WizardState>) => void;
}

/** Generic card-grid selector with search, used for Category / Tag / Product */
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

  const handleSelect = (id: number) => {
    onSelect(selectedId === id ? null : id);
  };

  return (
    <div className="space-y-3">
      {/* Search */}
      <div className="relative">
        <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={searchPlaceholder}
          className="w-full pl-9 pr-3 py-2 text-sm border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-teal-500/20 focus:border-teal-500 bg-white"
        />
      </div>

      {/* Card Grid */}
      <div className="grid grid-cols-3 gap-2 max-h-[16rem] overflow-y-auto custom-scrollbar content-start">
        {filtered.length === 0 ? (
          <div className="col-span-3 text-center py-6 text-sm text-gray-400">
            {emptyText}
          </div>
        ) : (
          filtered.map((item) => {
            const isSelected = selectedId === item.id;
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => handleSelect(item.id)}
                className={`
                  relative p-3 rounded-xl border-2 transition-all text-left flex flex-col items-start min-h-[3.5rem] justify-center
                  ${isSelected
                    ? 'border-teal-500 bg-teal-50 ring-2 ring-teal-200'
                    : 'bg-white text-gray-700 border-gray-200 hover:border-teal-300 hover:bg-teal-50/30'
                  }
                `}
              >
                <span className={`text-xs font-bold leading-tight ${isSelected ? 'text-teal-800' : 'text-gray-900'}`}>
                  {item.name}
                </span>
                {renderExtra?.(item)}
                {/* Checkmark */}
                {isSelected && (
                  <div className="absolute top-1.5 right-1.5">
                    <div className="w-4 h-4 bg-teal-500 rounded-full flex items-center justify-center">
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

export const Step3Scope: React.FC<Step3ScopeProps> = ({ state, updateState }) => {
  const { t } = useI18n();

  const categories = useCategoryStore((s) => s.items);
  const tags = useTagStore((s) => s.items);
  const products = useProductStore((s) => s.items);
  const zones = useZoneStore((s) => s.items);
  const fetchZones = useZoneStore((s) => s.fetchAll);

  // Build category lookup for product cards
  const categoryMap = useMemo(() => {
    const map = new Map<number, string>();
    categories.forEach((cat) => map.set(cat.id, cat.name));
    return map;
  }, [categories]);

  // Load zones on mount
  useEffect(() => {
    fetchZones();
  }, [fetchZones]);

  const scopeOptions = [
    { value: 'GLOBAL', label: t('settings.price_rule.scope.global'), icon: LayoutGrid, desc: t('settings.price_rule.wizard.scope_global_desc') },
    { value: 'CATEGORY', label: t('settings.price_rule.scope.category'), icon: LayoutGrid, desc: t('settings.price_rule.wizard.scope_category_desc') },
    { value: 'TAG', label: t('settings.price_rule.scope.tag'), icon: Tag, desc: t('settings.price_rule.wizard.scope_tag_desc') },
    { value: 'PRODUCT', label: t('settings.price_rule.scope.product'), icon: Package, desc: t('settings.price_rule.wizard.scope_product_desc') },
  ];

  const handleScopeChange = (scope: WizardState['product_scope']) => {
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
              renderExtra={(tag) => (
                <span
                  className="w-2.5 h-2.5 rounded-full mt-1 shrink-0"
                  style={{ backgroundColor: tag.color || '#9ca3af' }}
                />
              )}
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
    <FormSection title={t('settings.price_rule.wizard.step3_section')} icon={Target}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.price_rule.wizard.step3_desc')}
      </p>

      {/* Product Scope */}
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
                onClick={() => handleScopeChange(option.value as WizardState['product_scope'])}
                className={`flex items-start gap-3 p-4 rounded-xl border-2 text-left transition-all ${
                  isSelected
                    ? 'border-teal-500 bg-teal-50'
                    : 'border-gray-200 bg-white hover:border-gray-300'
                }`}
              >
                <div
                  className={`w-10 h-10 rounded-lg flex items-center justify-center shrink-0 ${
                    isSelected ? 'bg-teal-100 text-teal-600' : 'bg-gray-100 text-gray-400'
                  }`}
                >
                  <Icon size={20} />
                </div>
                <div>
                  <span className={`font-medium block ${isSelected ? 'text-teal-700' : 'text-gray-700'}`}>
                    {option.label}
                  </span>
                  <span className="text-xs text-gray-500">{option.desc}</span>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Target Selector */}
      {state.product_scope !== 'GLOBAL' && (
        <div className="mb-6">{renderTargetSelector()}</div>
      )}

      {/* Zone Scope */}
      <FormField label={t('settings.price_rule.wizard.zone_scope')} required>
        <div className="flex items-center gap-2 mb-2">
          <MapPin size={16} className="text-gray-400" />
          <span className="text-sm text-gray-600">{t('settings.price_rule.wizard.zone_scope_desc')}</span>
        </div>
        <select
          value={state.zone_scope}
          onChange={(e) => updateState({ zone_scope: e.target.value })}
          className={selectClass}
        >
          <option value="all">{t('settings.price_rule.zone.all')}</option>
          <option value="retail">{t('settings.price_rule.zone.retail')}</option>
          {zones.map((zone) => (
            <option key={zone.id} value={String(zone.id)}>
              {zone.name}
            </option>
          ))}
        </select>
      </FormField>
    </FormSection>
  );
};
