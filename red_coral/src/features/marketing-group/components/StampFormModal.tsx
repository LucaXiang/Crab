import React, { useState, useMemo, useCallback } from 'react';
import {
  X, Check, FileText, Settings, Award, Crosshair,
  RefreshCw, Coins, Crown, Package, LayoutGrid,
} from 'lucide-react';
import { MultiCardGridSelector } from '@/shared/components/MultiCardGridSelector';
import { useI18n } from '@/hooks/useI18n';
import { useCategoryStore } from '@/features/category/store';
import { useProductStore } from '@/core/stores/resources';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { FormSection, FormField, inputClass, selectClass } from '@/shared/components/FormField';
import { updateStampActivity } from '../mutations';
import type {
  StampActivityDetail,
  StampActivityCreate,
  StampTargetInput,
  StampTargetType,
  RewardStrategy,
} from '@/core/domain/types/api';

// ── Shared: Strategy card options ──

const strategyOptions: { value: RewardStrategy; icon: typeof Coins; variant: 'green' | 'amber' | 'blue' }[] = [
  { value: 'ECONOMIZADOR', icon: Coins, variant: 'green' },
  { value: 'GENEROSO', icon: Crown, variant: 'amber' },
  { value: 'DESIGNATED', icon: Package, variant: 'blue' },
];

const variantStyles = {
  green: {
    border: 'border-green-500', bg: 'bg-green-50', ring: 'ring-green-100',
    iconBg: 'bg-green-100', iconText: 'text-green-600', titleText: 'text-green-700',
    checkBg: 'bg-green-500',
  },
  amber: {
    border: 'border-amber-500', bg: 'bg-amber-50', ring: 'ring-amber-100',
    iconBg: 'bg-amber-100', iconText: 'text-amber-600', titleText: 'text-amber-700',
    checkBg: 'bg-amber-500',
  },
  blue: {
    border: 'border-blue-500', bg: 'bg-blue-50', ring: 'ring-blue-100',
    iconBg: 'bg-blue-100', iconText: 'text-blue-600', titleText: 'text-blue-700',
    checkBg: 'bg-blue-500',
  },
};

// ── Target section with tab toggle ──

type TargetTab = 'CATEGORY' | 'PRODUCT';

