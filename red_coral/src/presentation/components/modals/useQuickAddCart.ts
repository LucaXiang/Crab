import { useState, useMemo, useEffect, useCallback } from 'react';
import { Currency } from '@/utils/currency';
import { logger } from '@/utils/logger';
import { CartItem as CartItemType, Product, ItemOption, Attribute, AttributeOption, ProductSpec, ProductAttribute } from '@/core/domain/types';
import { calculateOptionsModifier, generateCartKey } from '@/utils/pricing';
import { useCategories, useCategoryStore } from '@/features/category';
import { useProducts, useProductStore, useProductsLoading, ProductWithPrice } from '@/features/product';

interface SelectedProductForOptions {
  product: Product;
  basePrice: number;
  attributes: Attribute[];
  options: Map<number, AttributeOption[]>;
  bindings: ProductAttribute[];
  specifications?: ProductSpec[];
  hasMultiSpec?: boolean;
}

interface UseQuickAddCartProps {
  onClose: () => void;
  onConfirm: (items: CartItemType[]) => void;
}

export function useQuickAddCart({ onClose, onConfirm }: UseQuickAddCartProps) {
  // Use new resources stores
  const categories = useCategories();
  const products = useProducts();
  const isLoading = useProductsLoading();

  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [tempItems, setTempItems] = useState<CartItemType[]>([]);
  const [editingItem, setEditingItem] = useState<CartItemType | null>(null);

  // Product Options Modal State
  const [optionsModalOpen, setOptionsModalOpen] = useState(false);
  const [selectedProductForOptions, setSelectedProductForOptions] = useState<SelectedProductForOptions | null>(null);

  // Ensure data is loaded
  useEffect(() => {
    useCategoryStore.getState().fetchAll();
    useProductStore.getState().fetchAll();
  }, []);

  // Helper to get default spec from product
  const getDefaultSpec = (p: Product) => p.specs?.find(s => s.is_default) ?? p.specs?.[0];

  // Helper to map product with price from default spec
  const mapProductWithSpec = (p: Product): ProductWithPrice => {
    const defaultSpec = getDefaultSpec(p);
    return {
      ...p,
      price: defaultSpec?.price ?? 0,
    };
  };

  // Filter products (same logic as POSScreen)
  const filteredProducts = useMemo(() => {
    if (selectedCategory === 'all') {
      return [...products]
        .filter((p) => p.is_active)
        .sort((a, b) => {
          const aId = a.external_id ?? Number.MAX_SAFE_INTEGER;
          const bId = b.external_id ?? Number.MAX_SAFE_INTEGER;
          return aId - bId;
        })
        .map(mapProductWithSpec);
    }

    const category = categories.find((c) => c.name === selectedCategory);
    if (!category) {
      return [];
    }

    if (category.is_virtual) {
      const tagIds = category.tag_ids || [];
      if (tagIds.length === 0) {
        return [];
      }
      return products
        .filter((p) => {
          if (!p.is_active) return false;
          const productTagIds = (p.tags || []).map((t) => t.id);
          if (category.match_mode === 'all') {
            return tagIds.every((tagId) => productTagIds.includes(tagId));
          } else {
            return tagIds.some((tagId) => productTagIds.includes(tagId));
          }
        })
        .map(mapProductWithSpec);
    }

    return products
      .filter((p) => p.is_active && p.category_id === category.id)
      .map(mapProductWithSpec);
  }, [products, categories, selectedCategory]);

  // Handle product add from ProductGrid
  const handleAddProduct = useCallback(async (product: Product, _startRect?: DOMRect, skipQuickAdd: boolean = false) => {
    const productFull = useProductStore.getState().getById(product.id);
    if (!productFull) {
      logger.error('Product not found in store', undefined, { productId: product.id });
      return;
    }

    {
      const attrBindings = productFull.attributes || [];
      const attributeList: Attribute[] = attrBindings.map(b => b.attribute);
      const optionsMap = new Map<number, AttributeOption[]>();
      attributeList.forEach(attr => {
        if (attr.options && attr.options.length > 0) {
          optionsMap.set(attr.id, attr.options);
        }
      });
      const allBindings: ProductAttribute[] = attrBindings.map(binding => ({
        id: binding.id,
        owner_id: binding.is_inherited ? productFull.category_id : product.id,
        attribute_id: binding.attribute.id,
        is_required: binding.is_required,
        display_order: binding.display_order,
        default_option_ids: binding.default_option_ids,
        attribute: binding.attribute,
      }));

      const hasMultiSpec = product.specs.length > 1;
      const specifications: ProductSpec[] = product.specs || [];
      const defaultSpec = specifications.find((s) => s.is_default) || specifications[0];
      const basePrice = defaultSpec?.price ?? 0;

      // CASE 1: Force Detail View (Image Click)
      if (skipQuickAdd) {
        setSelectedProductForOptions({
          product,
          basePrice,
          attributes: attributeList,
          options: optionsMap,
          bindings: allBindings,
          specifications,
          hasMultiSpec,
        });
        setOptionsModalOpen(true);
        return;
      }

      // CASE 2: Has Multi-Spec or Attributes -> Open Modal
      if (hasMultiSpec) {
        const selectedDefaultSpec = specifications.find((s) => s.is_default === true);
        if (!selectedDefaultSpec) {
          setSelectedProductForOptions({
            product,
            basePrice,
            attributes: attributeList,
            options: optionsMap,
            bindings: allBindings,
            specifications,
            hasMultiSpec,
          });
          setOptionsModalOpen(true);
          return;
        }
      }

      if (hasMultiSpec || allBindings.length > 0) {
        setSelectedProductForOptions({
          product,
          basePrice,
          attributes: attributeList,
          options: optionsMap,
          bindings: allBindings,
          specifications,
          hasMultiSpec,
        });
        setOptionsModalOpen(true);
        return;
      }
    }

    // CASE 3: No attributes - add directly
    const defaultSpec = getDefaultSpec(product);
    const defaultSpecIdx = product.specs?.findIndex(s => s === defaultSpec) ?? 0;
    const price = defaultSpec?.price ?? 0;
    const cartKey = generateCartKey(product.id, price, undefined, undefined, defaultSpecIdx);

    const newItem: CartItemType = {
      id: product.id,
      name: product.name,
      price,
      original_price: price,
      quantity: 1,
      unpaid_quantity: 1,
      instance_id: cartKey,
      selected_options: undefined,
      selected_specification: defaultSpec ? {
        id: defaultSpecIdx,
        name: defaultSpec.name,
        price: defaultSpec.price,
        is_multi_spec: (product.specs?.length ?? 0) > 1,
      } : undefined,
      rule_discount_amount: 0,
      rule_surcharge_amount: 0,
      applied_rules: [],
      applied_mg_rules: [],
      mg_discount_amount: 0,
      unit_price: price,
      line_total: price,
      tax: 0,
      tax_rate: 0,
    };

    setTempItems(prev => {
      const existingIdx = prev.findIndex(item => item.instance_id === cartKey);
      if (existingIdx !== -1) {
        const newItems = [...prev];
        const existing = newItems[existingIdx];
        const newQty = existing.quantity + 1;
        newItems[existingIdx] = {
          ...existing,
          quantity: newQty,
          unpaid_quantity: existing.unpaid_quantity + 1,
          line_total: Currency.round2(Currency.mul(existing.unit_price, newQty)).toNumber(),
        };
        return newItems;
      }
      return [...prev, newItem];
    });
  }, []);

  // Handle options confirmed from ProductOptionsModal
  const handleOptionsConfirmed = useCallback(
    (
      selectedOptions: ItemOption[],
      quantity: number,
      discount: number,
      authorizer?: { id: number; name: string },
      selectedSpecification?: { id: number; name: string; receiptName?: string; price?: number }
    ) => {
      if (!selectedProductForOptions) return;

      const { product } = selectedProductForOptions;
      const defaultSpec = getDefaultSpec(product);
      const defaultSpecIdx = product.specs?.findIndex(s => s === defaultSpec) ?? 0;

      const specPrice = selectedSpecification?.price ?? defaultSpec?.price ?? 0;
      const opts = selectedOptions && selectedOptions.length > 0 ? selectedOptions : undefined;
      const optsMod = calculateOptionsModifier(opts);
      const unitPriceBeforeDiscount = specPrice + optsMod;
      const discountMul = discount > 0 ? (1 - discount / 100) : 1;
      const unitPrice = Currency.round2(Currency.mul(unitPriceBeforeDiscount, discountMul)).toNumber();
      const lineTotal = Currency.round2(Currency.mul(unitPrice, quantity)).toNumber();

      const specId = selectedSpecification?.id ?? defaultSpecIdx;
      const cartKey = generateCartKey(product.id, specPrice, discount > 0 ? discount : undefined, opts, specId);

      const newItem: CartItemType = {
        id: product.id,
        name: product.name,
        price: specPrice,
        original_price: specPrice,
        quantity,
        unpaid_quantity: quantity,
        instance_id: cartKey,
        selected_options: opts,
        selected_specification: selectedSpecification ? {
          id: selectedSpecification.id,
          name: selectedSpecification.name,
          receipt_name: selectedSpecification.receiptName,
          price: selectedSpecification.price,
          is_multi_spec: (product.specs?.length ?? 0) > 1,
        } : defaultSpec ? {
          id: defaultSpecIdx,
          name: defaultSpec.name,
          price: defaultSpec.price,
          is_multi_spec: (product.specs?.length ?? 0) > 1,
        } : undefined,
        rule_discount_amount: 0,
        rule_surcharge_amount: 0,
        applied_rules: [],
        applied_mg_rules: [],
        mg_discount_amount: 0,
        unit_price: unitPrice,
        line_total: lineTotal,
        tax: 0,
        tax_rate: 0,
        manual_discount_percent: discount > 0 ? discount : undefined,
        authorizer_id: authorizer?.id,
        authorizer_name: authorizer?.name,
      };

      setTempItems(prev => {
        const existingIdx = prev.findIndex(item => item.instance_id === cartKey);
        if (existingIdx !== -1) {
          const newItems = [...prev];
          const existing = newItems[existingIdx];
          const newQty = existing.quantity + quantity;
          newItems[existingIdx] = {
            ...existing,
            quantity: newQty,
            unpaid_quantity: existing.unpaid_quantity + quantity,
            line_total: Currency.round2(Currency.mul(existing.unit_price, newQty)).toNumber(),
          };
          return newItems;
        }
        return [...prev, newItem];
      });
      setOptionsModalOpen(false);
      setSelectedProductForOptions(null);
    },
    [selectedProductForOptions]
  );

  // Handle quantity change (used by CartItem)
  const handleQuantityChange = useCallback((instanceId: string, delta: number) => {
    setTempItems(prev => {
      const idx = prev.findIndex(item => item.instance_id === instanceId);
      if (idx === -1) return prev;

      const item = prev[idx];
      const newQty = item.quantity + delta;

      if (newQty <= 0) {
        return prev.filter(i => i.instance_id !== instanceId);
      }

      const newItems = [...prev];
      newItems[idx] = {
        ...item,
        quantity: newQty,
        unpaid_quantity: newQty,
        line_total: Currency.round2(Currency.mul(item.unit_price, newQty)).toNumber(),
      };
      return newItems;
    });
  }, []);

  // Handle item click - open edit modal
  const handleItemClick = useCallback((item: CartItemType) => {
    setEditingItem(item);
  }, []);

  // Handle item update from edit modal
  const handleUpdateItem = useCallback((instanceId: string, updates: Partial<CartItemType>) => {
    setTempItems(prev => prev.map(item =>
      item.instance_id === instanceId ? { ...item, ...updates } : item
    ));
  }, []);

  // Handle item remove from edit modal
  const handleRemoveItem = useCallback((instanceId: string) => {
    setTempItems(prev => prev.filter(item => item.instance_id !== instanceId));
  }, []);

  const totalAmount = tempItems.reduce(
    (sum, item) => Currency.add(sum, item.line_total).toNumber(),
    0
  );

  const handleConfirm = () => {
    if (tempItems.length > 0) {
      onConfirm(tempItems);
    }
    onClose();
  };

  const closeOptionsModal = useCallback(() => {
    setOptionsModalOpen(false);
    setSelectedProductForOptions(null);
  }, []);

  const closeEditingItem = useCallback(() => {
    setEditingItem(null);
  }, []);

  return {
    // Data
    categories,
    isLoading,
    filteredProducts,

    // Cart state
    tempItems,
    editingItem,
    optionsModalOpen,
    selectedProductForOptions,
    totalAmount,

    // Category
    selectedCategory,
    setSelectedCategory,

    // Handlers
    handleAddProduct,
    handleOptionsConfirmed,
    handleQuantityChange,
    handleItemClick,
    handleUpdateItem,
    handleRemoveItem,
    handleConfirm,
    closeOptionsModal,
    closeEditingItem,
  };
}
