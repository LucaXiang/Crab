import React, { useMemo } from 'react';
import { Zap, AlertTriangle, Info } from 'lucide-react';
import type { PriceRule } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';

interface RuleConflictAnalysisProps {
  currentRule: PriceRule;
  allRules: PriceRule[];
}

// Calculate priority score for a rule
// Higher zone specificity + higher product specificity = higher priority
const calculatePriority = (rule: PriceRule): number => {
  // Zone weight: all < retail < specific
  let zoneWeight = 0;
  if (rule.zone_scope === 'zone:all') zoneWeight = 0;
  else if (rule.zone_scope === 'zone:retail') zoneWeight = 1;
  else zoneWeight = 2;

  // Product weight: global < category < tag < product
  const productWeights: Record<string, number> = {
    GLOBAL: 0,
    CATEGORY: 1,
    TAG: 2,
    PRODUCT: 3,
  };
  const productWeight = productWeights[rule.product_scope] || 0;

  // Combine: zone_weight * 10 + product_weight
  return zoneWeight * 10 + productWeight;
};

// Get stacking mode label
const getStackingMode = (rule: PriceRule): 'exclusive' | 'non_stackable' | 'stackable' => {
  if (rule.is_exclusive) return 'exclusive';
  if (rule.is_stackable) return 'stackable';
  return 'non_stackable';
};

