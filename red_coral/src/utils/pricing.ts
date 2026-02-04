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
