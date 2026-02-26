import React from 'react';
import { Pencil, Trash2, Plus, Globe, Layers, Package, Stamp, RefreshCw, ToggleLeft, ToggleRight } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency';
import { Permission } from '@/core/domain/types';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { useCategoryStore } from '@/features/category/store';
import { useProductStore } from '@/core/stores/resources';
import type { MarketingGroupDetail, MgDiscountRule, StampActivityDetail, RewardStrategy } from '@/core/domain/types/api';

// Scope icons
const PRODUCT_SCOPE_ICONS: Record<string, React.ElementType> = {
  GLOBAL: Globe,
  CATEGORY: Layers,
  PRODUCT: Package,
};

const STRATEGY_LABELS: Record<RewardStrategy, string> = {
  ECONOMIZADOR: 'settings.marketing_group.stamp.strategy.economizador',
  GENEROSO: 'settings.marketing_group.stamp.strategy.generoso',
  DESIGNATED: 'settings.marketing_group.stamp.strategy.designated',
};

interface GroupDetailPanelProps {
  detail: MarketingGroupDetail;
  onEditGroup: () => void;
  onDeleteGroup: () => void;
  onAddRule: () => void;
  onEditRule: (rule: MgDiscountRule) => void;
  onDeleteRule: (rule: MgDiscountRule) => void;
  onAddStamp: () => void;
  onEditStamp: (activity: StampActivityDetail) => void;
  onToggleStamp: (activity: StampActivityDetail) => void;
}

