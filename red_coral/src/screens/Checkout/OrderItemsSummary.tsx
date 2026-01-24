import React from 'react';
import { CartItem, CheckoutMode } from '@/core/domain/types';
import { CheckCircle } from 'lucide-react';
import { UnpaidItemRow, PaidItemRow } from './components';

interface OrderItemsSummaryProps {
  items: CartItem[];
  unpaidItems: CartItem[];
  mode: CheckoutMode;
  selectedQuantities: Record<number, number>;
  onUpdateSelectedQty: (index: number, delta: number) => void;
  onEditItem: (item: CartItem) => void;
  t: (key: string) => string;
  surchargeExempt?: boolean;
  paid_item_quantities?: Record<string, number>;
}

export const OrderItemsSummary: React.FC<OrderItemsSummaryProps> = ({
  items,
  unpaidItems,
  mode,
  selectedQuantities,
  onUpdateSelectedQty,
  onEditItem,
  t,
  surchargeExempt,
  paid_item_quantities
}) => {
  // Separate into Active (Unpaid) and Completed (Paid) lists
  const activeItems: { item: CartItem; remainingQty: number; originalIndex: number }[] = [];
  const paidItemsMap: Record<string, { item: CartItem; paidQty: number }> = {};

  items.forEach((item) => {
    // Calculate paid quantity
    let paidQty = 0;
    if (paid_item_quantities && item.instance_id && paid_item_quantities[item.instance_id] !== undefined) {
      paidQty = paid_item_quantities[item.instance_id];
    }

    // Skip removed items with no paid quantity
    if (item._removed && paidQty <= 0) return;

    // Find corresponding unpaid item
    const unpaidIdx = unpaidItems.findIndex(u =>
      item.instance_id
        ? u.instance_id === item.instance_id
        : u.id === item.id
    );

    const remainingItem = unpaidIdx !== -1 ? unpaidItems[unpaidIdx] : null;
    const remainingQty = remainingItem ? remainingItem.quantity : 0;

    // Fallback calculation for legacy items
    if ((!paid_item_quantities || !item.instance_id || paid_item_quantities[item.instance_id] === undefined) && !item._removed) {
      paidQty = item.quantity - remainingQty;
    }

    // Add to Active List if there's remaining quantity
    if (remainingQty > 0 && !item._removed) {
      activeItems.push({ item, remainingQty, originalIndex: unpaidIdx });
    }

    // Add to Paid Map if there's paid quantity
    if (paidQty > 0) {
      const effectiveUnitPrice = surchargeExempt
        ? (item.unit_price ?? item.price) - (item.surcharge ?? 0)
        : (item.unit_price ?? item.price);

      const key = `${item.id}-${effectiveUnitPrice.toFixed(2)}`;

      if (!paidItemsMap[key]) {
        paidItemsMap[key] = { item, paidQty: 0 };
      }
      paidItemsMap[key].paidQty += paidQty;
    }
  });

  const completedItems = Object.values(paidItemsMap);

  // Sorting by external_id then name
  const sortItems = (aItem: CartItem, bItem: CartItem) => {
    const extIdA = aItem.selected_specification?.external_id ? parseInt(String(aItem.selected_specification.external_id), 10) || 0 : 0;
    const extIdB = bItem.selected_specification?.external_id ? parseInt(String(bItem.selected_specification.external_id), 10) || 0 : 0;
    if (extIdA !== extIdB) return extIdA - extIdB;
    return aItem.name.localeCompare(bItem.name);
  };

  activeItems.sort((a, b) => sortItems(a.item, b.item));
  completedItems.sort((a, b) => sortItems(a.item, b.item));

  return (
    <div className="space-y-6">
      {/* Unpaid Items */}
      <div className="space-y-3">
        {activeItems.map(({ item, remainingQty, originalIndex }, idx) => (
          <UnpaidItemRow
            key={`unpaid-${idx}`}
            item={item}
            remainingQty={remainingQty}
            originalIndex={originalIndex}
            mode={mode}
            selectedQuantities={selectedQuantities}
            onUpdateSelectedQty={onUpdateSelectedQty}
            onEditItem={onEditItem}
            surchargeExempt={surchargeExempt}
          />
        ))}
      </div>

      {/* Paid Items */}
      {completedItems.length > 0 && (
        <div className="border-t border-gray-100 pt-6 mt-2">
          <div className="text-xs font-bold text-gray-400 uppercase tracking-wider mb-4 flex items-center gap-2">
            <CheckCircle size={14} className="text-green-500" />
            {t('checkout.items.paid')}
          </div>
          <div className="space-y-3">
            {completedItems.map(({ item, paidQty }, idx) => (
              <PaidItemRow
                key={`paid-${idx}`}
                item={item}
                paidQty={paidQty}
                surchargeExempt={surchargeExempt}
                t={t}
              />
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
