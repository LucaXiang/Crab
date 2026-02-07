import React, { useMemo } from 'react';
import { CartItem, CheckoutMode } from '@/core/domain/types';
import { useProductStore, useCategories } from '@/core/stores/resources';
import { useI18n } from '@/hooks/useI18n';
import { UnpaidItemRow } from './components';
import { CATEGORY_BG, CATEGORY_HEADER_BG, CATEGORY_ACCENT, hashToColorIndex } from '@/utils/categoryColors';

interface OrderItemsSummaryProps {
  items: CartItem[];
  mode: CheckoutMode;
  selectedQuantities: Record<number, number>;
  onUpdateSelectedQty: (index: number, delta: number) => void;
  onEditItem: (item: CartItem) => void;
}

export const OrderItemsSummary: React.FC<OrderItemsSummaryProps> = ({
  items,
  mode,
  selectedQuantities,
  onUpdateSelectedQty,
  onEditItem,
}) => {
  const { t } = useI18n();
  const products = useProductStore(state => state.items);
  const categories = useCategories();

  // Filter to items with unpaid quantity > 0
  const activeItems = useMemo(() => {
    return items
      .map((item, idx) => {
        if (item._removed) return null;
        const remainingQty = item.unpaid_quantity ?? item.quantity;
        if (remainingQty <= 0) return null;
        return { item, remainingQty, originalIndex: idx };
      })
      .filter((entry): entry is NonNullable<typeof entry> => entry !== null);
  }, [items]);

  // Group by category
  const groupedByCategory = useMemo(() => {
    const productMap = new Map(products.map(p => [String(p.id), p]));
    const groups: Record<string, typeof activeItems> = {};

    activeItems.forEach(entry => {
      const product = productMap.get(entry.item.id);
      const categoryId = product?.category_id != null ? String(product.category_id) : 'uncategorized';

      if (!groups[categoryId]) {
        groups[categoryId] = [];
      }
      groups[categoryId].push(entry);
    });

    // Sort within each group by external_id
    const externalIdMap = new Map<string, number | null>();
    for (const p of products) externalIdMap.set(String(p.id), p.external_id);

    for (const entries of Object.values(groups)) {
      entries.sort((a, b) => {
        // Comped items sink to end of each category group
        const compA = a.item.is_comped ? 1 : 0;
        const compB = b.item.is_comped ? 1 : 0;
        if (compA !== compB) return compA - compB;

        const extIdA = externalIdMap.get(a.item.id) ?? Number.MAX_SAFE_INTEGER;
        const extIdB = externalIdMap.get(b.item.id) ?? Number.MAX_SAFE_INTEGER;
        if (extIdA !== extIdB) return extIdA - extIdB;
        return a.item.name.localeCompare(b.item.name);
      });
    }

    return groups;
  }, [activeItems, products]);

  // Sort groups by category sort_order
  const sortedGroups = useMemo(() => {
    const categoryMap = new Map(categories.map(c => [String(c.id), c]));

    return Object.entries(groupedByCategory).sort(([catIdA], [catIdB]) => {
      if (catIdA === 'uncategorized') return 1;
      if (catIdB === 'uncategorized') return -1;

      const catA = categoryMap.get(catIdA);
      const catB = categoryMap.get(catIdB);

      return (catA?.sort_order ?? 0) - (catB?.sort_order ?? 0);
    });
  }, [groupedByCategory, categories]);

  return (
    <div className="space-y-6">
      {sortedGroups.map(([categoryId, entries]) => {
        const colorIdx = hashToColorIndex(categoryId);
        return (
          <div key={categoryId} className="space-y-3">
            {entries.map(({ item, remainingQty, originalIndex }) => (
              <UnpaidItemRow
                key={`unpaid-${originalIndex}`}
                item={item}
                remainingQty={remainingQty}
                originalIndex={originalIndex}
                mode={mode}
                selectedQuantities={selectedQuantities}
                onUpdateSelectedQty={onUpdateSelectedQty}
                onEditItem={onEditItem}
                bgColor={CATEGORY_BG[colorIdx]}
                hoverColor={CATEGORY_HEADER_BG[colorIdx]}
                accentColor={CATEGORY_ACCENT[colorIdx]}
              />
            ))}
          </div>
        );
      })}
    </div>
  );
};
