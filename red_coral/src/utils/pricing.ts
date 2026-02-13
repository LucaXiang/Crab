import type { ItemOption } from '@/core/domain/types';
import { Currency } from '@/utils/currency';

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
 * 计算草稿购物车项目的 unit_price 和 line_total
 *
 * 与后端 calculate_unit_price 的公式对齐（不含 rule discount/surcharge，
 * 那些由服务端在 AddItems 时计算）:
 *   base_with_options = (original_price || price) + options_modifier
 *   manual_discount = base_with_options × (manual_discount_percent / 100)
 *   unit_price = base_with_options - manual_discount
 *   line_total = unit_price × quantity
 */
export function computeDraftItemPrices(item: {
  price: number;
  original_price?: number;
  quantity: number;
  manual_discount_percent?: number | null;
  selected_options?: ItemOption[] | null;
}): { unit_price: number; line_total: number } {
  const basePrice = item.original_price || item.price;
  const optionsModifier = calculateOptionsModifier(item.selected_options);
  const baseWithOptions = basePrice + optionsModifier;

  const discountPercent = item.manual_discount_percent || 0;
  let unitPrice: number;
  if (discountPercent > 0) {
    const discountFactor = Currency.sub(1, Currency.div(discountPercent, 100));
    unitPrice = Currency.round2(Currency.mul(baseWithOptions, discountFactor)).toNumber();
  } else {
    unitPrice = baseWithOptions;
  }

  const lineTotal = Currency.round2(Currency.mul(unitPrice, item.quantity)).toNumber();
  return { unit_price: unitPrice, line_total: lineTotal };
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
