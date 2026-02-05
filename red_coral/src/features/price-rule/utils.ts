import type { PriceRule } from '@/core/domain/types/api';

/**
 * Calculate priority score for a rule.
 * Higher zone specificity + higher product specificity = higher priority.
 *
 * Zone weight: all (0) < retail (1) < specific (2)
 * Product weight: global (0) < category (1) < tag (2) < product (3)
 *
 * Formula: zone_weight * 10 + product_weight
 * Range: 0-23
 */
export const calculatePriority = (rule: PriceRule): number => {
  // Zone weight
  let zoneWeight = 0;
  if (rule.zone_scope === 'zone:all') zoneWeight = 0;
  else if (rule.zone_scope === 'zone:retail') zoneWeight = 1;
  else zoneWeight = 2;

  // Product weight
  const productWeights: Record<string, number> = {
    GLOBAL: 0,
    CATEGORY: 1,
    TAG: 2,
    PRODUCT: 3,
  };
  const productWeight = productWeights[rule.product_scope] || 0;

  return zoneWeight * 10 + productWeight;
};

/**
 * Get stacking mode for a rule.
 */
export const getStackingMode = (rule: PriceRule): 'exclusive' | 'non_stackable' | 'stackable' => {
  if (rule.is_exclusive) return 'exclusive';
  if (rule.is_stackable) return 'stackable';
  return 'non_stackable';
};
