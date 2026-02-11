import { useCallback, useMemo } from 'react';
import { CartItem, DraftOrder, PaymentRecord } from '@/core/domain/types';
import { useCartStore } from '@/core/stores/cart/useCartStore';

interface UseDraftHandlersParams {
  saveDraft: (draft: DraftOrder) => void;
  restoreDraft: (id: string) => CartItem[];
  deleteDraft: (id: string) => void;
  clearCart: () => void;
  setCart: (items: CartItem[]) => void;
  setShowDraftModal: (v: boolean) => void;
  setCurrentOrderKey: (key: string | number | null) => void;
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
    const draftId = `draft-${Date.now()}`;
    const now = Date.now();
    const draft: DraftOrder = {
      // Required fields from OrderSnapshot
      order_id: draftId,
      table_id: null,
      table_name: null,
      zone_id: null,
      zone_name: null,
      guest_count: 0,
      is_retail: true,
      status: 'ACTIVE',
      items: cart,
      payments: [] as PaymentRecord[],
      original_total: totalAmount,
      subtotal: totalAmount,
      total_discount: 0,
      total_surcharge: 0,
      tax: 0,
      discount: 0,
      comp_total_amount: 0,
      order_manual_discount_amount: 0,
      order_manual_surcharge_amount: 0,
      total: totalAmount,
      paid_amount: 0,
      remaining_amount: totalAmount,
      paid_item_quantities: {},
      order_rule_discount_amount: 0,
      order_rule_surcharge_amount: 0,
      order_applied_rules: [],
      mg_discount_amount: 0,
      receipt_number: null,
      is_pre_payment: false,
      start_time: now,
      end_time: null,
      created_at: now,
      updated_at: now,
      last_sequence: 0,
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
