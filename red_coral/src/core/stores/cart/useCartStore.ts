import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { CartItem, Product, ItemAttributeSelection } from '@/core/domain/types';
import { calculateOrderTotal } from '@/utils/pricing';

/**
 * Compare two selectedOptions arrays for equality
 */
function areOptionsEqual(
  options1: ItemAttributeSelection[] | undefined,
  options2: ItemAttributeSelection[] | undefined
): boolean {
  // Normalize empty arrays to undefined for comparison
  const o1 = options1 && options1.length > 0 ? options1 : undefined;
  const o2 = options2 && options2.length > 0 ? options2 : undefined;

  if (!o1 && !o2) return true;
  if (!o1 || !o2) return false;
  if (o1.length !== o2.length) return false;

  // Sort both arrays by attributeId + optionId for comparison
  const sorted1 = [...o1].sort((a, b) =>
    `${a.attributeId}-${a.optionId}`.localeCompare(`${b.attributeId}-${b.optionId}`)
  );
  const sorted2 = [...o2].sort((a, b) =>
    `${a.attributeId}-${a.optionId}`.localeCompare(`${b.attributeId}-${b.optionId}`)
  );

  return sorted1.every((opt1, index) => {
    const opt2 = sorted2[index];
    return (
      opt1.attributeId === opt2.attributeId &&
      opt1.optionId === opt2.optionId &&
      opt1.priceModifier === opt2.priceModifier
    );
  });
}

/**
 * Compare two specifications for equality
 */
function areSpecificationsEqual(
  spec1: { id: string; name: string; receiptName?: string; price?: number } | undefined,
  spec2: { id: string; name: string; receiptName?: string; price?: number } | undefined
): boolean {
  if (!spec1 && !spec2) return true;
  if (!spec1 || !spec2) return false;
  return (
    spec1.id === spec2.id &&
    spec1.price === spec2.price &&
    spec1.name === spec2.name &&
    spec1.receiptName === spec2.receiptName
  );
}

interface CartStore {
  // State
  cart: CartItem[];
  receiptNumber?: string;

  // Computed
  totalAmount: number;
  itemCount: number;

  // Actions
  addToCart: (product: Product, selectedOptions?: ItemAttributeSelection[], quantity?: number, discount?: number, authorizer?: { id: string; username: string }, selectedSpecification?: { id: string; name: string; receiptName?: string; price?: number }) => void;
  removeFromCart: (instanceId: string) => void;
  updateCartItem: (instanceId: string, updates: Partial<CartItem>) => void;
  incrementItemQuantity: (instanceId: string, delta: number) => void;
  setItemQuantity: (instanceId: string, quantity: number) => void;
  clearCart: () => void;
  setCart: (items: CartItem[]) => void;
  setReceiptNumber: (number: string) => void;
  calculateTotal: () => void;
}

export const useCartStore = create<CartStore>((set, get) => ({
  // Initial State
  cart: [],
  receiptNumber: undefined,
  totalAmount: 0,
  itemCount: 0,

  // Actions
  addToCart: (product: Product, selectedOptions?: ItemAttributeSelection[], quantity: number = 1, discount: number = 0, authorizer?: { id: string; username: string }, selectedSpecification?: { id: string; name: string; receiptName?: string; price?: number }) => {
    set((state) => {
      // Only merge if:
      // 1. Same product ID
      // 2. Same discount (or both zero)
      // 3. Same selectedOptions (or both undefined)
      // 4. Same authorizer (if any)
      // 5. Same specification (if any)
      const existingIndex = state.cart.findIndex(item =>
        item.id === String(product.id) &&
        (item.discountPercent || 0) === discount &&
        areOptionsEqual(item.selectedOptions, selectedOptions) &&
        areSpecificationsEqual(item.selectedSpecification, selectedSpecification) &&
        item.authorizerId === authorizer?.id
      );

      if (existingIndex >= 0) {
        const newCart = [...state.cart];
        const item = newCart[existingIndex];
        newCart[existingIndex] = { ...item, quantity: item.quantity + quantity };
        return { cart: newCart };
      }

      // Use specification price if available, otherwise use product price
      const effectivePrice = selectedSpecification?.price !== undefined ? selectedSpecification.price : (product.price ?? 0);

      return {
        cart: [...state.cart, {
          id: String(product.id),
          productId: product.id,
          name: product.name,
          quantity: quantity,
          price: effectivePrice,
          originalPrice: effectivePrice,
          discountPercent: discount,
          selectedOptions: selectedOptions && selectedOptions.length > 0 ? selectedOptions : undefined,
          selectedSpecification: selectedSpecification,
          instanceId: `item-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`,
          authorizerId: authorizer?.id,
          authorizerName: authorizer?.username
        }]
      };
    });
    get().calculateTotal();
  },

  removeFromCart: (instanceId: string) => {
    set((state) => ({
      cart: state.cart.filter(item => item.instanceId !== instanceId)
    }));
    get().calculateTotal();
  },

  incrementItemQuantity: (instanceId: string, delta: number) => {
    set((state) => ({
      cart: state.cart.map((item) =>
        item.instanceId === instanceId
          ? { ...item, quantity: Math.max(1, item.quantity + delta) }
          : item
      ),
    }));
    get().calculateTotal();
  },

  // Set absolute quantity for a cart item (high-level API)
  setItemQuantity: (instanceId: string, quantity: number) => {
    const safeQty = Math.max(1, Math.floor(quantity));
    set((state) => ({
      cart: state.cart.map((item) =>
        item.instanceId === instanceId ? { ...item, quantity: safeQty } : item
      ),
    }));
    get().calculateTotal();
  },

  updateCartItem: (instanceId: string, updates: Partial<CartItem>) => {
    set((state) => ({
      cart: state.cart.map(item =>
        item.instanceId === instanceId ? { ...item, ...updates } : item
      )
    }));
    get().calculateTotal();
  },

  clearCart: () => {
    set({
      cart: [],
      receiptNumber: undefined,
      totalAmount: 0,
      itemCount: 0
    });
  },

  setCart: (items: CartItem[]) => {
    set({ cart: items });
    get().calculateTotal();
  },

  setReceiptNumber: (number: string) => {
    set({ receiptNumber: number });
  },

  calculateTotal: () => {
    const { cart } = get();
    const total = calculateOrderTotal(cart).toNumber();
    const count = cart.reduce((sum, item) => sum + item.quantity, 0);
    set({ totalAmount: total, itemCount: count });
  }
}));

// ============ Granular Selectors (Performance Optimization) ============
// These selectors prevent unnecessary re-renders by subscribing to specific slices

export const useCart = () => useCartStore((state) => state.cart);
export const useTotalAmount = () => useCartStore((state) => state.totalAmount);
export const useIsCartEmpty = () => useCartStore((state) => state.cart.length === 0);

// Cart Actions (stable references)
export const useCartActions = () => useCartStore(
  useShallow((state) => ({
    addToCart: state.addToCart,
    removeFromCart: state.removeFromCart,
    updateCartItem: state.updateCartItem,
    incrementItemQuantity: state.incrementItemQuantity,
    setItemQuantity: state.setItemQuantity,
    clearCart: state.clearCart,
    setCart: state.setCart,
    setReceiptNumber: state.setReceiptNumber
  }))
);