export const GroupDetailPanel: React.FC<GroupDetailPanelProps> = ({
  detail,
  onEditGroup,
  onDeleteGroup,
  onAddRule,
  onEditRule,
  onDeleteRule,
  onAddStamp,
  onEditStamp,
  onToggleStamp,
}) => {
  const { t } = useI18n();
  const categories = useCategoryStore((s) => s.items);
  const products = useProductStore((s) => s.items);

  const getTargetName = (targetType: string, targetId: number): string => {
    if (targetType === 'CATEGORY') {
      return categories.find((c) => c.id === targetId)?.name || String(targetId);
    }
    return products.find((p) => p.id === targetId)?.name || String(targetId);
  };

  const getRuleScopeTargetName = (rule: MgDiscountRule): string | null => {
    if (rule.product_scope === 'GLOBAL' || !rule.target_id) return null;
    if (rule.product_scope === 'CATEGORY') {
      return categories.find((c) => c.id === rule.target_id)?.name || String(rule.target_id);
    }
    return products.find((p) => p.id === rule.target_id)?.name || String(rule.target_id);
  };

  return (
    <div className="max-w-2xl mx-auto p-6 space-y-6">
      {/* Header */}
      <div className="flex items-start justify-between">
        <div className="flex-1 min-w-0">
          <h2 className="text-2xl font-bold text-gray-900">{detail.name}</h2>
        </div>
        <ProtectedGate permission={Permission.MARKETING_MANAGE}>
          <div className="flex items-center gap-2 ml-4">
            <button
              onClick={onEditGroup}
              className="p-2 text-gray-500 hover:text-violet-600 hover:bg-violet-50 rounded-lg transition-colors"
            >
              <Pencil size={20} />
            </button>
            <button
              onClick={onDeleteGroup}
              className="p-2 text-gray-500 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
            >
              <Trash2 size={20} />
            </button>
          </div>
        </ProtectedGate>
      </div>

      {/* Group Info Card */}
      {detail.description && (
        <div className="bg-gray-50 rounded-xl p-4">
          <p className="text-sm text-gray-700">{detail.description}</p>
        </div>
      )}

      {/* ── Discount Rules Section ── */}
      <div>
        <div className="flex items-center justify-between mb-3">
          <span className="text-sm font-bold text-gray-700">
            {t('settings.marketing_group.rules')} ({detail.discount_rules.length})
          </span>
          <ProtectedGate permission={Permission.MARKETING_MANAGE}>
            <button
              onClick={onAddRule}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-violet-500 text-white rounded-lg text-sm font-medium hover:bg-violet-600 transition-colors"
            >
              <Plus size={14} />
              {t('settings.marketing_group.add_rule')}
            </button>
          </ProtectedGate>
        </div>

        {detail.discount_rules.length === 0 ? (
          <div className="bg-gray-50 rounded-xl p-8 text-center text-gray-400 text-sm">
            {t('settings.marketing_group.no_rules')}
          </div>
        ) : (
          <div className="space-y-2">
            {detail.discount_rules.map((rule) => {
              const ScopeIcon = PRODUCT_SCOPE_ICONS[rule.product_scope] || Globe;
              const targetName = getRuleScopeTargetName(rule);
              return (
                <div
                  key={rule.id}
                  className={`bg-gray-50 rounded-xl p-3 ${!rule.is_active ? 'opacity-50' : ''}`}
                >
                  {/* Row 1: dot + name + adjustment + actions */}
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 min-w-0">
                      <span
                        className={`w-2 h-2 rounded-full shrink-0 ${
                          !rule.is_active ? 'bg-gray-400' : 'bg-orange-500'
                        }`}
                      />
                      <span className="font-medium text-gray-900 truncate">
                        {rule.name}
                      </span>
                      <span className="text-sm font-bold text-orange-600 shrink-0">
                        {rule.adjustment_type === 'PERCENTAGE'
                          ? `-${rule.adjustment_value}%`
                          : `-${formatCurrency(rule.adjustment_value)}`}
                      </span>
                    </div>
                    <ProtectedGate permission={Permission.MARKETING_MANAGE}>
                      <div className="flex gap-1 shrink-0 ml-2">
                        <button
                          onClick={() => onEditRule(rule)}
                          className="p-1.5 text-gray-400 hover:text-violet-600 hover:bg-violet-50 rounded-lg transition-colors"
                        >
                          <Pencil size={14} />
                        </button>
                        <button
                          onClick={() => onDeleteRule(rule)}
                          className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                        >
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </ProtectedGate>
                  </div>
                  {/* Row 2: scope + receipt name */}
                  <div className="flex items-center gap-2 mt-1.5 text-xs text-gray-500 ml-4">
                    <span className="inline-flex items-center gap-1">
                      <ScopeIcon size={12} />
                      {t(`settings.marketing_group.scope.${rule.product_scope.toLowerCase()}`)}
                      {targetName && ` - ${targetName}`}
                    </span>
                    <span className="text-gray-300">·</span>
                    <span>{rule.receipt_name}</span>
                  </div>
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* ── Stamp Activities Section ── */}
      <div>
        <div className="flex items-center justify-between mb-3">
          <span className="text-sm font-bold text-gray-700">
            {t('settings.marketing_group.stamp_activities')} ({detail.stamp_activities.length})
          </span>
          <ProtectedGate permission={Permission.MARKETING_MANAGE}>
            <button
              onClick={onAddStamp}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-violet-500 text-white rounded-lg text-sm font-medium hover:bg-violet-600 transition-colors"
            >
              <Plus size={14} />
              {t('settings.marketing_group.add_stamp')}
            </button>
          </ProtectedGate>
        </div>

        {detail.stamp_activities.length === 0 ? (
          <div className="bg-gray-50 rounded-xl p-8 text-center text-gray-400 text-sm">
            {t('settings.marketing_group.no_stamps')}
          </div>
        ) : (
          <div className="space-y-2">
            {detail.stamp_activities.map((activity) => {
              const a = activity;
              return (
                <div
                  key={a.id}
                  className={`bg-gray-50 rounded-xl p-3 ${!a.is_active ? 'opacity-50' : ''}`}
                >
                  {/* Row 1: dot + name + actions */}
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-2 min-w-0">
                      <span
                        className={`w-2 h-2 rounded-full shrink-0 ${
                          !a.is_active ? 'bg-gray-400' : 'bg-teal-500'
                        }`}
                      />
                      <span className="font-medium text-gray-900 truncate">
                        {a.name}
                      </span>
                    </div>
                    <ProtectedGate permission={Permission.MARKETING_MANAGE}>
                      <div className="flex gap-1 shrink-0 ml-2">
                        <button
                          onClick={() => onToggleStamp(activity)}
                          className={`p-1.5 rounded-lg transition-colors ${
                            a.is_active
                              ? 'text-teal-500 hover:text-orange-600 hover:bg-orange-50'
                              : 'text-gray-400 hover:text-teal-600 hover:bg-teal-50'
                          }`}
                          title={a.is_active ? t('common.action.disable') : t('common.action.enable')}
                        >
                          {a.is_active ? <ToggleRight size={16} /> : <ToggleLeft size={16} />}
                        </button>
                        <button
                          onClick={() => onEditStamp(activity)}
                          className="p-1.5 text-gray-400 hover:text-violet-600 hover:bg-violet-50 rounded-lg transition-colors"
                        >
                          <Pencil size={14} />
                        </button>
                      </div>
                    </ProtectedGate>
                  </div>
                  {/* Row 2: stamps info */}
                  <div className="flex items-center gap-2 mt-1.5 text-xs text-gray-500 ml-4">
                    <span className="inline-flex items-center gap-1">
                      <Stamp size={12} />
                      {t('settings.marketing_group.stamp.collect_n', { n: a.stamps_required })}
                    </span>
                    <span className="text-gray-300">→</span>
                    <span>
                      {t('settings.marketing_group.stamp.reward_n', { n: a.reward_quantity })}
                    </span>
                    <span className="text-gray-300">·</span>
                    <span>{t(STRATEGY_LABELS[a.reward_strategy])}</span>
                    {a.is_cyclic && (
                      <>
                        <span className="text-gray-300">·</span>
                        <span className="inline-flex items-center gap-0.5 text-teal-600">
                          <RefreshCw size={10} />
                          {t('settings.marketing_group.stamp.cyclic')}
                        </span>
                      </>
                    )}
                  </div>
                  {/* Row 3: targets */}
                  {(activity.stamp_targets.length > 0 || activity.reward_targets.length > 0) && (
                    <div className="flex items-center gap-2 mt-1 text-xs text-gray-400 ml-4">
                      {activity.stamp_targets.length > 0 && (
                        <span>
                          {t('settings.marketing_group.stamp.stamp_targets')}:{' '}
                          {activity.stamp_targets
                            .map((st) => getTargetName(st.target_type, st.target_id))
                            .join(', ')}
                        </span>
                      )}
                      {activity.stamp_targets.length > 0 && activity.reward_targets.length > 0 && (
                        <span className="text-gray-300">·</span>
                      )}
                      {activity.reward_targets.length > 0 && (
                        <span>
                          {t('settings.marketing_group.stamp.reward_targets')}:{' '}
                          {activity.reward_targets
                            .map((rt) => getTargetName(rt.target_type, rt.target_id))
                            .join(', ')}
                        </span>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );
};