export const RuleConflictAnalysis: React.FC<RuleConflictAnalysisProps> = ({
  currentRule,
  allRules,
}) => {
  const { t } = useI18n();

  // Find rules that could potentially conflict (overlap in scope)
  const relatedRules = useMemo(() => {
    return allRules
      .filter(r => r.id !== currentRule.id && r.is_active)
      .filter(r => {
        // Check zone overlap
        const zoneOverlaps =
          r.zone_scope === 'zone:all' ||
          currentRule.zone_scope === 'zone:all' ||
          r.zone_scope === currentRule.zone_scope;

        // Check product scope overlap (simplified)
        const productOverlaps =
          r.product_scope === 'GLOBAL' ||
          currentRule.product_scope === 'GLOBAL' ||
          (r.product_scope === currentRule.product_scope && r.target === currentRule.target);

        return zoneOverlaps && productOverlaps;
      })
      .map(r => ({
        rule: r,
        priority: calculatePriority(r),
        stackingMode: getStackingMode(r),
        isCurrent: false as const,
      }))
      .sort((a, b) => b.priority - a.priority);
  }, [allRules, currentRule]);

  const currentPriority = calculatePriority(currentRule);
  const currentStackingMode = getStackingMode(currentRule);

  // Find potential issues
  const issues = useMemo(() => {
    const result: string[] = [];

    // Check if exclusive rule with higher priority exists
    const higherExclusive = relatedRules.find(
      r => r.stackingMode === 'exclusive' && r.priority > currentPriority
    );
    if (higherExclusive) {
      result.push(
        t('settings.price_rule.conflict.blocked_by_exclusive', {
          name: higherExclusive.rule.display_name,
        })
      );
    }

    // Check if current rule is exclusive and would block others
    if (currentStackingMode === 'exclusive' && relatedRules.length > 0) {
      const blockedCount = relatedRules.filter(r => r.priority < currentPriority).length;
      if (blockedCount > 0) {
        result.push(
          t('settings.price_rule.conflict.will_block', { count: blockedCount.toString() })
        );
      }
    }

    return result;
  }, [relatedRules, currentPriority, currentStackingMode, t]);

  if (relatedRules.length === 0) {
    return (
      <div className="bg-gray-50 rounded-xl p-4">
        <div className="flex items-center gap-2 mb-3">
          <Zap size={16} className="text-gray-500" />
          <span className="text-sm font-medium text-gray-700">
            {t('settings.price_rule.conflict.title')}
          </span>
        </div>
        <div className="text-center py-4 text-sm text-gray-400">
          {t('settings.price_rule.conflict.no_related')}
        </div>
      </div>
    );
  }

  // Insert current rule in the sorted list
  const allSorted = [
    ...relatedRules.filter(r => r.priority > currentPriority),
    { rule: currentRule, priority: currentPriority, stackingMode: currentStackingMode, isCurrent: true },
    ...relatedRules.filter(r => r.priority <= currentPriority),
  ];

  return (
    <div className="bg-gray-50 rounded-xl p-4">
      <div className="flex items-center gap-2 mb-3">
        <Zap size={16} className="text-gray-500" />
        <span className="text-sm font-medium text-gray-700">
          {t('settings.price_rule.conflict.title')}
        </span>
      </div>

      {/* Issues */}
      {issues.length > 0 && (
        <div className="mb-3 space-y-2">
          {issues.map((issue, i) => (
            <div
              key={i}
              className="flex items-start gap-2 p-2 bg-amber-50 border border-amber-200 rounded-lg text-sm text-amber-700"
            >
              <AlertTriangle size={14} className="shrink-0 mt-0.5" />
              <span>{issue}</span>
            </div>
          ))}
        </div>
      )}

      {/* Rule priority list */}
      <div className="text-xs text-gray-500 mb-2">
        {t('settings.price_rule.conflict.priority_order')}:
      </div>

      <div className="bg-white rounded-lg divide-y divide-gray-100">
        {allSorted.map(({ rule, priority, stackingMode, isCurrent }, index) => {
          const isDiscount = rule.rule_type === 'DISCOUNT';

          return (
            <div
              key={rule.id}
              className={`flex items-center gap-3 p-3 ${
                isCurrent ? 'bg-blue-50' : ''
              }`}
            >
              {/* Rank number */}
              <span className="w-5 h-5 rounded-full bg-gray-100 text-gray-500 text-xs flex items-center justify-center shrink-0">
                {index + 1}
              </span>

              {/* Rule indicator */}
              <span
                className={`w-2 h-2 rounded-full shrink-0 ${
                  isDiscount ? 'bg-amber-500' : 'bg-purple-500'
                }`}
              />

              {/* Rule name */}
              <div className="flex-1 min-w-0">
                <span
                  className={`text-sm ${isCurrent ? 'font-medium text-gray-900' : 'text-gray-700'}`}
                >
                  {rule.display_name}
                  {isCurrent && (
                    <span className="text-blue-500 text-xs ml-1">
                      ← {t('settings.price_rule.conflict.current')}
                    </span>
                  )}
                </span>
              </div>

              {/* Adjustment */}
              <span
                className={`text-sm shrink-0 ${
                  isDiscount ? 'text-amber-600' : 'text-purple-600'
                }`}
              >
                {isDiscount ? '-' : '+'}
                {rule.adjustment_type === 'PERCENTAGE'
                  ? `${rule.adjustment_value}%`
                  : `€${rule.adjustment_value.toFixed(2)}`}
              </span>

              {/* Priority */}
              <span className="text-xs text-gray-400 shrink-0 w-8 text-right">P{priority}</span>

              {/* Stacking mode */}
              <span
                className={`text-xs px-2 py-0.5 rounded-full shrink-0 ${
                  stackingMode === 'exclusive'
                    ? 'bg-red-100 text-red-600'
                    : stackingMode === 'stackable'
                      ? 'bg-green-100 text-green-600'
                      : 'bg-gray-100 text-gray-500'
                }`}
              >
                {t(`settings.price_rule.stacking.${stackingMode}`)}
              </span>
            </div>
          );
        })}
      </div>

      {/* Help text */}
      <div className="mt-3 flex items-start gap-2 text-xs text-gray-400">
        <Info size={12} className="shrink-0 mt-0.5" />
        <span>{t('settings.price_rule.conflict.help')}</span>
      </div>
    </div>
  );
};
