/**
 * 价格调整规则缓存服务 - 前端优化示例
 *
 * 优化策略：
 * 1. 内存缓存：商品规则列表
 * 2. 批量预加载：页面加载时预取常用规则
 * 3. 懒加载：按需加载单个商品的规则
 */

import { useCallback, useEffect, useState } from 'react';
import { getApplicableAdjustmentRules, fetchAdjustmentRules } from '@/services/api/price_adjustments';
import type { AdjustmentRule } from '@/types/priceAdjustment';

// ============================================
// 1. 商品规则缓存 Hook
// ============================================

interface CachedRules {
  rules: AdjustmentRule[];
  timestamp: number;
  productIds: Set<string>;
}

class ProductRulesCache {
  private cache = new Map<string, CachedRules>();
  private readonly MAX_CACHE_SIZE = 1000; // 最多缓存 1000 个商品
  private readonly CACHE_TTL = 5 * 60 * 1000; // 5 分钟过期

  // 获取商品的适用规则（带缓存）
  async getRules(productId: string, _category: string): Promise<AdjustmentRule[]> {
    const cacheKey = `${productId}:${_category}`;
    const cached = this.cache.get(cacheKey);

    if (cached && Date.now() - cached.timestamp < this.CACHE_TTL) {
      return cached.rules;
    }

    // 缓存未命中，从服务器获取
    const rules = await getApplicableAdjustmentRules(productId);

    // 更新缓存
    this.setCache(cacheKey, rules, new Set([productId]));

    return rules;
  }

  // 批量获取多个商品的规则
  async getRulesBatch(productIds: string[]): Promise<Map<string, AdjustmentRule[]>> {
    const result = new Map<string, AdjustmentRule[]>();
    const uncachedIds: string[] = [];

    // 先检查缓存
    for (const id of productIds) {
      const cacheKey = `${id}:${''}`; // 简化，实际应传分类
      const cached = this.cache.get(cacheKey);

      if (cached && Date.now() - cached.timestamp < this.CACHE_TTL) {
        result.set(id, cached.rules);
      } else {
        uncachedIds.push(id);
      }
    }

    // 批量请求未缓存的商品
    if (uncachedIds.length > 0) {
      try {
        // 获取所有规则，用于所有商品
        const allRules = await fetchAdjustmentRules();

        // 为每个商品获取其规则
        await Promise.all(
          uncachedIds.map(async (id) => {
            const productRules = allRules.filter(
              r => (r.scope === 'global' || r.scope === 'product') && r.status === 'active'
            );
            result.set(id, productRules);
            this.setCache(`${id}:${''}`, productRules, new Set([id]));
          })
        );
      } catch (error) {
        console.error('批量获取规则失败:', error);
        // 降级为单条请求
        for (const id of uncachedIds) {
          const rules = await this.getRules(id, '');
          result.set(id, rules);
        }
      }
    }

    return result;
  }

  // 设置缓存
  private setCache(key: string, rules: AdjustmentRule[], productIds: Set<string>): void {
    // 缓存已满，清理最旧的条目
    if (this.cache.size >= this.MAX_CACHE_SIZE) {
      const oldestKey = Array.from(this.cache.entries())
        .sort((a, b) => a[1].timestamp - b[1].timestamp)[0]?.[0];
      if (oldestKey) {
        this.cache.delete(oldestKey);
      }
    }

    this.cache.set(key, {
      rules,
      timestamp: Date.now(),
      productIds,
    });
  }

  // 清除缓存
  clear(): void {
    this.cache.clear();
  }

  // 规则变更时失效相关缓存
  invalidate(_ruleId: string): void {
    // 简化处理：清除所有缓存
    // 生产环境可以维护 ruleId -> productIds 的映射，精准失效
    this.clear();
  }
}

// 全局缓存实例
export const productRulesCache = new ProductRulesCache();

// ============================================
// 2. React Hooks
// ============================================

/**
 * 使用商品的适用规则（带缓存）
 */
export function useProductRules(productId: string, category: string) {
  const [rules, setRules] = useState<AdjustmentRule[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await productRulesCache.getRules(productId, category);
      setRules(result);
    } catch (e) {
      setError(e instanceof Error ? e.message : '获取规则失败');
    } finally {
      setLoading(false);
    }
  }, [productId, category]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { rules, loading, error, refresh };
}

/**
 * 批量使用商品的适用规则（带缓存）
 */
export function useProductRulesBatch(productIds: string[]) {
  const [rulesMap, setRulesMap] = useState<Map<string, AdjustmentRule[]>>(new Map());
  const [loading, setLoading] = useState(false);

  const refresh = useCallback(async () => {
    if (productIds.length === 0) return;

    setLoading(true);
    try {
      const result = await productRulesCache.getRulesBatch(productIds);
      setRulesMap(result);
    } catch (e) {
      console.error('批量获取规则失败:', e);
    } finally {
      setLoading(false);
    }
  }, [productIds]);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { rulesMap, loading, refresh };
}

// ============================================
// 3. 价格计算示例
// ============================================

/**
 * 计算商品最终价格（应用所有适用规则）
 */
export function calculateFinalPrice(
  basePrice: number,
  rules: AdjustmentRule[]
): { finalPrice: number; appliedRules: AdjustmentRule[] } {
  let price = basePrice;
  const appliedRules: AdjustmentRule[] = [];

  // 按优先级排序（scope_priority 已确保顺序，但确保稳定性）
  const sortedRules = [...rules].sort((a, b) => {
    // 先按 scope 优先级
    const scopePriority: Record<string, number> = {
      'product': 100,
      'category': 60,
      'global': 40,
      'order': 20,
    };
    const aScopePrio = scopePriority[a.scope] || 0;
    const bScopePrio = scopePriority[b.scope] || 0;

    if (aScopePrio !== bScopePrio) return bScopePrio - aScopePrio;

    // 再按 priority
    return (b.priority || 0) - (a.priority || 0);
  });

  // 依次应用规则
  for (const rule of sortedRules) {
    if (rule.adjustmentType === 'percentage_discount') {
      price *= (1 - rule.adjustmentValue / 100);
    } else if (rule.adjustmentType === 'percentage_surcharge') {
      price *= (1 + rule.adjustmentValue / 100);
    } else if (rule.adjustmentType === 'fixed_discount') {
      price -= rule.adjustmentValue;
    } else if (rule.adjustmentType === 'fixed_surcharge') {
      price += rule.adjustmentValue;
    }
    appliedRules.push(rule);
  }

  return {
    finalPrice: Math.max(0, price), // 确保不为负数
    appliedRules,
  };
}

// ============================================
// 4. 订单加载时预热缓存
// ============================================

/**
 * 预加载订单中所有商品的规则缓存
 */
export function warmUpOrderRulesCache(orderItems: Array<{ productId: string; category: string }>) {
  const productIds = orderItems.map(item => item.productId);
  productRulesCache.getRulesBatch(productIds);
}
