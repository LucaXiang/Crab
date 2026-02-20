import { Currency } from './currency';

interface BreakdownItem {
  rule_discount_amount: number;
  rule_surcharge_amount: number;
  quantity: number;
  _removed?: boolean;
}

interface BreakdownOrder {
  total_discount: number;
  order_manual_discount_amount: number;
  order_rule_discount_amount: number;
  order_rule_surcharge_amount: number;
}

export interface PriceBreakdown {
  displayItemDiscount: number;
  totalRuleDiscount: number;
  totalRuleSurcharge: number;
  manualItemDiscount: number;
}

export function computePriceBreakdown(items: BreakdownItem[], order: BreakdownOrder): PriceBreakdown {
  const displayItemDiscount = Currency.sub(order.total_discount, order.order_manual_discount_amount).toNumber();

  const activeItems = items.filter(i => !i._removed);
  const itemRuleDiscount = activeItems.reduce(
    (sum, item) => Currency.add(sum, Currency.mul(item.rule_discount_amount, item.quantity)).toNumber(), 0,
  );
  const itemRuleSurcharge = activeItems.reduce(
    (sum, item) => Currency.add(sum, Currency.mul(item.rule_surcharge_amount, item.quantity)).toNumber(), 0,
  );
  const totalRuleDiscount = Currency.add(itemRuleDiscount, order.order_rule_discount_amount).toNumber();
  const totalRuleSurcharge = Currency.add(itemRuleSurcharge, order.order_rule_surcharge_amount).toNumber();
  const manualItemDiscount = Currency.sub(displayItemDiscount, totalRuleDiscount).toNumber();

  return { displayItemDiscount, totalRuleDiscount, totalRuleSurcharge, manualItemDiscount };
}