function TargetSection({
  field,
  label,
  description,
  required,
  targets,
  onToggle,
  categories,
  products,
  categoryMap,
  getSelectedIds,
}: {
  field: string;
  label: string;
  description: string;
  required: boolean;
  targets: StampTargetInput[];
  onToggle: (type: StampTargetType, id: number) => void;
  categories: { id: number; name: string }[];
  products: { id: number; name: string; category_id?: number }[];
  categoryMap: Map<number, string>;
  getSelectedIds: (targets: StampTargetInput[], type: StampTargetType) => Set<number>;
}) {
  const { t } = useI18n();
  const [activeTab, setTab] = useState<TargetTab>('CATEGORY');

  const catIds = getSelectedIds(targets, 'CATEGORY');
  const prodIds = getSelectedIds(targets, 'PRODUCT');
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

      <div className="flex gap-2">
        {(['CATEGORY', 'PRODUCT'] as const).map((tab) => {
          const count = tab === 'CATEGORY' ? catIds.size : prodIds.size;
          const Icon = tab === 'CATEGORY' ? LayoutGrid : Package;
          return (
            <button
              key={tab}
              type="button"
              onClick={() => setTab(tab)}
              className={`flex items-center gap-1.5 px-4 py-2 rounded-xl text-sm font-medium transition-all ${
                activeTab === tab
                  ? 'bg-teal-50 text-teal-700 border-2 border-teal-500'
                  : 'bg-gray-50 text-gray-600 border-2 border-transparent hover:bg-gray-100'
              }`}
            >
              <Icon size={16} />
              {t(`settings.marketing_group.stamp.target_type.${tab.toLowerCase()}`)}
              {count > 0 && (
                <span className={`text-xs px-1.5 py-0.5 rounded-full ${
                  activeTab === tab ? 'bg-teal-200 text-teal-800' : 'bg-gray-200 text-gray-600'
                }`}>
                  {count}
                </span>
              )}
            </button>
          );
        })}
      </div>

      {activeTab === 'CATEGORY' ? (
        <MultiCardGridSelector
          key={`${field}-cat`}
          items={categories}
          selectedIds={catIds}
          onToggle={(id) => onToggle('CATEGORY', id)}
          searchPlaceholder={t('common.action.search')}
          emptyText={t('common.empty.no_results')}
        />
      ) : (
        <MultiCardGridSelector
          key={`${field}-prod`}
          items={products}
          selectedIds={prodIds}
          onToggle={(id) => onToggle('PRODUCT', id)}
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
}

// ── Main Edit Modal ──

interface StampEditModalProps {
  activity: StampActivityDetail;
  groupId: number;
  onClose: () => void;
  onSuccess: () => void;
}

export const StampEditModal: React.FC<StampEditModalProps> = ({
  activity,
  groupId,
  onClose,
  onSuccess,
}) => {
  const { t } = useI18n();
  const categories = useCategoryStore((s) => s.items);
  const products = useProductStore((s) => s.items);
  const [saving, setSaving] = useState(false);

  // ── Form state ──
  const [name, setName] = useState(activity.name);
  const [stampsRequired, setStampsRequired] = useState(activity.stamps_required);
  const [rewardQuantity, setRewardQuantity] = useState(activity.reward_quantity);
  const [isCyclic, setIsCyclic] = useState(activity.is_cyclic);
  const [rewardStrategy, setRewardStrategy] = useState<RewardStrategy>(activity.reward_strategy);
  const [designatedProductId, setDesignatedProductId] = useState<number | null>(
    activity.designated_product_id ?? null,
  );
  const [stampTargets, setStampTargets] = useState<StampTargetInput[]>(
    activity.stamp_targets.map((st) => ({ target_type: st.target_type, target_id: st.target_id })),
  );
  const [rewardTargets, setRewardTargets] = useState<StampTargetInput[]>(
    activity.reward_targets.map((rt) => ({ target_type: rt.target_type, target_id: rt.target_id })),
  );

  const categoryMap = useMemo(() => {
    const map = new Map<number, string>();
    categories.forEach((cat) => map.set(cat.id, cat.name));
    return map;
  }, [categories]);

  const getSelectedIds = useCallback((targets: StampTargetInput[], type: StampTargetType): Set<number> => {
    return new Set(targets.filter((t) => t.target_type === type).map((t) => t.target_id));
  }, []);

  const toggleTarget = useCallback(
    (field: 'stamp' | 'reward', type: StampTargetType, id: number) => {
      const setFn = field === 'stamp' ? setStampTargets : setRewardTargets;
      setFn((prev) => {
        const exists = prev.some((t) => t.target_type === type && t.target_id === id);
        if (exists) {
          return prev.filter((t) => !(t.target_type === type && t.target_id === id));
        }
        return [...prev, { target_type: type, target_id: id }];
      });
    },
    [],
  );

  const canSave =
    name.trim() !== '' &&
    stampsRequired > 0 &&
    rewardQuantity > 0 &&
    stampTargets.length > 0 &&
    (rewardStrategy !== 'DESIGNATED' || designatedProductId != null);

  const handleSave = async () => {
    if (!canSave) return;
    setSaving(true);
    try {
      const payload: StampActivityCreate = {
        name: name.trim(),
        stamps_required: stampsRequired,
        reward_quantity: rewardQuantity,
        reward_strategy: rewardStrategy,
        designated_product_id: rewardStrategy === 'DESIGNATED' ? designatedProductId : null,
        is_cyclic: isCyclic,
        stamp_targets: stampTargets,
        reward_targets: rewardStrategy === 'DESIGNATED' ? [] : rewardTargets,
      };
      await updateStampActivity(groupId, activity.id, payload);
      toast.success(t('settings.marketing_group.message.stamp_updated'));
      onSuccess();
    } catch (e) {
      logger.error('Failed to update stamp activity', e);
      toast.error(t('common.message.save_failed'));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[90vh] overflow-hidden animate-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="shrink-0 px-6 py-4 border-b border-gray-100 bg-gradient-to-r from-teal-50 to-white flex items-center justify-between">
          <h2 className="text-lg font-bold text-gray-900">
            {t('settings.marketing_group.edit_stamp')}
          </h2>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-xl transition-colors">
            <X size={18} className="text-gray-500" />
          </button>
        </div>

        {/* Scrollable content - all sections flat */}
        <div className="flex-1 overflow-y-auto p-6 space-y-8">

          {/* ── Section 1: Naming ── */}
          <FormSection title={t('settings.marketing_group.stamp_wizard.step1_section')} icon={FileText}>
            <div className="grid grid-cols-2 gap-4">
              <FormField label={t('settings.marketing_group.field.name')} required>
                <input
                  type="text"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  className={inputClass}
                  placeholder="coffee_stamp"
                />
                <p className="mt-1 text-xs text-gray-500">
                  {t('settings.marketing_group.stamp_wizard.name_hint')}
                </p>
              </FormField>
            </div>
          </FormSection>

          {/* ── Section 2: Config ── */}
          <FormSection title={t('settings.marketing_group.stamp_wizard.step2_section')} icon={Settings}>
            <div className="grid grid-cols-2 gap-4 mb-4">
              <FormField label={t('settings.marketing_group.stamp.stamps_required')} required>
                <input
                  type="number"
                  value={stampsRequired}
                  onChange={(e) => setStampsRequired(Number(e.target.value) || 0)}
                  min={1}
                  className={inputClass}
                />
                <p className="mt-1 text-xs text-gray-500">
                  {t('settings.marketing_group.stamp_wizard.stamps_required_hint')}
                </p>
              </FormField>
              <FormField label={t('settings.marketing_group.stamp.reward_quantity')} required>
                <input
                  type="number"
                  value={rewardQuantity}
                  onChange={(e) => setRewardQuantity(Number(e.target.value) || 0)}
                  min={1}
                  className={inputClass}
                />
                <p className="mt-1 text-xs text-gray-500">
                  {t('settings.marketing_group.stamp_wizard.reward_quantity_hint')}
                </p>
              </FormField>
            </div>

            {/* Cyclic Toggle */}
            <div className="flex items-center justify-between p-4 bg-gray-50 rounded-xl border border-gray-100">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-lg bg-teal-100 flex items-center justify-center">
                  <RefreshCw size={20} className="text-teal-600" />
                </div>
                <div>
                  <span className="text-sm font-medium text-gray-700 block">
                    {t('settings.marketing_group.stamp.is_cyclic')}
                  </span>
                  <p className="text-xs text-gray-400 mt-0.5">
                    {t('settings.marketing_group.stamp.cyclic_hint')}
                  </p>
                </div>
              </div>
              <button
                type="button"
                onClick={() => setIsCyclic(!isCyclic)}
                className={`px-4 py-2 rounded-full text-sm font-medium transition-colors ${
                  isCyclic
                    ? 'bg-teal-100 text-teal-700'
                    : 'bg-gray-200 text-gray-500'
                }`}
              >
                {isCyclic ? t('common.status.enabled') : t('common.status.disabled')}
              </button>
            </div>

            {/* Preview */}
            <div className="p-4 bg-violet-50 rounded-xl border border-violet-100 mt-4">
              <p className="text-sm text-violet-700">
                <span className="font-medium">{t('settings.price_rule.wizard.preview')}: </span>
                {t('settings.marketing_group.stamp_wizard.config_preview', {
                  stamps: stampsRequired,
                  reward: rewardQuantity,
                  cyclic: isCyclic
                    ? t('settings.marketing_group.stamp.cyclic')
                    : t('settings.marketing_group.stamp_wizard.one_time'),
                })}
              </p>
            </div>
          </FormSection>

          {/* ── Section 3: Strategy ── */}
          <FormSection title={t('settings.marketing_group.stamp_wizard.step3_section')} icon={Award}>
            <div className="grid grid-cols-3 gap-4 mb-4">
              {strategyOptions.map(({ value, icon: Icon, variant }) => {
                const styles = variantStyles[variant];
                const isSelected = rewardStrategy === value;
                return (
                  <button
                    key={value}
                    type="button"
                    onClick={() => setRewardStrategy(value)}
                    className={`relative flex flex-col items-center p-5 rounded-xl border-2 transition-all ${
                      isSelected
                        ? `${styles.border} ${styles.bg} ring-4 ${styles.ring}`
                        : 'border-gray-200 bg-white hover:border-gray-300 hover:bg-gray-50'
                    }`}
                  >
                    <div className={`w-14 h-14 rounded-full flex items-center justify-center mb-3 ${
                      isSelected ? styles.iconBg : 'bg-gray-100'
                    }`}>
                      <Icon size={28} className={isSelected ? styles.iconText : 'text-gray-400'} />
                    </div>
                    <h4 className={`text-sm font-bold mb-1 ${isSelected ? styles.titleText : 'text-gray-700'}`}>
                      {t(`settings.marketing_group.stamp.strategy.${value.toLowerCase()}`)}
                    </h4>
                    <p className="text-xs text-gray-500 text-center">
                      {t(`settings.marketing_group.stamp_wizard.strategy_${value.toLowerCase()}_desc`)}
                    </p>
                    {isSelected && (
                      <div className={`absolute top-2 right-2 w-5 h-5 rounded-full ${styles.checkBg} flex items-center justify-center`}>
                        <svg className="w-3 h-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                        </svg>
                      </div>
                    )}
                  </button>
                );
              })}
            </div>

            {rewardStrategy === 'DESIGNATED' && (
              <FormField label={t('settings.marketing_group.stamp.designated_product')} required>
                <select
                  value={designatedProductId ?? ''}
                  onChange={(e) => setDesignatedProductId(e.target.value ? Number(e.target.value) : null)}
                  className={selectClass}
                >
                  <option value="">{t('common.hint.select')}</option>
                  {products.map((p) => (
                    <option key={p.id} value={p.id}>{p.name}</option>
                  ))}
                </select>
              </FormField>
            )}
          </FormSection>

          {/* ── Section 4: Targets ── */}
          <FormSection title={t('settings.marketing_group.stamp_wizard.step4_section')} icon={Crosshair}>
            <div className="space-y-8">
              <TargetSection
                field="stamp"
                label={t('settings.marketing_group.stamp.stamp_targets')}
                description={t('settings.marketing_group.stamp_wizard.stamp_targets_desc')}
                required
                targets={stampTargets}
                onToggle={(type, id) => toggleTarget('stamp', type, id)}
                categories={categories}
                products={products}
                categoryMap={categoryMap}
                getSelectedIds={getSelectedIds}
              />

              {rewardStrategy !== 'DESIGNATED' && (
                <>
                  <div className="border-t border-gray-200" />
                  <TargetSection
                    field="reward"
                    label={t('settings.marketing_group.stamp.reward_targets')}
                    description={t('settings.marketing_group.stamp_wizard.reward_targets_desc')}
                    required={false}
                    targets={rewardTargets}
                    onToggle={(type, id) => toggleTarget('reward', type, id)}
                    categories={categories}
                    products={products}
                    categoryMap={categoryMap}
                    getSelectedIds={getSelectedIds}
                  />
                </>
              )}
            </div>
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
            className="flex items-center gap-2 px-5 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-semibold hover:bg-teal-700 transition-colors shadow-lg shadow-teal-600/20 disabled:opacity-50"
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
