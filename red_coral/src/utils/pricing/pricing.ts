/**
 * Unified Pricing Utilities
 *
 * 统一的价格计算工具，使用Currency (Decimal.js) 确保精度
 *
 * 计算顺序：原价 (Base) -> 属性选项 (Options) -> 折扣 (Discount) -> 加价 (Surcharge) -> 最终价 (Final)
 */

import { Currency } from '../currency/currency';
import { CartItem, ItemAttributeSelection } from '@/core/domain/types';
import Decimal from 'decimal.js';

/**
 * 计算属性选项的总价格调整
 * @param selectedOptions 选中的属性选项
 * @returns 总价格调整（可为正或负）
 */
export function calculateOptionsModifier(
  selectedOptions?: ItemAttributeSelection[]
): Decimal {
  if (!selectedOptions || selectedOptions.length === 0) {
    return new Decimal(0);
  }
  return selectedOptions.reduce((total, option) => {
    return Currency.add(total, option.price_modifier);
  }, new Decimal(0));
}

/**
 * 计算折扣金额
 * @param basePrice 基础价格
 * @param discountPercent 折扣百分比 (0-100)
 * @returns 折扣金额
 */
export function calculateDiscountAmount(
  basePrice: number | Decimal,
  discountPercent: number
): Decimal {
  if (!discountPercent || discountPercent <= 0) {
    return new Decimal(0);
  }
  return Currency.floor2(Currency.mul(basePrice, discountPercent / 100));
}

/**
 * 应用折扣后的价格
 * @param basePrice 基础价格
 * @param discountPercent 折扣百分比 (0-100)
 * @returns 折扣后价格
 */
function applyDiscount(
  basePrice: number | Decimal,
  discountPercent: number
): Decimal {
  const discountAmount = calculateDiscountAmount(basePrice, discountPercent);
  return Currency.sub(basePrice, discountAmount);
}

/**
 * 计算商品的最终单价
 * 顺序：原价 -> 应用属性选项 -> 应用折扣 -> 应用加价 -> 最终单价
 *
 * @param item CartItem
 * @returns 最终单价
 */
export function calculateItemFinalPrice(item: CartItem): Decimal {
  let basePrice = new Decimal(item.originalPrice ?? item.price);

  // 2. Apply attribute options modifier
  const optionsModifier = calculateOptionsModifier(item.selectedOptions);
  basePrice = Currency.add(basePrice, optionsModifier);

  // 3. Apply discount
  const afterDiscount = applyDiscount(basePrice, item.discountPercent || 0);

  // 4. Apply surcharge
  const surchargeAmount = item.surcharge || 0;
  const finalPrice = Currency.add(afterDiscount, surchargeAmount);

  return Currency.floor2(finalPrice);
}

/**
 * 计算商品行总价（最终单价 * 数量）
 * @param item CartItem
 * @returns 行总价
 */
export function calculateItemTotal(item: CartItem): Decimal {
  const finalPrice = calculateItemFinalPrice(item);
  return Currency.floor2(Currency.mul(finalPrice, item.quantity));
}

/**
 * 计算订单商品总价
 * @param items CartItem[]
 * @returns 订单总价
 */
export function calculateOrderTotal(items: CartItem[]): Decimal {
  return items.reduce((total, item) => {
    if (item._removed) return total;
    const itemTotal = calculateItemTotal(item);
    return Currency.add(total, itemTotal);
  }, new Decimal(0));
}

/**
 * 计算商品组（按客人分组）的总价
 * @param items CartItem[]
 * @param guestId 客人ID（可选）
 * @returns 分组总价
 */
 

/**
 * 格式化价格为显示字符串
 * @param price 价格（Decimal或number）
 * @param currencySymbol 货币符号（默认$）
 * @returns 格式化后的字符串
 */
 

/**
 * 计算折扣后的节省金额
 * @param basePrice 原价
 * @param discountPercent 折扣百分比
 * @returns 节省的金额
 */
 

/**
 * 检查商品是否有优惠（折扣或加价）
 * @param item CartItem
 * @returns 是否有优惠
 */
 
