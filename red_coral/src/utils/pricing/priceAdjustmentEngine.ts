/**
 * Price Adjustment Engine
 * 价格调整规则计算引擎
 *
 * 优先级顺序：PRODUCT > CATEGORY > GLOBAL
 * 相同优先级：按 priority 字段降序，再按创建时间升序
 */

import { Decimal } from 'decimal.js';
import type {
  AdjustmentRule,
  AdjustmentScope,
} from '@/core/domain/types/pricing';

/**
 * 价格调整结果
 */
export interface PriceAdjustmentResult {
  totalAdjustment: Decimal; // 总调整金额（正数为加价，负数为折扣）
  appliedRules: Array<{
    rule: AdjustmentRule;
    adjustment: Decimal; // 该规则的调整金额
  }>;
  isValid: boolean;
  validationResults: TimeValidationResult[];
}

/**
 * 时间验证结果
 */
export interface TimeValidationResult {
  isValid: boolean;
  reason?: 'NOT_STARTED' | 'EXPIRED' | 'OUT_OF_SCHEDULE';
  nextActiveTime?: number;
}

/**
 * 价格计算上下文
 */
export interface PriceCalculationContext {
  productId: string;
  productCategory?: string;
  productTags?: string[];
  zoneId?: string;
  currentTime: number;
}

/**
 * 检查时间是否有效
 */
export function validateTime(
  rule: AdjustmentRule,
  currentTime: number
): TimeValidationResult {
  const { timeMode, validFrom, validTo, config } = rule;

  // PERMANENT - 永久有效
  if (timeMode === 'permanent') {
    return { isValid: true };
  }

  // ONETIME - 一次性，过期失效
  if (timeMode === 'onetime') {
    if (!validFrom || !validTo) {
      return { isValid: false, reason: 'NOT_STARTED' };
    }

    if (currentTime < validFrom) {
      return {
        isValid: false,
        reason: 'NOT_STARTED',
        nextActiveTime: validFrom,
      };
    }

    if (currentTime > validTo) {
      return { isValid: false, reason: 'EXPIRED' };
    }

    return { isValid: true };
  }

  // RECURRING - 周期性生效
  if (timeMode === 'recurring') {
    if (!config?.timeRange) {
      return { isValid: true }; // 无时间配置则始终有效
    }

    const now = new Date(currentTime * 1000);
    const currentHour = now.getHours();
    const currentDay = now.getDay(); // 0 = Sunday, 1 = Monday, ...

    const { startTime, endTime, daysOfWeek } = config.timeRange;

    // 解析时间
    const [startHour, startMinute] = startTime.split(':').map(Number);
    const [endHour, endMinute] = endTime.split(':').map(Number);

    // 检查日期
    if (daysOfWeek && daysOfWeek.length > 0) {
      if (!daysOfWeek.includes(currentDay)) {
        return {
          isValid: false,
          reason: 'OUT_OF_SCHEDULE',
        };
      }
    }

    // 检查小时
    const currentMinutes = currentHour * 60 + now.getMinutes();
    const startMinutes = startHour * 60 + startMinute;
    const endMinutes = endHour * 60 + endMinute;

    // 处理跨天的情况（如 22:00-06:00）
    if (endMinutes < startMinutes) {
      // 跨天
      if (currentMinutes >= startMinutes || currentMinutes < endMinutes) {
        return { isValid: true };
      }
    } else {
      // 不跨天
      if (currentMinutes >= startMinutes && currentMinutes < endMinutes) {
        return { isValid: true };
      }
    }

    return { isValid: false, reason: 'OUT_OF_SCHEDULE' };
  }

  return { isValid: false, reason: 'OUT_OF_SCHEDULE' };
}

/**
 * 计算单个规则的调整金额
 */
export function calculateRuleAdjustment(
  rule: AdjustmentRule,
  basePrice: Decimal,
  quantity: number = 1
): Decimal {
  const { adjustmentType, adjustmentValue, ruleType } = rule;

  let adjustment: Decimal;

  // 根据调整类型计算
  switch (adjustmentType) {
    case 'percentage_discount':
    case 'percentage_surcharge':
      adjustment = basePrice.mul(Math.abs(adjustmentValue) / 100);
      break;
    case 'fixed_discount':
    case 'fixed_surcharge':
      adjustment = new Decimal(Math.abs(adjustmentValue));
      break;
    default:
      adjustment = new Decimal(0);
  }

  // 折扣为负数，加价为正数
  if (ruleType === 'discount') {
    adjustment = adjustment.neg();
  }

  // 乘以数量
  return adjustment.mul(quantity);
}

