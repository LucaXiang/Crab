import React from 'react';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/formatCurrency';

interface CartCheckoutBarProps {
  total: number;
  isCartEmpty: boolean;
  onCheckout: () => void;
}

export const CartCheckoutBar = React.memo<CartCheckoutBarProps>(({
  total,
  isCartEmpty,
  onCheckout
}) => {
  const { t } = useI18n();

  return (
    <div className="bg-[#FF5E5E] text-white flex h-14 relative z-30 shadow-inner">
      <div className="w-24 flex items-center justify-center text-lg font-medium border-r border-white/20 bg-black/5">
        {t('pos.sidebar.checkout')}
      </div>
      <div
	        className={`flex-1 flex items-center justify-end px-6 text-2xl font-light transition-colors ${
	          isCartEmpty ? 'cursor-default' : 'cursor-pointer hover:bg-white/10'
	        }`}
	        onClick={isCartEmpty ? undefined : onCheckout}
	      >
	        {formatCurrency(total)}
	      </div>
    </div>
  );
});

CartCheckoutBar.displayName = 'CartCheckoutBar';
