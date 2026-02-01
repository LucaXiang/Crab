import React from 'react';
import { CartItem, CheckoutMode } from '@/core/domain/types';
import { UnpaidItemRow } from './components';

interface OrderItemsSummaryProps {
  items: CartItem[];
  unpaidItems: CartItem[];
  mode: CheckoutMode;
  selectedQuantities: Record<number, number>;
  onUpdateSelectedQty: (index: number, delta: number) => void;
  onEditItem: (item: CartItem) => void;
}

export const OrderItemsSummary: React.FC<OrderItemsSummaryProps> = ({
  items,
  unpaidItems,
  mode,
  selectedQuantities,
  onUpdateSelectedQty,
  onEditItem,
}) => {
  // Build active (unpaid) items list
  const activeItems: { item: CartItem; remainingQty: number; originalIndex: number }[] = [];

  items.forEach((item) => {
    if (item._removed) return;

    // Find corresponding unpaid item
    const unpaidIdx = unpaidItems.findIndex(u =>
      item.instance_id
        ? u.instance_id === item.instance_id
        : u.id === item.id
    );

    const remainingItem = unpaidIdx !== -1 ? unpaidItems[unpaidIdx] : null;
    const remainingQty = remainingItem ? remainingItem.quantity : 0;

    if (remainingQty > 0) {
      activeItems.push({ item, remainingQty, originalIndex: unpaidIdx });
    }
  });

  // Sorting by external_id then name
  const sortItems = (aItem: CartItem, bItem: CartItem) => {
    const extIdA = aItem.selected_specification?.external_id ? parseInt(String(aItem.selected_specification.external_id), 10) || 0 : 0;
    const extIdB = bItem.selected_specification?.external_id ? parseInt(String(bItem.selected_specification.external_id), 10) || 0 : 0;
    if (extIdA !== extIdB) return extIdA - extIdB;
    return aItem.name.localeCompare(bItem.name);
  };

  activeItems.sort((a, b) => sortItems(a.item, b.item));

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
          />
        ))}
      </div>

    </div>
  );
};
