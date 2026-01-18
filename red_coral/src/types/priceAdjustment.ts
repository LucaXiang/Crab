/**
 * Price Adjustment Types
 * Unified types for price adjustment rules and configurations
 */

/**
 * Time mode for price adjustments
 */
export type TimeMode = 'onetime' | 'recurring' | 'permanent';

/**
 * Adjustment rule type
 */
export type AdjustmentRuleType = 'discount' | 'special_price' | 'surcharge';

/**
 * Adjustment type - how the adjustment is calculated
 */
export type AdjustmentType =
  | 'percentage_discount'   // 百分比折扣 (如 20% off)
  | 'fixed_discount'        // 固定金额减免 (如 减10元)
  | 'percentage_surcharge'  // 百分比加价
  | 'fixed_surcharge';      // 固定金额加价

/**
 * Adjustment scope - what the adjustment applies to
 */
export type AdjustmentScope = 'global' | 'category' | 'product' | 'order';

/**
 * Price adjustment rule status
 */
export type AdjustmentStatus = 'active' | 'inactive';

/**
 * Price adjustment configuration
 */
export interface AdjustmentConfig {
  /** Time-based settings */
  timeRange?: {
    startTime: string;  // HH:mm format
    endTime: string;
    daysOfWeek?: number[];  // 0-6, Sunday = 0
  };
  /** Quantity-based settings */
  quantityRange?: {
    minQuantity: number;
    maxQuantity?: number;
  };
  /** Conditions */
  conditions?: {
    minOrderAmount?: number;
    maxOrderAmount?: number;
    applyToDiscountedItems?: boolean;
  };
}

/**
 * Price adjustment rule
 */
export interface AdjustmentRule {
  id: number;
  name: string;
  displayName?: string;
  receiptName?: string;
  ruleType: AdjustmentRuleType;
  adjustmentType: AdjustmentType;
  adjustmentValue: number;
  scope: AdjustmentScope;
  targetId?: string;  // For category/product scope
  timeMode?: TimeMode;
  validFrom?: number;  // Unix timestamp
  validTo?: number;    // Unix timestamp
  description?: string;
  priority?: number;
  status: AdjustmentStatus;
  config?: AdjustmentConfig;
  createdAt?: string;
  updatedAt?: string;
}

/**
 * Create price adjustment rule params
 */
export interface CreateAdjustmentRuleParams {
  name: string;
  ruleType: AdjustmentRuleType;
  adjustmentType: AdjustmentType;
  adjustmentValue: number;
  scope: AdjustmentScope;
  targetId?: string;
  timeMode?: TimeMode;
  validFrom?: number;
  validTo?: number;
  description?: string;
  priority?: number;
  status?: AdjustmentStatus;
  config?: AdjustmentConfig;
}

/**
 * Update price adjustment rule params
 */
export interface UpdateAdjustmentRuleParams extends Partial<CreateAdjustmentRuleParams> {
  id: number;
}

/**
 * Price adjustment template (for predefined rules)
 */
export interface AdjustmentTemplate {
  id: number;
  name: string;
  displayName?: string;
  receiptName?: string;
  ruleType: AdjustmentRuleType;
  adjustmentType: AdjustmentType;
  adjustmentValue: number;
  scope: AdjustmentScope;
  timeMode?: TimeMode;
  validFrom?: number;
  validTo?: number;
  description?: string;
  icon?: string;
}

/**
 * Predefined price adjustment templates
 */
