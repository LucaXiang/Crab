import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';
import { CartItem, ItemOption } from '@/core/domain/types';
import { ProductWithPrice } from '@/features/product';
import { Currency } from '@/utils/currency';
import { calculateOptionsModifier, generateCartKey } from '@/utils/pricing';

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
        effectiveSpec = {
          id: defaultSpec.id!,
          name: defaultSpec.name,
          price: defaultSpec.price,
          is_multi_spec: (product.specs?.length ?? 0) > 1,
        };
      }

      // Use specification price if available, otherwise use product price
      const productPrice = product.price ?? 0;
      const effectivePrice = effectiveSpec?.price !== undefined ? effectiveSpec.price : productPrice;

      // Content-addressed key: same product+price+discount+options+spec → same key → merge
      const cartKey = generateCartKey(product.id, effectivePrice, discount, selectedOptions, effectiveSpec?.id);
      const existingIndex = state.cart.findIndex(item => item.instance_id === cartKey);

      if (existingIndex >= 0) {
        const newCart = [...state.cart];
        const item = newCart[existingIndex];
        const newQty = item.quantity + quantity;
        newCart[existingIndex] = {
          ...item,
          quantity: newQty,
          line_total: Currency.round2(Currency.mul(item.unit_price, newQty)).toNumber(),
        };
        return { cart: newCart };
      }

      return {
        cart: [...state.cart, {
          id: product.id,
          name: product.name,
          quantity: quantity,
          unpaid_quantity: quantity,
          price: effectivePrice,
          original_price: effectivePrice,
          rule_discount_amount: 0,
          rule_surcharge_amount: 0,
          applied_rules: [],
          applied_mg_rules: [],
          mg_discount_amount: 0,
          unit_price: effectivePrice,
          line_total: Currency.round2(Currency.mul(effectivePrice, quantity)).toNumber(),
          tax: 0,
          tax_rate: 0,
          manual_discount_percent: discount,
          selected_options: selectedOptions && selectedOptions.length > 0 ? selectedOptions : undefined,
          selected_specification: effectiveSpec,
          instance_id: cartKey,
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
      cart: state.cart.map((item) => {
        if (item.instance_id !== instanceId) return item;
        const newQty = Math.max(1, item.quantity + delta);
        return {
          ...item,
          quantity: newQty,
          line_total: Currency.round2(Currency.mul(item.unit_price, newQty)).toNumber(),
        };
      }),
    }));
    get().calculateTotal();
  },

  // Set absolute quantity for a cart item (high-level API)
  setItemQuantity: (instanceId: string, quantity: number) => {
    const safeQty = Math.max(1, Math.floor(quantity));
    set((state) => ({
      cart: state.cart.map((item) => {
        if (item.instance_id !== instanceId) return item;
        return {
          ...item,
          quantity: safeQty,
          line_total: Currency.round2(Currency.mul(item.unit_price, safeQty)).toNumber(),
        };
      }),
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
      
      // Use server-computed line_total if available (non-zero means server has computed it)
      if (item.line_total !== 0) {
        return Currency.add(sum, item.line_total).toNumber();
      }

      // Fallback to local calculation (for draft items not yet sent to server)
      const discountPercent = item.manual_discount_percent || 0;
      // Options modifier considers quantity for each option
      const optionsModifier = calculateOptionsModifier(item.selected_options);
      const basePrice = item.original_price || item.price;
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
