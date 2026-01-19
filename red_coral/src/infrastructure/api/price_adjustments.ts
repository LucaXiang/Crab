/**
 * Price Adjustments API
 * Service for managing price adjustment rules
 *
 * NOTE: This module now uses validated API calls from infrastructure/apiValidator.ts
 * All responses are validated with Zod schemas before being returned.
 */

export {
  fetchAdjustmentRules,
  getAdjustmentRule,
  createAdjustmentRule,
  updateAdjustmentRule,
  toggleAdjustmentRule,
  deleteAdjustmentRule,
  getApplicableAdjustmentRules,
  getGlobalAdjustmentRules,
  type AdjustmentRule,
  type AdjustmentConfig,
} from '@/infrastructure/apiValidator';

// Re-export types from new location
export type {
  AdjustmentRuleType,
  AdjustmentType,
  AdjustmentScope,
  AdjustmentStatus,
  CreateAdjustmentRuleParams,
  UpdateAdjustmentRuleParams,
} from '@/core/domain/types/pricing';

// Backward compatibility aliases
import type { AdjustmentRule as PriceAdjustmentRule } from '@/infrastructure/apiValidator';
export type { PriceAdjustmentRule };
