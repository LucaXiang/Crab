import { useCallback, useMemo } from 'react';
import { CartItem, DraftOrder } from '@/core/domain/types';
import { useCartStore } from '@/core/stores/cart/useCartStore';

interface UseDraftHandlersParams {
  saveDraft: (draft: DraftOrder) => void;
  restoreDraft: (id: string) => CartItem[];
  deleteDraft: (id: string) => void;
  clearCart: () => void;
  setCart: (items: CartItem[]) => void;
  setShowDraftModal: (v: boolean) => void;
  setCurrentOrderKey: (key: string | null) => void;
}

export function useDraftHandlers(params: UseDraftHandlersParams) {
  const {
    saveDraft,
    restoreDraft,
    deleteDraft,
    clearCart,
    setCart,
    setShowDraftModal,
    setCurrentOrderKey,
  } = params;

  const handleSaveDraft = useCallback(() => {
    const { cart, totalAmount } = useCartStore.getState();
    const draft: DraftOrder = {
      id: `draft-${Date.now()}`,
      items: cart,
      total: totalAmount,
      subtotal: totalAmount,
      tax: 0,
      discount: 0,
      payments: [],
      guestCount: 0,
      createdAt: Date.now(),
      updatedAt: Date.now(),
      timeline: [],
    };
    saveDraft(draft);
    clearCart();
  }, [saveDraft, clearCart]);

  const handleOpenDraftModal = useCallback(() => {
    setShowDraftModal(true);
  }, [setShowDraftModal]);

  const handleRestoreDraft = useCallback((id: string) => {
    const items = restoreDraft(id);
    setCart(items);
    setCurrentOrderKey(null);
    setShowDraftModal(false);
  }, [restoreDraft, setCart, setCurrentOrderKey, setShowDraftModal]);

  const handleDeleteDraft = useCallback((id: string) => {
    deleteDraft(id);
  }, [deleteDraft]);

  return useMemo(() => ({
    handleSaveDraft,
    handleOpenDraftModal,
    handleRestoreDraft,
    handleDeleteDraft
  }), [
    handleSaveDraft,
    handleOpenDraftModal,
    handleRestoreDraft,
    handleDeleteDraft
  ]);
}
