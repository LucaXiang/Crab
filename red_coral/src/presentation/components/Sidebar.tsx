import React, { useState, useCallback } from 'react';
import { CartItem } from '@/core/domain/types';
import { CartItemDetailModal } from './modals/CartItemDetailModal';
import { CartFooterActions } from './cart/CartFooterActions';
import { CartCheckoutBar } from './cart/CartCheckoutBar';

// Using new granular selectors
import {
  useCartStore,
  useTotalAmount,
  useIsCartEmpty,
  useCartActions
} from '@/stores';

import {
	  useHeldOrdersCount,
	  useDraftOrdersCount
	} from '@/stores';
import { CartList } from './cart/CartList';

interface SidebarProps {
	  currentOrderNumber: string | null;
	  onManageTable: () => void;
	  onSaveDraft: () => void;
	  onRestoreDraft: () => void;
	  onCheckout: () => void;
	}

const SidebarInner: React.FC<SidebarProps> = ({
	  onManageTable,
	  onSaveDraft,
	  onRestoreDraft,
	  onCheckout,
	}) => {
	  const cartState = useCartStore((state) => state);
	  const cart = cartState.cart;
	  const totalAmount = useTotalAmount();
	  const isCartEmpty = useIsCartEmpty();
	  const { updateCartItem, removeFromCart, clearCart, incrementItemQuantity } = useCartActions();

	  const heldOrdersCount = useHeldOrdersCount();
	  const draftOrdersCount = useDraftOrdersCount();

	  const [editingItem, setEditingItem] = useState<CartItem | null>(null);

	  const handleQuantityChange = useCallback((instanceId: string, delta: number) => {
		incrementItemQuantity(instanceId, delta);
	  }, [incrementItemQuantity]);

	  const handleItemClick = useCallback((item: CartItem) => {
		setEditingItem(item);
	  }, []);

	  const handleCloseModal = useCallback(() => {
		setEditingItem(null);
	  }, []);

	  // Wrap update/remove to match CartItemDetailModal expected signature
	  const handleUpdateItem = useCallback((instanceId: string, updates: Partial<CartItem>, _options?: { userId?: string }) => {
		updateCartItem(instanceId, updates);
	  }, [updateCartItem]);

	  const handleRemoveItem = useCallback((instanceId: string, _options?: { userId?: string }) => {
		removeFromCart(instanceId);
	  }, [removeFromCart]);

	  const handleClearCart = useCallback(() => {
		clearCart();
	  }, [clearCart]);

		return (
			<div
				className="w-full h-full flex flex-col bg-white border-r border-gray-200 shadow-xl relative z-20 font-sans"
			>
	      {editingItem && (
	        <CartItemDetailModal
	          item={editingItem}
	          onClose={handleCloseModal}
	          onUpdate={handleUpdateItem}
	          onRemove={handleRemoveItem}
	        />
	      )}

	         <div className="flex-1 overflow-y-auto bg-white relative">
	        <CartList
	          cart={cart}
	          onQuantityChange={handleQuantityChange}
	          onItemClick={handleItemClick}
	        />
	      </div>


	      <div className="bg-gray-100 shrink-0">
	        <CartFooterActions
	          isCartEmpty={isCartEmpty}
	          heldOrdersCount={heldOrdersCount}
	          draftOrdersCount={draftOrdersCount}
	          onManageTable={onManageTable}
	          onSaveDraft={onSaveDraft}
	          onRestoreDraft={onRestoreDraft}
	          onClear={handleClearCart}
	        />

	        <CartCheckoutBar
	          total={totalAmount}
	          isCartEmpty={isCartEmpty}
	          onCheckout={onCheckout}
	        />
	      </div>
	    </div>
	  );
	};

SidebarInner.displayName = 'Sidebar';

export const Sidebar: React.FC<SidebarProps> = React.memo(
	SidebarInner,
	(prevProps, nextProps) => {
		return (
			prevProps.currentOrderNumber === nextProps.currentOrderNumber &&
			prevProps.onManageTable === nextProps.onManageTable &&
			prevProps.onSaveDraft === nextProps.onSaveDraft &&
			prevProps.onRestoreDraft === nextProps.onRestoreDraft &&
			prevProps.onCheckout === nextProps.onCheckout
		);
	}
);
