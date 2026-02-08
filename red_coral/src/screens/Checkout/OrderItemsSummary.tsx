import React, { useMemo } from 'react';
import { CartItem } from '@/core/domain/types';
import { useProductStore, useCategories } from '@/core/stores/resources';
import { UnpaidItemRow } from './components';
import { CATEGORY_BG, CATEGORY_HEADER_BG, CATEGORY_ACCENT, buildCategoryColorMap } from '@/utils/categoryColors';

interface OrderItemsSummaryProps {
  items: CartItem[];
  onEditItem: (item: CartItem) => void;
}

export const OrderItemsSummary: React.FC<OrderItemsSummaryProps> = ({
  items,
  onEditItem,
}) => {
  const products = useProductStore(state => state.items);
  const categories = useCategories();

  // Filter to items with unpaid quantity > 0
  const activeItems = useMemo(() => {
    return items
      .filter(item => !item._removed && item.unpaid_quantity > 0);
  }, [items]);

  // Group by category
  const groupedByCategory = useMemo(() => {
    const productMap = new Map(products.map(p => [p.id, p]));
    const groups: Record<string, CartItem[]> = {};

    activeItems.forEach(item => {
      const product = productMap.get(item.id);
      const categoryId = product?.category_id != null ? String(product.category_id) : 'uncategorized';

      if (!groups[categoryId]) {
        groups[categoryId] = [];
      }
      groups[categoryId].push(item);
    });

    // Sort within each group by external_id
    const externalIdMap = new Map<number, number | null>();
    for (const p of products) externalIdMap.set(p.id, p.external_id);

    for (const entries of Object.values(groups)) {
      entries.sort((a, b) => {
        // Comped items sink to end of each category group
        const compA = a.is_comped ? 1 : 0;
        const compB = b.is_comped ? 1 : 0;
        if (compA !== compB) return compA - compB;

        const extIdA = externalIdMap.get(a.id) ?? Number.MAX_SAFE_INTEGER;
        const extIdB = externalIdMap.get(b.id) ?? Number.MAX_SAFE_INTEGER;
        if (extIdA !== extIdB) return extIdA - extIdB;
        return a.name.localeCompare(b.name);
      });
    }

    return groups;
  }, [activeItems, products]);

  // Map item id -> external_id for passing to UnpaidItemRow
  const externalIdByItemId = useMemo(() => {
    const map = new Map<number, number | null>();
    for (const p of products) map.set(p.id, p.external_id);
    return map;
  }, [products]);

  const colorMap = useMemo(() => buildCategoryColorMap(categories), [categories]);

  // Sort groups by category sort_order
  const sortedGroups = useMemo(() => {
    const categoryMap = new Map(categories.map(c => [c.id, c]));

    return Object.entries(groupedByCategory).sort(([catIdA], [catIdB]) => {
      if (catIdA === 'uncategorized') return 1;
      if (catIdB === 'uncategorized') return -1;

      const catA = categoryMap.get(Number(catIdA));
      const catB = categoryMap.get(Number(catIdB));

      return (catA?.sort_order ?? 0) - (catB?.sort_order ?? 0);
    });
  }, [groupedByCategory, categories]);

  return (
    <div className="space-y-6">
      {sortedGroups.map(([categoryId, entries]) => {
        const colorIdx = colorMap.get(categoryId) ?? 0;
        return (
          <div key={categoryId} className="space-y-3">
            {entries.map((item) => (
              <UnpaidItemRow
                key={item.instance_id}
                item={item}
                onEditItem={onEditItem}
                externalId={externalIdByItemId.get(item.id)}
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
