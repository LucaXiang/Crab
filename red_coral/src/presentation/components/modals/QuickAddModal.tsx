import React, { useState, useMemo, useEffect, useCallback } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { X, ShoppingBag } from 'lucide-react';
import { formatCurrency, Currency } from '@/utils/currency';
import { CartItem as CartItemType, Product, ItemOption, Attribute, AttributeOption, EmbeddedSpec, ProductAttribute } from '@/core/domain/types';
import { v4 as uuidv4 } from 'uuid';
import { useCategories, useCategoryStore } from '@/features/category';
import { useProducts, useProductStore, useProductsLoading, ProductWithPrice } from '@/features/product';
import { ProductGrid } from '@/screens/POS/components/ProductGrid';
import { CategoryNav } from '@/presentation/components/CategoryNav';
import { CartList } from '@/presentation/components/cart/CartList';
import { CartItemDetailModal } from '@/presentation/components/modals/CartItemDetailModal';
import { ProductOptionsModal } from '@/presentation/components/modals/ProductOptionsModal';


interface QuickAddModalProps {
  onClose: () => void;
  onConfirm: (items: CartItemType[]) => void;
}

// Helper to check if two items are identical for merging
const areItemsEqual = (item1: CartItemType, item2: Partial<CartItemType> & { id: string }) => {
  if (item1.id !== item2.id) return false;
  
  // Check price (safeguard for open price or other variations)
  if (item1.price !== item2.price) return false;

  // Check specs
  const spec1 = item1.selected_specification?.id;
  const spec2 = item2.selected_specification?.id;
  if (spec1 !== spec2) return false;
  
  // Check manual discount (normalize null/undefined)
  const discount1 = item1.manual_discount_percent ?? 0;
  const discount2 = item2.manual_discount_percent ?? 0;
  if (discount1 !== discount2) return false;

  // Check options
  const opts1 = item1.selected_options || [];
  const opts2 = item2.selected_options || [];
  
  if (opts1.length !== opts2.length) return false;
  
  const sorted1 = [...opts1].sort((a, b) => a.attribute_id.localeCompare(b.attribute_id));
  const sorted2 = [...opts2].sort((a, b) => a.attribute_id.localeCompare(b.attribute_id));
  
  return sorted1.every((opt1, idx) => {
    const opt2 = sorted2[idx];
    return opt1.attribute_id === opt2.attribute_id && opt1.option_idx === opt2.option_idx;
  });
};

