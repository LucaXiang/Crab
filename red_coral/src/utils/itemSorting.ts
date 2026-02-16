/** Calculate sink priority for item sorting: unpaid=0, fully paid=1, comped=2 */
export function calculateItemSink(item: { is_comped?: boolean; unpaid_quantity: number }): number {
  return item.is_comped ? 2 : item.unpaid_quantity === 0 ? 1 : 0;
}
