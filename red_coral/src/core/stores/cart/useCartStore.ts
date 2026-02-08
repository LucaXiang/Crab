import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { CartItem, ItemOption } from '@/core/domain/types';
import { ProductWithPrice } from '@/features/product';
import { Currency } from '@/utils/currency';
import { calculateOptionsModifier } from '@/utils/pricing';

/**
 * Compare two selectedOptions arrays for equality
 */
function areOptionsEqual(
  options1: ItemOption[] | null | undefined,
  options2: ItemOption[] | null | undefined
): boolean {
  // Normalize empty arrays to undefined for comparison
  const o1 = options1 && options1.length > 0 ? options1 : undefined;
  const o2 = options2 && options2.length > 0 ? options2 : undefined;

  if (!o1 && !o2) return true;
  if (!o1 || !o2) return false;
  if (o1.length !== o2.length) return false;

  // Sort both arrays by attribute_id + option_idx for comparison
  const sorted1 = [...o1].sort((a, b) =>
    `${a.attribute_id}-${a.option_idx}`.localeCompare(`${b.attribute_id}-${b.option_idx}`)
  );
  const sorted2 = [...o2].sort((a, b) =>
    `${a.attribute_id}-${a.option_idx}`.localeCompare(`${b.attribute_id}-${b.option_idx}`)
  );

  return sorted1.every((opt1, index) => {
    const opt2 = sorted2[index];
    return (
      opt1.attribute_id === opt2.attribute_id &&
      opt1.option_idx === opt2.option_idx &&
      opt1.price_modifier === opt2.price_modifier &&
      (opt1.quantity ?? 1) === (opt2.quantity ?? 1)
    );
  });
}

/**
 * Compare two specifications for equality
 */
function areSpecificationsEqual(
  spec1: { id: number; name: string; receiptName?: string; price?: number } | null | undefined,
  spec2: { id: number; name: string; receiptName?: string; price?: number } | null | undefined
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
  // Computed
  totalAmount: number;
  itemCount: number;

  // Actions
  addToCart: (product: ProductWithPrice, selectedOptions?: ItemOption[], quantity?: number, discount?: number, authorizer?: { id: number; name: string }, selectedSpecification?: { id: number; name: string; receiptName?: string; price?: number }) => void;
  removeFromCart: (instanceId: string) => void;
  updateCartItem: (instanceId: string, updates: Partial<CartItem>) => void;
  incrementItemQuantity: (instanceId: string, delta: number) => void;
  setItemQuantity: (instanceId: string, quantity: number) => void;
  clearCart: () => void;
  setCart: (items: CartItem[]) => void;
  calculateTotal: () => void;
}

export const useCartStore = create<CartStore>((set, get) => ({
  // Initial State
  cart: [],
  totalAmount: 0,
  itemCount: 0,

  // Actions
  addToCart: (product: ProductWithPrice, selectedOptions?: ItemOption[], quantity: number = 1, discount: number = 0, authorizer?: { id: number; name: string }, selectedSpecification?: { id: number; name: string; receiptName?: string; price?: number; is_multi_spec?: boolean }) => {
    set((state) => {
      // Get default spec from product if no selectedSpecification provided
      let effectiveSpec = selectedSpecification;
      if (!effectiveSpec && product.specs && product.specs.length > 0) {
        const defaultSpec = product.specs.find(s => s.is_default) ?? product.specs[0];
        const specIdx = product.specs.indexOf(defaultSpec);
        effectiveSpec = {
          id: specIdx,
          name: defaultSpec.name,
          price: defaultSpec.price,
          is_multi_spec: (product.specs?.length ?? 0) > 1,
        };
      }

      // Only merge if:
      // 1. Same product ID
      // 2. Same discount (or both zero)
      // 3. Same selectedOptions (or both undefined)
      // 4. Same authorizer (if any)
      // 5. Same specification (if any)
      const existingIndex = state.cart.findIndex(item =>
        item.id === product.id &&
        (item.manual_discount_percent || 0) === discount &&
        areOptionsEqual(item.selected_options, selectedOptions) &&
        areSpecificationsEqual(item.selected_specification, effectiveSpec) &&
        item.authorizer_id === authorizer?.id
      );

      if (existingIndex >= 0) {
        const newCart = [...state.cart];
        const item = newCart[existingIndex];
        newCart[existingIndex] = { ...item, quantity: item.quantity + quantity };
        return { cart: newCart };
      }

      // Use specification price if available, otherwise use product price (from root spec or passed in)
      const productPrice = product.price ?? 0;
      const effectivePrice = effectiveSpec?.price !== undefined ? effectiveSpec.price : productPrice;

      return {
        cart: [...state.cart, {
          id: product.id,
          name: product.name,
          quantity: quantity,
          unpaid_quantity: quantity,  // All items are unpaid when first added to cart
          price: effectivePrice,
          original_price: effectivePrice,
          manual_discount_percent: discount,
          selected_options: selectedOptions && selectedOptions.length > 0 ? selectedOptions : undefined,
          selected_specification: effectiveSpec,
          instance_id: `item-${Date.now()}-${Math.random().toString(36).slice(2, 11)}`,
          authorizer_id: authorizer?.id,
          authorizer_name: authorizer?.name,
        }]
      };
    });
    get().calculateTotal();
  },

  removeFromCart: (instanceId: string) => {
    set((state) => ({
      cart: state.cart.filter(item => item.instance_id !== instanceId)
    }));
    get().calculateTotal();
  },

  incrementItemQuantity: (instanceId: string, delta: number) => {
    set((state) => ({
      cart: state.cart.map((item) =>
        item.instance_id === instanceId
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
        item.instance_id === instanceId ? { ...item, quantity: safeQty } : item
      ),
    }));
    get().calculateTotal();
  },

  updateCartItem: (instanceId: string, updates: Partial<CartItem>) => {
    set((state) => ({
      cart: state.cart.map(item =>
        item.instance_id === instanceId ? { ...item, ...updates } : item
      )
    }));
    get().calculateTotal();
  },

  clearCart: () => {
    set({
      cart: [],
      totalAmount: 0,
      itemCount: 0
    });
  },

  setCart: (items: CartItem[]) => {
    set({ cart: items });
    get().calculateTotal();
  },

  calculateTotal: () => {
    const { cart } = get();
    // Calculate total considering manual discounts (same logic as CartItem.tsx)
    const total = cart.reduce((sum, item) => {
      if (item._removed) return sum;
      
      // Use server-computed line_total if available
      if (item.line_total !== undefined && item.line_total !== null) {
        return Currency.add(sum, item.line_total).toNumber();
      }
      
      // Fallback to local calculation
      const discountPercent = item.manual_discount_percent || 0;
      // Options modifier considers quantity for each option
      const optionsModifier = calculateOptionsModifier(item.selected_options);
      const basePrice = item.original_price ?? item.price;
      const baseUnitPrice = basePrice + optionsModifier;
      
      let finalUnitPrice: number;
      if (discountPercent > 0) {
        const discountFactor = Currency.sub(1, Currency.div(discountPercent, 100));
        finalUnitPrice = Currency.round2(Currency.mul(baseUnitPrice, discountFactor)).toNumber();
      } else {
        finalUnitPrice = baseUnitPrice;
      }
      
      const lineTotal = Currency.round2(Currency.mul(finalUnitPrice, item.quantity)).toNumber();
      return Currency.add(sum, lineTotal).toNumber();
    }, 0);
    const count = cart.reduce((sum, item) => sum + item.quantity, 0);
    set({ totalAmount: Currency.round2(total).toNumber(), itemCount: count });
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
  }))
);