export const QuickAddModal: React.FC<QuickAddModalProps> = ({ onClose, onConfirm }) => {
  const { t } = useI18n();

  // Use new resources stores
  const categories = useCategories();
  const products = useProducts();
  const isLoading = useProductsLoading();

  const [selectedCategory, setSelectedCategory] = useState<string>('all');
  const [tempItems, setTempItems] = useState<CartItemType[]>([]);
  const [editingItem, setEditingItem] = useState<CartItemType | null>(null);

  // Product Options Modal State
  const [optionsModalOpen, setOptionsModalOpen] = useState(false);
  const [selectedProductForOptions, setSelectedProductForOptions] = useState<{
    product: Product;
    basePrice: number;
    attributes: Attribute[];
    options: Map<string, AttributeOption[]>;
    bindings: ProductAttribute[];
    specifications?: EmbeddedSpec[];
    hasMultiSpec?: boolean;
  } | null>(null);

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
    // Category filter (same logic as POSScreen)
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
      // Virtual category: filter by tags
      const tagIds = category.tag_ids || [];
      if (tagIds.length === 0) {
        return [];
      }
      return products
        .filter((p) => {
          if (!p.is_active) return false;
          // Extract tag IDs from Tag[] objects
          const productTagIds = (p.tags || []).map((t) => t.id);
          if (category.match_mode === 'all') {
            return tagIds.every((tagId) => productTagIds.includes(tagId));
          } else {
            return tagIds.some((tagId) => productTagIds.includes(tagId));
          }
        })
        .map(mapProductWithSpec);
    }

    // Regular category
    return products
      .filter((p) => p.is_active && p.category === category.id)
      .map(mapProductWithSpec);
  }, [products, categories, selectedCategory]);

  // Handle product add from ProductGrid (same logic as POSScreen)
  const handleAddProduct = useCallback(async (product: Product, _startRect?: DOMRect, skipQuickAdd: boolean = false) => {
    // Get full product data from store (ProductFull includes attributes)
    const productFull = useProductStore.getState().getById(String(product.id));
    if (!productFull) {
      console.error('Product not found in store:', product.id);
      return;
    }

    {
      // ProductFull.attributes 已包含产品直接绑定 + 分类继承属性
      const attrBindings = productFull.attributes || [];
      const attributeList: Attribute[] = attrBindings.map(b => b.attribute);
      const optionsMap = new Map<string, AttributeOption[]>();
      attributeList.forEach(attr => {
        if (attr.options && attr.options.length > 0) {
          optionsMap.set(String(attr.id), attr.options);
        }
      });
      const allBindings: ProductAttribute[] = attrBindings.map(binding => ({
        id: binding.id ?? null,
        in: binding.is_inherited ? productFull.category : String(product.id),
        out: String(binding.attribute.id),
        is_required: binding.is_required,
        display_order: binding.display_order,
        default_option_indices: binding.default_option_indices,
        attribute: binding.attribute,
      }));

      // Specs
      const hasMultiSpec = product.specs.length > 1;
      const specifications: EmbeddedSpec[] = product.specs || [];
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

    const newItem: CartItemType = {
      id: String(product.id),
      name: product.name,
      price,
      quantity: 1,
      unpaid_quantity: 1,
      instance_id: uuidv4(),
      selected_options: [],
      selected_specification: defaultSpec ? {
        id: String(defaultSpecIdx),
        name: defaultSpec.name,
        price: defaultSpec.price,
        is_multi_spec: (product.specs?.length ?? 0) > 1,
      } : undefined,
    };

    setTempItems(prev => {
      const existingIdx = prev.findIndex(item => areItemsEqual(item, newItem));
      if (existingIdx !== -1) {
        const newItems = [...prev];
        const existing = newItems[existingIdx];
        newItems[existingIdx] = {
          ...existing,
          quantity: existing.quantity + 1,
          unpaid_quantity: existing.unpaid_quantity + 1,
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
      authorizer?: { id: string; name: string },
      selectedSpecification?: { id: string; name: string; receiptName?: string; price?: number }
    ) => {
      if (!selectedProductForOptions) return;

      const { product } = selectedProductForOptions;
      const defaultSpec = getDefaultSpec(product);
      const defaultSpecIdx = product.specs?.findIndex(s => s === defaultSpec) ?? 0;

      // Calculate price from spec or base
      const specPrice = selectedSpecification?.price ?? defaultSpec?.price ?? 0;
      const optionsModifier = selectedOptions.reduce((sum, opt) => sum + (opt.price_modifier ?? 0), 0);
      const finalPrice = specPrice + optionsModifier;

      const newItem: CartItemType = {
        id: String(product.id),
        name: product.name,
        price: finalPrice,
        original_price: specPrice,
        quantity,
        unpaid_quantity: quantity,
        instance_id: uuidv4(),
        selected_options: selectedOptions,
        selected_specification: selectedSpecification ? {
          id: selectedSpecification.id,
          name: selectedSpecification.name,
          receipt_name: selectedSpecification.receiptName,
          price: selectedSpecification.price,
          is_multi_spec: (product.specs?.length ?? 0) > 1,
        } : defaultSpec ? {
          id: String(defaultSpecIdx),
          name: defaultSpec.name,
          price: defaultSpec.price,
          is_multi_spec: (product.specs?.length ?? 0) > 1,
        } : undefined,
        manual_discount_percent: discount > 0 ? discount : undefined,
        authorizer_id: authorizer?.id,
        authorizer_name: authorizer?.name,
      };

      setTempItems(prev => {
        const existingIdx = prev.findIndex(item => areItemsEqual(item, newItem));
        if (existingIdx !== -1) {
          const newItems = [...prev];
          const existing = newItems[existingIdx];
          newItems[existingIdx] = {
            ...existing,
            quantity: existing.quantity + quantity,
            unpaid_quantity: existing.unpaid_quantity + quantity,
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
      newItems[idx] = { ...item, quantity: newQty };
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

  const totalAmount = tempItems.reduce((sum, item) =>
    Currency.add(sum, Currency.mul(item.price, item.quantity)).toNumber(), 0);

  const handleConfirm = () => {
    if (tempItems.length > 0) {
      onConfirm(tempItems);
    }
    onClose();
  };

  return (
    <div className="fixed inset-0 z-60 bg-black/60 backdrop-blur-md flex items-center justify-center p-4 sm:p-6 animate-in fade-in duration-200">
      {/* Cart Item Edit Modal */}
      {editingItem && (
        <CartItemDetailModal
          item={editingItem}
          onClose={() => setEditingItem(null)}
          onUpdate={handleUpdateItem}
          onRemove={handleRemoveItem}
        />
      )}

      {/* Product Options Modal */}
      {selectedProductForOptions && (
        <ProductOptionsModal
          isOpen={optionsModalOpen}
          onClose={() => {
            setOptionsModalOpen(false);
            setSelectedProductForOptions(null);
          }}
          productName={selectedProductForOptions.product.name}
          basePrice={selectedProductForOptions.basePrice}
          attributes={selectedProductForOptions.attributes}
          allOptions={selectedProductForOptions.options}
          bindings={selectedProductForOptions.bindings}
          specifications={selectedProductForOptions.specifications}
          hasMultiSpec={selectedProductForOptions.hasMultiSpec}
          onConfirm={handleOptionsConfirmed}
        />
      )}

      <div
        className="bg-white rounded-2xl shadow-2xl w-full max-w-[95vw] h-[92vh] overflow-hidden flex relative animate-in zoom-in-95 duration-200"
        onClick={e => e.stopPropagation()}
      >
        {/* Left Side: Cart Sidebar (Matches POS Sidebar style) */}
        <div className="w-[27.5rem] shrink-0 flex flex-col bg-white z-30">
          {/* Header */}
          <div className="h-16 flex items-center justify-between px-6 bg-primary-500 text-white shrink-0 relative z-20">
            <div className="flex items-center gap-3">
              <h3 className="font-bold text-xl">{t('pos.quick_add.title')}</h3>
              <span className="bg-white text-primary-500 px-2.5 py-0.5 rounded-full text-sm font-bold">
                {tempItems.reduce((acc, item) => acc + item.quantity, 0)}
              </span>
            </div>
            <button
              onClick={onClose}
              className="p-2 text-white/80 hover:bg-white/10 hover:text-white rounded-xl transition-all"
            >
              <X size={24} />
            </button>
          </div>

          {/* Cart List & Footer Container */}
          <div className="flex-1 flex flex-col min-h-0 border-r border-gray-200 shadow-xl relative z-10">
            {/* Cart List */}
            <div className="flex-1 overflow-y-auto bg-white relative custom-scrollbar">
              {tempItems.length === 0 ? (
                <div className="absolute inset-0 flex flex-col items-center justify-center text-gray-300 select-none">
                  <div className="w-24 h-24 rounded-full bg-gray-50 mb-4 flex items-center justify-center">
                    <ShoppingBag size={36} className="opacity-20" />
                  </div>
                  <p className="text-gray-400 text-sm">{t('pos.quick_add.select_prompt')}</p>
                </div>
              ) : (
                <CartList
                  cart={tempItems}
                  onQuantityChange={handleQuantityChange}
                  onItemClick={handleItemClick}
                />
              )}
            </div>
  
            {/* Footer (Matches CartCheckoutBar style) */}
            <div className="bg-primary-500 text-white flex h-16 relative z-30 shadow-inner shrink-0">
              <div 
                className="w-28 flex items-center justify-center text-lg font-medium border-r border-white/20 bg-black/5 cursor-pointer hover:bg-black/10 transition-colors"
                onClick={onClose}
              >
                {t('common.action.cancel')} 
              </div>
              <div
                className={`flex-1 flex items-center justify-between px-8 text-2xl font-light transition-colors ${
                  tempItems.length === 0 ? 'cursor-default opacity-50' : 'cursor-pointer hover:bg-white/10'
                }`}
                onClick={tempItems.length > 0 ? handleConfirm : undefined}
              >
                <span className="text-lg font-medium opacity-90">{t('pos.quick_add.confirm')}</span>
                <span className="text-3xl font-semibold">{formatCurrency(totalAmount)}</span>
              </div>
            </div>
          </div>
        </div>

        {/* Right Side: Category Nav + Products */}
        <div className="flex-1 flex flex-col min-w-0 bg-gray-100">
          {/* Category Nav */}
          <div className="shrink-0 bg-primary-500 shadow-sm z-20">
            <CategoryNav
              selected={selectedCategory}
              onSelect={setSelectedCategory}
              categories={categories}
            />
          </div>

          {/* Product Grid */}
          <ProductGrid
            products={filteredProducts}
            isLoading={isLoading}
            onAdd={handleAddProduct}
            className="grid-cols-2 lg:grid-cols-3 2xl:grid-cols-4 gap-4 p-4"
          />
        </div>
      </div>
    </div>
  );
};
