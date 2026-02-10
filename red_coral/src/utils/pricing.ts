import type { ItemOption } from '@/core/domain/types';

/**
 * 计算选项加价总额（考虑选项数量）
 * options_modifier = Σ(price_modifier × quantity)
 */
export function calculateOptionsModifier(options: ItemOption[] | undefined | null): number {
  if (!options || options.length === 0) return 0;
  return options.reduce(
    (sum, opt) => sum + (opt.price_modifier ?? 0) * (opt.quantity ?? 1),
    0
  );
}

/**
 * 生成前端购物车内容 key（与后端 SHA256 instance_id 字段对齐）
 *
 * 用于本地购物车合并：相同商品+价格+折扣+选项+规格 → 相同 key → 合并数量。
 * 不需要和后端 hash 完全一致（后端会重新生成），只需前端内部一致。
 */
export function generateCartKey(
  productId: number,
  price: number,
  discount?: number,
  options?: ItemOption[] | null,
  specId?: number,
): string {
  let key = `${productId}:${price}`;
  if (discount && Math.abs(discount) > 0.01) key += `:d${discount}`;
  if (options && options.length > 0) {
    const sorted = [...options].sort((a, b) => a.attribute_id - b.attribute_id || a.option_id - b.option_id);
    key += `:o${sorted.map(o => `${o.attribute_id}-${o.option_id}-${o.quantity ?? 1}`).join(',')}`;
  }
  if (specId !== undefined) key += `:s${specId}`;
  return key;
}
