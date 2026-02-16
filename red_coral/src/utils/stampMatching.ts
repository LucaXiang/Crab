import type { MemberStampProgressDetail } from '@/core/domain/types/api';
import type { CartItemSnapshot } from '@/core/domain/types/orderEvent';

/** Find order items matching stamp reward_targets that are not comped */
export function getMatchingItems(items: CartItemSnapshot[], sp: MemberStampProgressDetail): CartItemSnapshot[] {
  return items.filter(item =>
    !item.is_comped && sp.reward_targets.some(rt =>
      rt.target_type === 'PRODUCT' ? rt.target_id === item.id
      : rt.target_type === 'CATEGORY' ? rt.target_id === item.category_id
      : false
    ),
  );
}

/** Find order items matching designated_product_id that are not comped */
export function getDesignatedMatchingItems(items: CartItemSnapshot[], sp: MemberStampProgressDetail): CartItemSnapshot[] {
  if (!sp.designated_product_id) return [];
  return items.filter(item => !item.is_comped && item.id === sp.designated_product_id);
}
