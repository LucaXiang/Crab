import React from 'react';
import { CartItem, CheckoutMode } from '@/core/domain/types';
import { useProductStore } from '@/core/stores/resources';
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

  // Build product ID → external_id map for O(1) lookup during sort
  const externalIdMap = useProductStore(state => {
    const map = new Map<string, number | null>();
    for (const p of state.items) map.set(p.id, p.external_id);
    return map;
  });
  const sortItems = (aItem: CartItem, bItem: CartItem) => {
    const extIdA = externalIdMap.get(aItem.id) ?? Number.MAX_SAFE_INTEGER;
    const extIdB = externalIdMap.get(bItem.id) ?? Number.MAX_SAFE_INTEGER;
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
