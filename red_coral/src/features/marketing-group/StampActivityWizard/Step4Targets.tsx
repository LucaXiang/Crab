import React, { useState, useMemo, useCallback } from 'react';
import { Crosshair, LayoutGrid, Package } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { StampWizardState } from './index';
import type { StampTargetInput, StampTargetType } from '@/core/domain/types/api';
import { FormSection } from '@/shared/components/FormField';
import { useCategoryStore, useProductStore } from '@/core/stores/resources';
import { MultiCardGridSelector } from '@/shared/components/MultiCardGridSelector';

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

        {state.reward_strategy !== 'DESIGNATED' && (
          <>
            <div className="border-t border-gray-200" />

            {renderTargetSection(
              'reward_targets',
              t('settings.marketing_group.stamp.reward_targets'),
              t('settings.marketing_group.stamp_wizard.reward_targets_desc'),
              false,
              rewardTab,
              setRewardTab,
            )}
          </>
        )}
      </div>
    </FormSection>
  );
};