export const ADJUSTMENT_TEMPLATES: AdjustmentTemplate[] = [
  {
    id: 1,
    name: 'national_day_promotion',
    displayName: '开业特惠',
    receiptName: '开业折扣',
    ruleType: 'discount',
    adjustmentType: 'percentage_discount',
    adjustmentValue: 20, // 8折 = 减免20%
    scope: 'global',
    timeMode: 'onetime',
    validFrom: Math.floor(new Date('2024-10-01 09:00:00').getTime() / 1000),
    validTo: Math.floor(new Date('2024-10-07 23:59:59').getTime() / 1000),
    description: '国庆节开业活动，全场8折优惠',
  },
  {
    id: 2,
    name: 'double11_promotion',
    displayName: '双11特惠',
    receiptName: '双11折扣',
    ruleType: 'discount',
    adjustmentType: 'percentage_discount',
    adjustmentValue: 10, // 9折 = 减免10%
    scope: 'global',
    timeMode: 'onetime',
    validFrom: Math.floor(new Date('2024-11-11 00:00:00').getTime() / 1000),
    validTo: Math.floor(new Date('2024-11-11 23:59:59').getTime() / 1000),
    description: '双11购物节，全场9折',
  },
  {
    id: 3,
    name: 'breakfast_promotion',
    displayName: '早餐特惠',
    receiptName: '早餐折扣',
    ruleType: 'discount',
    adjustmentType: 'percentage_discount',
    adjustmentValue: 15, // 85折 = 减免15%
    scope: 'global',
    timeMode: 'recurring',
    description: '每天6:00-10:00，早餐全场85折',
  },
  {
    id: 4,
    name: 'afternoon_tea_promotion',
    displayName: '下午茶特惠',
    receiptName: '下午茶折扣',
    ruleType: 'discount',
    adjustmentType: 'percentage_discount',
    adjustmentValue: 10, // 9折 = 减免10%
    scope: 'global',
    timeMode: 'recurring',
    description: '每天14:00-17:00，下午茶全场9折',
  },
  {
    id: 5,
    name: 'member_discount',
    displayName: '会员95折',
    receiptName: '会员折扣',
    ruleType: 'discount',
    adjustmentType: 'percentage_discount',
    adjustmentValue: 5, // 95折 = 减免5%
    scope: 'global',
    timeMode: 'permanent',
    description: '会员专享，全场95折',
  },
  {
    id: 6,
    name: 'min_spend_discount',
    displayName: '满减优惠',
    receiptName: '满减折扣',
    ruleType: 'discount',
    adjustmentType: 'fixed_discount',
    adjustmentValue: 10,
    scope: 'order',
    timeMode: 'permanent',
    description: '消费满100元，减10元',
  },
  {
    id: 7,
    name: 'new_product_promotion',
    displayName: '新品特惠',
    receiptName: '新品折扣',
    ruleType: 'special_price',
    adjustmentType: 'percentage_discount',
    adjustmentValue: 20,
    scope: 'product',
    timeMode: 'onetime',
    description: '新品上市，首周8折尝鲜',
  },
  {
    id: 8,
    name: 'combo_discount',
    displayName: '套餐特价',
    receiptName: '套餐折扣',
    ruleType: 'special_price',
    adjustmentType: 'fixed_discount',
    adjustmentValue: 15,
    scope: 'order',
    timeMode: 'permanent',
    description: '套餐立减15元',
  },
  {
    id: 9,
    name: 'weekday_lunch_promotion',
    displayName: '午餐特惠',
    receiptName: '午餐折扣',
    ruleType: 'discount',
    adjustmentType: 'percentage_discount',
    adjustmentValue: 12,
    scope: 'global',
    timeMode: 'recurring',
    description: '工作日11:00-14:00，午餐全场88折',
  },
  {
    id: 10,
    name: 'happy_hour',
    displayName: '欢乐时光',
    receiptName: '时段折扣',
    ruleType: 'discount',
    adjustmentType: 'percentage_discount',
    adjustmentValue: 20,
    scope: 'global',
    timeMode: 'recurring',
    description: '每天20:00-22:00，酒水8折',
  },
];

// Backward compatibility exports (deprecated)
export type {
  PriceAdjustmentType as PriceAdjustmentTypeDeprecated,
  PriceAdjustmentScope as PriceAdjustmentScopeDeprecated,
  PriceAdjustmentStatus as PriceAdjustmentStatusDeprecated,
  PriceAdjustmentConfig as PriceAdjustmentConfigDeprecated,
  PriceAdjustmentRule as PriceAdjustmentRuleDeprecated,
  CreatePriceAdjustmentRuleParams as CreatePriceAdjustmentRuleParamsDeprecated,
  UpdatePriceAdjustmentRuleParams as UpdatePriceAdjustmentRuleParamsDeprecated,
  PriceAdjustmentTemplate as PriceAdjustmentTemplateDeprecated,
} from './priceAdjustment.deprecated';
