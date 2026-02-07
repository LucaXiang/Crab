import React from 'react';
import { List } from 'lucide-react';
import { CartItem as CartItemType } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { useCategories, useProducts } from '@/core/stores/resources';
import { CartItem } from './CartItem';
import { CATEGORY_BG, CATEGORY_HEADER_BG, CATEGORY_ACCENT, hashToColorIndex } from '@/utils/categoryColors';

interface CartListProps {
  cart: CartItemType[];
  onQuantityChange: (instanceId: string, delta: number) => void;
  onItemClick: (item: CartItemType) => void;
}

export const CartList = React.memo<CartListProps>(({
  cart,
  onQuantityChange,
  onItemClick
}) => {
  const { t } = useI18n();
  const categories = useCategories();
  const products = useProducts();

  const groupedItems = React.useMemo(() => {
    const groups: Record<string, CartItemType[]> = {};
    const productMap = new Map(products.map(p => [String(p.id), p]));

    cart.forEach(item => {
      const product = productMap.get(item.id);
      const categoryId = product?.category_id != null ? String(product.category_id) : 'uncategorized';

      if (!groups[categoryId]) {
        groups[categoryId] = [];
      }
      groups[categoryId].push(item);
    });

    return groups;
  }, [cart, products]);

  const sortedGroups = React.useMemo(() => {
    const categoryMap = new Map(categories.map(c => [String(c.id), c]));

    return Object.entries(groupedItems).sort(([catIdA], [catIdB]) => {
      if (catIdA === 'uncategorized') return 1;
      if (catIdB === 'uncategorized') return -1;

      const catA = categoryMap.get(catIdA);
      const catB = categoryMap.get(catIdB);

      return (catA?.sort_order ?? 0) - (catB?.sort_order ?? 0);
    });
  }, [groupedItems, categories]);

  const getCategoryName = (categoryId: string) => {
    if (categoryId === 'uncategorized') return t('pos.cart.uncategorized');
    const category = categories.find(c => String(c.id) === categoryId);
    return category?.name || t('pos.cart.unknown_category');
  };

  if (cart.length === 0) {
    return (
      <div className="absolute inset-0 flex flex-col items-center justify-center text-gray-300">
        <div className="w-32 h-32 rounded-full bg-gray-100 mb-6 flex items-center justify-center">
          <List size={48} className="opacity-20" />
        </div>
        <p className="text-gray-400 text-sm tracking-wide">{t('pos.cart.empty')}</p>
      </div>
    );
  }

  return (
    <div className="pb-4">
      <div>
        {sortedGroups.map(([categoryId, items]) => {
          const colorIdx = hashToColorIndex(categoryId);
          return (
            <div key={categoryId} className="mb-0" style={{ backgroundColor: CATEGORY_BG[colorIdx] }}>
              <div
                className="px-4 py-2 text-xs font-semibold sticky top-0 z-10 border-y border-gray-100/50"
                style={{ backgroundColor: CATEGORY_HEADER_BG[colorIdx], color: CATEGORY_ACCENT[colorIdx] }}
              >
                {getCategoryName(categoryId)}
              </div>
              <div className="divide-y divide-gray-100/50">
                {items.map((item) => (
                  <CartItem
                    key={item.instance_id}
                    item={item}
                    onQuantityChange={onQuantityChange}
                    onClick={onItemClick}
                  />
                ))}
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
});

CartList.displayName = 'CartList';
