import React, { useCallback, useEffect, useMemo, useState } from 'react';
import DefaultImage from '../../assets/reshot.svg';
import { getImageUrl } from '@/core/services/imageCache';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { voidOrder } from '@/core/stores/order/useOrderOperations';
import { useCanManageProducts } from '@/hooks/usePermission';
import { ProductModal } from '@/features/product/ProductModal';
import { useShiftStore } from '@/core/stores/shift';
import { ShiftActionModal } from '@/features/shift';

// Components
import { Sidebar } from '@/presentation/components/Sidebar';
import { CategoryNav } from '@/presentation/components/CategoryNav';
import { toast } from '@/presentation/components/Toast';
import { ProductOptionsModal } from '@/presentation/components/modals/ProductOptionsModal';

// Local Components
import {
	  ActionBar,
	  ProductGrid,
	  POSModals,
	  POSOverlays,
	  CartAnimationLayer,
	}	from './components';

// Types
import { Product, ItemOption, Attribute, AttributeOption, EmbeddedSpec, ProductAttribute } from '@/core/domain/types';

// i18n
import { useI18n } from '@/hooks/useI18n';

// Stores - New Architecture
import {
  useProducts,
  useProductsLoading,
  useCategories,
  useProductStore,
} from '@/core/stores/resources';
import { usePreloadCoreData, useHealthCheck } from '@/core/hooks';
import {
  useCart,
  useCartActions,
} from '@/core/stores/cart/useCartStore';
import {
  useHeldOrders,
  useDraftOrders,
  useCurrentOrderKey,
  useCheckoutOrder,
  useOrderActions,
} from '@/core/stores/order';
import {
  useScreen,
  useViewMode,
  useModalStates,
  useReceiptPrinter,
  useUIActions,
  useSelectedCategory,
  usePOSUIActions,
} from '@/core/stores/ui';
import {
  useSettingsStore,
  useSettingsModal,
} from '@/core/stores/settings';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';

import { ConfirmDialog } from '@/shared/components/ConfirmDialog';

// Hooks
import { useOrderHandlers } from '@/hooks/useOrderHandlers';
import { useDraftHandlers } from '@/hooks/useDraftHandlers';
import { useRetailOrderRecovery } from '@/hooks/useRetailOrderRecovery';

