/**
 * @deprecated These types are kept for backward compatibility only.
 * Use the types from priceAdjustment.ts instead.
 */

// Re-export from main module to ensure compatibility
export * from './priceAdjustment';

// Legacy type aliases (deprecated)
import type {
  AdjustmentType,
  AdjustmentScope,
  AdjustmentStatus,
  AdjustmentConfig,
  AdjustmentRule,
  CreateAdjustmentRuleParams,
  UpdateAdjustmentRuleParams,
  AdjustmentTemplate,
} from './priceAdjustment';

export type PriceAdjustmentType = AdjustmentType;
export type PriceAdjustmentScope = AdjustmentScope;
export type PriceAdjustmentStatus = AdjustmentStatus;
export type PriceAdjustmentConfig = AdjustmentConfig;
export type PriceAdjustmentRule = AdjustmentRule;
export type CreatePriceAdjustmentRuleParams = CreateAdjustmentRuleParams;
export type UpdatePriceAdjustmentRuleParams = UpdateAdjustmentRuleParams;
export type PriceAdjustmentTemplate = AdjustmentTemplate;
