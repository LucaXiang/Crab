import React, { useState, useMemo, useCallback } from 'react';
import { Crosshair, LayoutGrid, Package, Search, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { StampWizardState } from './index';
import type { StampTargetInput, StampTargetType } from '@/core/domain/types/api';
import { FormSection } from '@/shared/components/FormField';
import { useCategoryStore, useProductStore } from '@/core/stores/resources';

// ── Multi-select Card Grid ──

function MultiCardGridSelector<T extends { id: number; name: string }>({
  items,
  selectedIds,
  onToggle,
  searchPlaceholder,
  emptyText,
  accentColor = 'teal',
  renderExtra,
}: {
  items: T[];
  selectedIds: Set<number>;
  onToggle: (id: number) => void;
  searchPlaceholder: string;
  emptyText: string;
  accentColor?: 'teal' | 'violet';
  renderExtra?: (item: T) => React.ReactNode;
}) {
  const [search, setSearch] = useState('');

  const filtered = useMemo(() => {
    if (!search.trim()) return items;
    const lower = search.toLowerCase();
    return items.filter((item) => item.name.toLowerCase().includes(lower));
  }, [items, search]);

  const colors = accentColor === 'teal'
    ? { border: 'border-teal-500', bg: 'bg-teal-50', ring: 'ring-teal-200', text: 'text-teal-800', check: 'bg-teal-500', hover: 'hover:border-teal-300 hover:bg-teal-50/30', focus: 'focus:ring-teal-500/20 focus:border-teal-500' }
    : { border: 'border-violet-500', bg: 'bg-violet-50', ring: 'ring-violet-200', text: 'text-violet-800', check: 'bg-violet-500', hover: 'hover:border-violet-300 hover:bg-violet-50/30', focus: 'focus:ring-violet-500/20 focus:border-violet-500' };

  return (
    <div className="space-y-3">
      <div className="relative">
        <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={searchPlaceholder}
          className={`w-full pl-9 pr-3 py-2 text-sm border border-gray-200 rounded-xl focus:outline-none focus:ring-2 ${colors.focus} bg-white`}
        />
      </div>
      <div className="grid grid-cols-3 gap-2 max-h-[14rem] overflow-y-auto custom-scrollbar content-start">
        {filtered.length === 0 ? (
          <div className="col-span-3 text-center py-6 text-sm text-gray-400">{emptyText}</div>
        ) : (
          filtered.map((item) => {
            const isSelected = selectedIds.has(item.id);
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => onToggle(item.id)}
                className={`relative p-3 rounded-xl border-2 transition-all text-left flex flex-col items-start min-h-[3.5rem] justify-center ${
                  isSelected
                    ? `${colors.border} ${colors.bg} ring-2 ${colors.ring}`
                    : `bg-white text-gray-700 border-gray-200 ${colors.hover}`
                }`}
              >
                <span className={`text-xs font-bold leading-tight ${isSelected ? colors.text : 'text-gray-900'}`}>
                  {item.name}
                </span>
                {renderExtra?.(item)}
                {isSelected && (
                  <div className="absolute top-1.5 right-1.5">
                    <div className={`w-4 h-4 ${colors.check} rounded-full flex items-center justify-center`}>
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

// ── Step 4 ──

interface Step4TargetsProps {
  state: StampWizardState;
  updateState: (updates: Partial<StampWizardState>) => void;
}

type TargetTab = 'CATEGORY' | 'PRODUCT';

export const Step4Targets: React.FC<Step4TargetsProps> = ({ state, updateState }) => {
  const { t } = useI18n();
  const categories = useCategoryStore((s) => s.items);
  const products = useProductStore((s) => s.items);

  const [stampTab, setStampTab] = useState<TargetTab>('CATEGORY');
  const [rewardTab, setRewardTab] = useState<TargetTab>('CATEGORY');

  const categoryMap = useMemo(() => {
    const map = new Map<number, string>();
    categories.forEach((cat) => map.set(cat.id, cat.name));
    return map;
  }, [categories]);

  // Convert StampTargetInput[] → Set<number> per type
  const getSelectedIds = useCallback((targets: StampTargetInput[], type: StampTargetType): Set<number> => {
    return new Set(targets.filter((t) => t.target_type === type).map((t) => t.target_id));
  }, []);

  // Toggle an item in targets array
  const toggleTarget = useCallback((field: 'stamp_targets' | 'reward_targets', type: StampTargetType, id: number) => {
    const current = state[field];
    const exists = current.some((t) => t.target_type === type && t.target_id === id);
    if (exists) {
      updateState({ [field]: current.filter((t) => !(t.target_type === type && t.target_id === id)) });
    } else {
      updateState({ [field]: [...current, { target_type: type, target_id: id }] });
    }
  }, [state, updateState]);

  const renderTargetSection = (
    field: 'stamp_targets' | 'reward_targets',
    label: string,
    description: string,
    required: boolean,
    activeTab: TargetTab,
    setTab: (tab: TargetTab) => void,
  ) => {
    const catIds = getSelectedIds(state[field], 'CATEGORY');
    const prodIds = getSelectedIds(state[field], 'PRODUCT');
    const totalCount = catIds.size + prodIds.size;

    return (
      <div className="space-y-4">
        <div>
          <div className="flex items-center gap-2">
            <label className="text-sm font-medium text-gray-700">{label}</label>
            {required && <span className="text-red-400">*</span>}
            {totalCount > 0 && (
              <span className="text-xs bg-teal-100 text-teal-700 px-2 py-0.5 rounded-full font-medium">
                {totalCount}
              </span>
            )}
          </div>
          <p className="text-xs text-gray-400 mt-0.5">{description}</p>
        </div>

        {/* Tab Toggle */}
        <div className="flex gap-2">
          <button
            type="button"
            onClick={() => setTab('CATEGORY')}
            className={`flex items-center gap-1.5 px-4 py-2 rounded-xl text-sm font-medium transition-all ${
              activeTab === 'CATEGORY'
                ? 'bg-teal-50 text-teal-700 border-2 border-teal-500'
                : 'bg-gray-50 text-gray-600 border-2 border-transparent hover:bg-gray-100'
            }`}
          >
            <LayoutGrid size={16} />
            {t('settings.marketing_group.stamp.target_type.category')}
            {catIds.size > 0 && (
              <span className={`text-xs px-1.5 py-0.5 rounded-full ${
                activeTab === 'CATEGORY' ? 'bg-teal-200 text-teal-800' : 'bg-gray-200 text-gray-600'
              }`}>
                {catIds.size}
              </span>
            )}
          </button>
          <button
            type="button"
            onClick={() => setTab('PRODUCT')}
            className={`flex items-center gap-1.5 px-4 py-2 rounded-xl text-sm font-medium transition-all ${
              activeTab === 'PRODUCT'
                ? 'bg-teal-50 text-teal-700 border-2 border-teal-500'
                : 'bg-gray-50 text-gray-600 border-2 border-transparent hover:bg-gray-100'
            }`}
          >
            <Package size={16} />
            {t('settings.marketing_group.stamp.target_type.product')}
            {prodIds.size > 0 && (
              <span className={`text-xs px-1.5 py-0.5 rounded-full ${
                activeTab === 'PRODUCT' ? 'bg-teal-200 text-teal-800' : 'bg-gray-200 text-gray-600'
              }`}>
                {prodIds.size}
              </span>
            )}
          </button>
        </div>

        {/* Card Grid */}
        {activeTab === 'CATEGORY' ? (
          <MultiCardGridSelector
            key="cat"
            items={categories}
            selectedIds={catIds}
            onToggle={(id) => toggleTarget(field, 'CATEGORY', id)}
            searchPlaceholder={t('common.action.search')}
            emptyText={t('common.empty.no_results')}
          />
        ) : (
          <MultiCardGridSelector
            key="prod"
            items={products}
            selectedIds={prodIds}
            onToggle={(id) => toggleTarget(field, 'PRODUCT', id)}
            searchPlaceholder={t('common.action.search')}
            emptyText={t('common.empty.no_results')}
            renderExtra={(prod) => {
              const catName = 'category_id' in prod ? categoryMap.get(prod.category_id as number) : undefined;
              return catName ? (
                <span className="text-[0.625rem] text-gray-400 mt-0.5 leading-tight">{catName}</span>
              ) : null;
            }}
          />
        )}
      </div>
    );
  };

  return (
    <FormSection title={t('settings.marketing_group.stamp_wizard.step4_section')} icon={Crosshair}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.marketing_group.stamp_wizard.step4_desc')}
      </p>

      <div className="space-y-8">
        {renderTargetSection(
          'stamp_targets',
          t('settings.marketing_group.stamp.stamp_targets'),
          t('settings.marketing_group.stamp_wizard.stamp_targets_desc'),
          true,
          stampTab,
          setStampTab,
        )}

        <div className="border-t border-gray-200" />

        {renderTargetSection(
          'reward_targets',
          t('settings.marketing_group.stamp.reward_targets'),
          t('settings.marketing_group.stamp_wizard.reward_targets_desc'),
          false,
          rewardTab,
          setRewardTab,
        )}
      </div>
    </FormSection>
  );
};