export const POSScreen: React.FC = () => {
  const { t } = useI18n();

  // With event sourcing, orders are hydrated from server events automatically
  // No local hydration needed

  // Permissions & Modal
  const canManageProducts = useCanManageProducts();
  const { openModal } = useSettingsModal();

  const handleLongPressProduct = useCallback((product: Product) => {
    if (canManageProducts) {
      openModal('PRODUCT', 'EDIT', product);
    } else {
      toast.error(t('auth.unauthorized.message'));
    }
  }, [canManageProducts, openModal, t]);

  // Product Store (New Architecture)
  const products = useProducts();
  const isProductLoading = useProductsLoading();
  const allCategories = useCategories();
  // Filter out categories with is_display: false
  const categories = allCategories.filter(c => c.is_display !== false);
  const selectedCategory = useSelectedCategory();
  const { setSelectedCategory } = usePOSUIActions();

  // Helper to get default spec from product
  const getDefaultSpec = (p: Product) => p.specs?.find(s => s.is_default) ?? p.specs?.[0];

  // Helper to map product with price from default spec
  const mapProductWithSpec = (p: Product) => {
    const defaultSpec = getDefaultSpec(p);
    return {
      ...p,
      price: defaultSpec?.price ?? 0,
    };
  };

  // Filter products based on selected category
  const filteredProducts = useMemo(() => {
    // "all" category: show all active products sorted by external_id
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

    // Find the selected category
    const category = categories.find((c) => c.name === selectedCategory);
    if (!category) {
      return [];
    }

    // Virtual category: filter by tags based on match_mode
    if (category.is_virtual) {
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
            // Product must have ALL tags
            return tagIds.every((tagId) => productTagIds.includes(tagId));
          } else {
            // Product must have ANY tag (default: 'any')
            return tagIds.some((tagId) => productTagIds.includes(tagId));
          }
        })
        .sort((a, b) => a.sort_order - b.sort_order)
        .map(mapProductWithSpec);
    }

    // Regular category: filter by category id
    return products
      .filter((p) => p.is_active && p.category === category.id)
      .sort((a, b) => a.sort_order - b.sort_order)
      .map(mapProductWithSpec);
  }, [products, categories, selectedCategory]);

  // Preload core resources on first mount (zones, tables, categories, products)
  const coreDataReady = usePreloadCoreData();

  // Cart Store
  const cart = useCart();
  const { addToCart: addToCartStore, clearCart, setCart } = useCartActions();

  // Order Store
  const heldOrders = useHeldOrders();
  const draftOrders = useDraftOrders();
  const currentOrderKey = useCurrentOrderKey();
  const checkoutOrder = useCheckoutOrder();
  const {
    handleTableSelect: handleTableSelectStore,
    setCheckoutOrder,
    setCurrentOrderKey,
    voidOrder,
    saveDraft,
    restoreDraft,
    deleteDraft,
  } = useOrderActions();

  // UI Store
  const screen = useScreen();
  const viewMode = useViewMode();
	const { showTableScreen, showDraftModal } = useModalStates();
	const [manageTableId, setManageTableId] = useState<string | null>(null);
	const performanceMode = useSettingsStore((state) => state.performanceMode);

  // Product Options Modal State
  // Note: Product type from backend doesn't have price - it's on ProductSpecification
  // We include a computed basePrice from root spec
  const [optionsModalOpen, setOptionsModalOpen] = useState(false);
  const [selectedProductForOptions, setSelectedProductForOptions] = useState<{
    product: Product;
    basePrice: number;  // Computed from root spec
    startRect?: DOMRect;
    attributes: Attribute[];
    options: Map<string, AttributeOption[]>;
    bindings: ProductAttribute[];
    specifications?: EmbeddedSpec[];
    hasMultiSpec?: boolean;
  } | null>(null);
  const {
    setScreen,
    setViewMode,
    setShowTableScreen,
    setShowDraftModal,
    addAnimation,
  } = useUIActions();
  const selectedPrinter = useReceiptPrinter();

  // DB Status
  const isDbOnline = useHealthCheck();


  // Custom Hooks
  const orderHandlers = useOrderHandlers({
    handleTableSelectStore,
    voidOrder,
    setCheckoutOrder,
    setCurrentOrderKey,
    setViewMode,
    setShowTableScreen,
  });

  const draftHandlers = useDraftHandlers({
    saveDraft,
    restoreDraft,
    deleteDraft,
    clearCart,
    setCart,
    setShowDraftModal,
    setCurrentOrderKey,
  });

  // 检查并恢复未完成的零售订单（断电/崩溃后恢复）
  useRetailOrderRecovery({
    setViewMode,
    setCurrentOrderKey,
  });

  const {
    handleTableSelect,
    handleManageTable,
    handleCheckoutStart,
    handleCheckoutComplete,
    handleCheckoutCancel,
  } = orderHandlers;

  const {
    handleSaveDraft,
    handleOpenDraftModal,
    handleRestoreDraft,
    handleDeleteDraft,
  } = draftHandlers;

  // Handlers
  const addToCart = useCallback(
    async (product: Product, startRect?: DOMRect, skipQuickAdd: boolean = false) => {
      // Get full product data from store (ProductFull includes attributes)
      const productFull = useProductStore.getState().getById(String(product.id));
      if (!productFull) {
        toast.error(t('pos.error.product_not_found'));
        return;
      }

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

      // Specs are now embedded in ProductFull (EmbeddedSpec[])
        const hasMultiSpec = productFull.specs.length > 1;
        const specifications: EmbeddedSpec[] = productFull.specs || [];

        // Get base price from default spec or first spec
        const defaultSpec = specifications.find((s) => s.is_default) || specifications[0];
        const basePrice = defaultSpec?.price ?? 0;

        // CASE 1: Force Detail View (e.g. Image Click)
        // If skipQuickAdd is true, we ALWAYS open the modal, regardless of whether attributes/specs exist.
        if (skipQuickAdd) {
            setSelectedProductForOptions({
                product,
                basePrice,
                startRect,
                attributes: attributeList,
                options: optionsMap,
                bindings: allBindings,
                specifications,
                hasMultiSpec,
            });
            setOptionsModalOpen(true);
            return;
        }

        // CASE 2: Has Multi-Spec or Attributes -> Check if we need modal

        let selectedDefaultSpec: EmbeddedSpec | undefined = undefined;

        if (hasMultiSpec) {
            // Check for default specification
            // is_default is boolean in the type definition
            selectedDefaultSpec = specifications.find((s) => s.is_default === true);

            // If no default specification is found, we MUST open the modal
            if (!selectedDefaultSpec) {
                setSelectedProductForOptions({
                    product,
                    basePrice,
                    startRect,
                    attributes: attributeList,
                    options: optionsMap,
                    bindings: allBindings,
                    specifications,
                    hasMultiSpec,
                });
                setOptionsModalOpen(true);
                return;
            }
            // If default spec exists, we continue to check attributes
        }

        if (hasMultiSpec || allBindings.length > 0) {
          // Quick-add: if all required attributes have defaults and spec is resolved, skip modal
          const canQuickAdd = allBindings.every(binding => {
            if (!binding.is_required) return true;
            const defaults = binding.default_option_indices
              ?? binding.attribute?.default_option_indices;
            if (!defaults || defaults.length === 0) return false;
            // 确保至少有一个默认选项存在
            const attrOpts = optionsMap.get(String(binding.attribute?.id)) || [];
            return defaults.some(idx => attrOpts[idx] !== undefined);
          });

          if (canQuickAdd && (!hasMultiSpec || selectedDefaultSpec)) {
            // Build ItemOption[] from defaults (respecting max_selections)
            const quickOptions: ItemOption[] = [];
            allBindings.forEach(binding => {
              const attr = binding.attribute;
              if (!attr) return;
              const defaults = binding.default_option_indices
                ?? attr.default_option_indices;
              if (!defaults || defaults.length === 0) return;
              const attrOpts = optionsMap.get(String(attr.id)) || [];
              let count = 0;
              defaults.forEach(idx => {
                // Enforce max_selections for multi-select
                if (attr.is_multi_select && attr.max_selections && count >= attr.max_selections) return;
                const opt = attrOpts[idx];
                if (opt) {
                  quickOptions.push({
                    attribute_id: String(attr.id),
                    attribute_name: attr.name,
                    option_idx: idx,
                    option_name: opt.name,
                    price_modifier: opt.price_modifier ?? null,
                    quantity: 1, // Default quantity for quick add
                  });
                  count++;
                }
              });
            });

            // Build spec info
            let quickSpec: { id: string; name: string; price?: number; is_multi_spec?: boolean } | undefined;
            if (selectedDefaultSpec) {
              const specIdx = specifications.indexOf(selectedDefaultSpec);
              quickSpec = {
                id: String(specIdx),
                name: selectedDefaultSpec.name,
                price: selectedDefaultSpec.price,
                is_multi_spec: hasMultiSpec,
              };
            } else if (specifications.length > 0) {
              const spec = specifications.find(s => s.is_default) ?? specifications[0];
              const specIdx = specifications.indexOf(spec);
              quickSpec = {
                id: String(specIdx),
                name: spec.name,
                price: spec.price,
                is_multi_spec: hasMultiSpec,
              };
            }

            addToCartStore(product, quickOptions, 1, 0, undefined, quickSpec);

            if (startRect && !performanceMode) {
              const id = `fly-${Date.now()}-${Math.random()}`;
              const targetX = 190;
              const targetY = window.innerHeight / 2;
              getImageUrl(product.image).then((imageForAnim) => {
                addAnimation({ id, type: 'fly', image: imageForAnim || DefaultImage, startRect, targetX, targetY });
              });
            }
            return;
          }

          // Cannot quick-add -> Open Modal for selection
          setSelectedProductForOptions({
            product,
            basePrice,
            startRect,
            attributes: attributeList,
            options: optionsMap,
            bindings: allBindings,
            specifications,
            hasMultiSpec,
          });
          setOptionsModalOpen(true);
          return;
        }

        // CASE 3: No Attributes -> Direct Add

      // No attributes: add directly to cart
      if (!skipQuickAdd) {
        addToCartStore(product);
      }

      if (startRect && !performanceMode) {
        const id = `fly-${Date.now()}-${Math.random()}`;
        const targetX = 190;
        const targetY = window.innerHeight / 2;

        // Get image URL async, use default immediately if not cached
        getImageUrl(product.image).then((imageForAnim) => {
          addAnimation({
            id,
            type: 'fly',
            image: imageForAnim || DefaultImage,
            startRect,
            targetX,
            targetY,
          });
        });
      }
    },
    [addToCartStore, addAnimation, performanceMode]
  );

  const handleOptionsConfirmed = useCallback(
    (
      selectedOptions: ItemOption[],
      quantity: number,
      discount: number,
      authorizer?: { id: string; name: string },
      selectedSpecification?: { id: string; name: string; receiptName?: string; price?: number }
    ) => {
      if (!selectedProductForOptions) return;

      const { product, startRect } = selectedProductForOptions;

      // Add to cart with selected options and specification
      addToCartStore(product, selectedOptions, quantity, discount, authorizer, selectedSpecification);

      // Play animation
      if (startRect && !performanceMode) {
        const id = `fly-${Date.now()}-${Math.random()}`;
        const targetX = 190;
        const targetY = window.innerHeight / 2;

        getImageUrl(product.image).then((imageForAnim) => {
          addAnimation({
            id,
            type: 'fly',
            image: imageForAnim || DefaultImage,
            startRect,
            targetX,
            targetY,
          });
        });
      }

      // Close modal
      setOptionsModalOpen(false);
      setSelectedProductForOptions(null);
    },
    [selectedProductForOptions, addToCartStore, addAnimation, performanceMode]
  );

  const handleOpenCashDrawer = useCallback(async () => {
    try {
      const { openCashDrawer } = await import('@/infrastructure/print/printService');
      await openCashDrawer(selectedPrinter || undefined);
      toast.success(t('app.action.cash_drawer_opened'));
    } catch (error) {
      console.error('Failed to open cash drawer:', error);
      toast.error(t('app.action.cash_drawer_failed'));
    }
  }, [t, selectedPrinter]);

  const handleManageTableWithId = useCallback(() => {
    setManageTableId(currentOrderKey);
    setShowTableScreen(true);
  }, [currentOrderKey, setShowTableScreen]);

  const handleCloseDraftModal = useCallback(() => {
    setShowDraftModal(false);
  }, [setShowDraftModal]);

  const handleCloseTableScreen = useCallback(() => {
    setShowTableScreen(false);
    setManageTableId(null);
  }, [setShowTableScreen]);

  const handleNavigateCheckout = useCallback((tableId: string) => {
    handleCheckoutStart(tableId);
  }, [handleCheckoutStart]);

  const logout = useAuthStore((state) => state.logout);
  const user = useAuthStore((state) => state.user);
  const { currentShift, clearShift } = useShiftStore();
  const [exitDialog, setExitDialog] = useState({ open: false, title: '', description: '', isBlocking: false });
  const [showCloseShiftModal, setShowCloseShiftModal] = useState(false);

  // 真正的登出操作 (清除 auth 和 shift)
  const handleLogout = useCallback(() => {
    clearShift();
    logout();
  }, [logout, clearShift]);

  const handleSidebarCheckout = useCallback(() => {
    handleCheckoutStart(cart.length > 0 ? null : currentOrderKey);
  }, [handleCheckoutStart, cart.length, currentOrderKey]);

  const handleRequestExit = useCallback(async () => {
    const store = useActiveOrdersStore.getState();

    const active = store.getActiveOrders();
    const retailActive = active.filter((o) => o.is_retail === true);

    // Void retail orders
    for (const snapshot of retailActive) {
      try {
        await voidOrder(snapshot.order_id, { voidType: 'CANCELLED', note: 'Retail session cancelled on logout' });
      } catch {
        // Ignore errors - best effort cleanup
      }
    }

    // Check for remaining non-retail orders
    const remaining = store
      .getActiveOrders()
      .filter((o) => o.is_retail !== true);

    if (remaining && remaining.length > 0) {
      const names = remaining.map((o) => o.table_name || o.order_id).slice(0, 5).join('、');
      const moreText = remaining.length > 5 ? ` ${t('app.logout.and_more', { count: remaining.length })}` : '';
      setExitDialog({
        open: true,
        title: t('app.logout.blocked'),
        description:
          (t('app.logout.description')) + `\n${names}${moreText}\n\n` +
          (t('app.logout.hint')),
        isBlocking: true,
      });
    } else {
      // 检查是否有打开的班次
      if (currentShift) {
        // 有班次，需要先收班
        setShowCloseShiftModal(true);
      } else {
        // 没有班次，直接登出
        handleLogout();
      }
    }
  }, [t, handleLogout, currentShift]);

  // 收班成功后登出
  const handleCloseShiftSuccess = useCallback(() => {
    setShowCloseShiftModal(false);
    handleLogout();
  }, [handleLogout]);

  const overlaysProps = useMemo(
    () => ({
      screen,
      viewMode,
      checkoutOrder,
      onCheckoutCancel: handleCheckoutCancel,
      onCheckoutComplete: handleCheckoutComplete,
    }),
    [screen, viewMode, checkoutOrder, handleCheckoutCancel, handleCheckoutComplete]
  );

  if (!coreDataReady) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="w-8 h-8 border-4 border-primary-500/30 border-t-primary-500 rounded-full animate-spin mx-auto mb-3" />
          <p className="text-gray-600">{t('pos.loading_core_data')}</p>
        </div>
      </div>
    );
  }

	return (
		<div className="relative h-full w-full overflow-hidden bg-gray-100 font-sans">
      <CartAnimationLayer />

      {/* Modals */}
      <POSModals
        showDraftModal={showDraftModal}
        draftOrders={draftOrders}
        onCloseDraftModal={handleCloseDraftModal}
        onRestoreDraft={handleRestoreDraft}
        onDeleteDraft={handleDeleteDraft}
        showTableScreen={showTableScreen}
        heldOrders={heldOrders}
        cart={cart}
        onSelectTable={handleTableSelect}
        onCloseTableScreen={handleCloseTableScreen}
        manageTableId={manageTableId}
        onNavigateCheckout={handleNavigateCheckout}
      />

			{/* Main Layout */}
			<div
				className={`flex h-full w-full transition-all duration-500 ease-[cubic-bezier(0.32,0.72,0,1)] ${
							  screen === 'HISTORY' ? 'scale-[0.96] opacity-60 brightness-95' : 'scale-100 opacity-100'
						}`}
				>
				{/* Left Column */}
				<div className="flex flex-col relative z-30 w-[calc(350px+3rem)] shrink-0">
          <ActionBar
            screen={screen}
            isDbOnline={isDbOnline}
            onSetScreen={setScreen}
            onOpenCashDrawer={handleOpenCashDrawer}
            onRequestExit={handleRequestExit}
          />

          {/* Sidebar */}
          <div className="flex-1 relative bg-white overflow-hidden border-r border-gray-200 shadow-xl">
          <Sidebar
            currentOrderNumber={currentOrderKey}
            onManageTable={handleManageTable}
            onSaveDraft={handleSaveDraft}
            onRestoreDraft={handleOpenDraftModal}
            onCheckout={handleSidebarCheckout}
          />
          </div>
        </div>

				{/* Right Column */}
				<div className="flex-1 flex flex-col min-w-0 bg-gray-100 relative z-10">
          {/* Category Nav */}
          <div className="shrink-0 bg-primary-500">
            <CategoryNav
              selected={selectedCategory}
              onSelect={setSelectedCategory}
              categories={categories}
            />
          </div>


          {/* Product Grid */}
          <ProductGrid
            products={filteredProducts}
            isLoading={isProductLoading}
            onAdd={addToCart}
            onLongPress={handleLongPressProduct}
          />
        </div>
      </div>

      <POSOverlays
        {...overlaysProps}
        onSetScreen={setScreen}
        onManageTable={handleManageTableWithId}
      />
        
      <ProductModal />

      <ConfirmDialog
        isOpen={exitDialog.open}
        title={exitDialog.title}
        description={exitDialog.description}
        variant={exitDialog.isBlocking ? "danger" : "warning"}
        confirmText={exitDialog.isBlocking ? (t('common.dialog.ok')) : undefined}
        showCancel={!exitDialog.isBlocking}
        onConfirm={() => {
          setExitDialog((d) => ({ ...d, open: false }));
          if (!exitDialog.isBlocking) {
            handleLogout();
          }
        }}
        onCancel={() => setExitDialog((d) => ({ ...d, open: false }))}
      />

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

      {/* 收班弹窗 (登出前) */}
      <ShiftActionModal
        open={showCloseShiftModal}
        action="close"
        shift={currentShift}
        onClose={() => setShowCloseShiftModal(false)}
        onSuccess={handleCloseShiftSuccess}
      />
    </div>
  );
};
