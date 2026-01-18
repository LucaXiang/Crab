import React from 'react';
import { List } from 'lucide-react';
import { CartItem as CartItemType } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { CartItem } from './CartItem';

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
        <div className="divide-y divide-gray-100">
          {cart.map((item) => {
            return (
              <React.Fragment key={item.instanceId}>
                <CartItem
                  item={item}
                  onQuantityChange={onQuantityChange}
                  onClick={onItemClick}
                />
              </React.Fragment>
            );
          })}
        </div>
      </div>
    </div>
  );
});

CartList.displayName = 'CartList';