/**
 * 排序规则（按优先级）
 * 1. 作用域优先级：PRODUCT > CATEGORY > GLOBAL
 * 2. 同级优先级：priority 降序
 * 3. 创建时间：升序
 */
export function sortRulesByPriority(rules: AdjustmentRule[]): AdjustmentRule[] {
  const scopePriority: Record<AdjustmentScope, number> = {
    'product': 4,
    'category': 2,
    'global': 1,
    'order': 0,
  };

  return [...rules].sort((a, b) => {
    // 作用域优先级
    const scopeDiff = (scopePriority[b.scope] || 0) - (scopePriority[a.scope] || 0);
    if (scopeDiff !== 0) return scopeDiff;

    // 自定义优先级
    const priorityDiff = (b.priority || 0) - (a.priority || 0);
    if (priorityDiff !== 0) return priorityDiff;

    // 创建时间（先创建的优先）
    const aTime = a.createdAt ? new Date(a.createdAt).getTime() : 0;
    const bTime = b.createdAt ? new Date(b.createdAt).getTime() : 0;
    return aTime - bTime;
  });
}

/**
 * 过滤有效的规则
 */
export function filterValidRules(
  rules: AdjustmentRule[],
  currentTime: number
): AdjustmentRule[] {
  return rules.filter((rule) => {
    const validation = validateTime(rule, currentTime);
    return validation.isValid;
  });
}

/**
 * 计算价格调整
 * @param basePrice 基础价格
 * @param context 计算上下文
 * @param rules 可用的价格调整规则
 * @returns 调整结果
 */
export function calculatePriceAdjustment(
  basePrice: Decimal,
  context: PriceCalculationContext,
  rules: AdjustmentRule[]
): PriceAdjustmentResult {
  // 1. 过滤当前时间有效的规则
  const validRules = filterValidRules(rules, context.currentTime);

  // 2. 筛选适用于当前商品的规则
  const applicableRules = validRules.filter((rule) => {
    // 检查scope匹配
    switch (rule.scope) {
      case 'global':
        return true;

      case 'category':
        return context.productCategory === rule.targetId;

      case 'product':
        return context.productId === rule.targetId;

      case 'order':
        // 订单级别的规则在订单层面处理
        return false;

      default:
        return false;
    }
  });

  // 3. 按优先级排序
  const sortedRules = sortRulesByPriority(applicableRules);

  // 4. 计算每个规则的调整
  const appliedRules = sortedRules.map((rule) => {
    const adjustment = calculateRuleAdjustment(rule, basePrice, 1);
    return { rule, adjustment };
  });

  // 5. 计算总调整
  const totalAdjustment = appliedRules.reduce(
    (sum, { adjustment }) => sum.add(adjustment),
    new Decimal(0)
  );

  // 6. 验证所有规则的时间有效性
  const validationResults = rules.map((rule) => validateTime(rule, context.currentTime));

  return {
    totalAdjustment,
    appliedRules,
    isValid: validRules.length > 0,
    validationResults,
  };
}

/**
 * 应用价格调整到基础价格
 * @param basePrice 基础价格
 * @param adjustment 调整金额
 * @returns 调整后的价格
 */
export function applyAdjustment(basePrice: Decimal, adjustment: Decimal): Decimal {
  const finalPrice = basePrice.add(adjustment);
  // 确保价格不为负数
  return finalPrice.greaterThan(0) ? finalPrice : new Decimal(0);
}

/**
 * 获取下一个生效时间（用于 UI 显示）
 */
export function getNextActiveTime(
  rule: AdjustmentRule,
  currentTime: number
): number | undefined {
  const validation = validateTime(rule, currentTime);
  return validation.nextActiveTime;
}

/**
 * 检查规则当前是否生效
 */
export function isRuleActive(
  rule: AdjustmentRule,
  currentTime: number
): boolean {
  const validation = validateTime(rule, currentTime);
  return validation.isValid;
}
