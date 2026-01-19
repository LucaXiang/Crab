import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import DefaultImage from '../../assets/reshot.svg';
import { useOrderEventStore } from '@/core/stores/order/useOrderEventStore';
import { useCanManageProducts } from '@/hooks/usePermission';
import { EntityFormModal } from '../Settings/EntityFormModal';

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
import { Product, ItemAttributeSelection, AttributeTemplate, AttributeOption, ProductSpecification, ProductAttribute } from '@/core/domain/types';

// i18n
import { useI18n } from '@/hooks/useI18n';

// Stores - New Architecture
import {
  useProducts,
  useProductsLoading,
  useCategories,
  useProductStore,
  useCategoryStore,
} from '@/core/stores/resources';
import {
  useCart,
  useCartActions,
} from '@/core/stores/cart';
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
  useSelectedPrinter,
  useUIActions,
  useSelectedCategory,
  usePOSUIActions,
} from '@/core/stores/ui';
import {
  useSettingsStore,
  useSettingsModal,
} from '@/core/stores/settings';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';

// Services
import { createApiClient } from '@/infrastructure/api';

const api = createApiClient();
import { ConfirmDialog } from '@/presentation/components/ui/ConfirmDialog';

// Hooks
import { useOrderHandlers } from '@/hooks/useOrderHandlers';
import { useDraftHandlers } from '@/hooks/useDraftHandlers';

export const POSScreen: React.FC = () => {
  const { t } = useI18n();
  const hydrateActive = useOrderEventStore((s) => s.hydrateActiveFromLocalStorage);
  useEffect(() => {
    hydrateActive();
  }, [hydrateActive]);

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
  const categories = useCategories();
  const selectedCategory = useSelectedCategory();
  const { setSelectedCategory } = usePOSUIActions();

  // Only load data on first mount (new architecture auto-handles sync)
  useEffect(() => {
    const initializeData = async () => {
      await useCategoryStore.getState().fetchAll();
      await useProductStore.getState().fetchAll();
    };
    initializeData();
  }, []); // Empty dependency array - only run on mount

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
    attributes: AttributeTemplate[];
    options: Map<string, AttributeOption[]>;
    bindings: ProductAttribute[];
    specifications?: ProductSpecification[];
    hasMultiSpec?: boolean;
  } | null>(null);
  const {
    setScreen,
    setViewMode,
    setShowTableScreen,
    setShowDraftModal,
    addAnimation,
  } = useUIActions();
  const selectedPrinter = useSelectedPrinter();

  // DB Status
  const [isDbOnline, setIsDbOnline] = useState<boolean | null>(null);

  useEffect(() => {
    // Skip health check in dev mode to reduce console noise
    if (import.meta.env.DEV) {
      setIsDbOnline(true);
      return;
    }

    let mounted = true;
    const check = async () => {
      try {
        const ok = await api.isAvailable();
        if (mounted) setIsDbOnline(ok);
      } catch {
        if (mounted) setIsDbOnline(false);
      }
    };
    check();
    const id = setInterval(check, 5000);
    return () => {
      mounted = false;
      clearInterval(id);
    };
  }, []);


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
      // Check if product has attributes or specifications
      try {
        const productWithAttrs = await api.fetchProductAttributes(String(product.id));

        const attributes = productWithAttrs.attributes || [];

        // Build options map from attributes array (unified structure)
        const optionsMap = new Map<string, AttributeOption[]>();
        (attributes as Array<AttributeTemplate & { options?: AttributeOption[] }>).forEach((attr) => {
          if (attr.options) {
            optionsMap.set(String(attr.id), attr.options.map((opt) => ({
              id: opt.id,
              uuid: '',
              name: opt.name,
              attribute_id: attr.id,
              value_code: opt.value_code || '',
              price_modifier: opt.price_modifier ?? 0,
              is_default: opt.is_default ?? false,
              display_order: 0,
              is_active: opt.is_active ?? true,
              receipt_name: opt.receipt_name || null,
              created_at: '',
              updated_at: '',
            })));
          }
        });

        // Load specifications if product has multi-spec enabled
        let specifications: ProductSpecification[] = [];
        const hasMultiSpec = product.has_multi_spec || false;
        if (hasMultiSpec) {
          try {
            const specsResponse = await api.listProductSpecs(product.id);
            specifications = specsResponse.data?.specs || [];
          } catch (error) {
            console.error('Failed to fetch specifications:', error);
          }
        }

        // Get base price from root/default spec or first spec
        type SpecWithAliases = ProductSpecification & { isRoot?: boolean; isDefault?: boolean };
        const specsWithAliases = specifications as SpecWithAliases[];
        const rootSpec = specsWithAliases.find((s) => s.is_root || s.isRoot)
          || specsWithAliases.find((s) => s.is_default || s.isDefault)
          || specsWithAliases[0];
        const basePrice = rootSpec?.price ?? (product as { price?: number }).price ?? 0;

        // CASE 1: Force Detail View (e.g. Image Click)
        // If skipQuickAdd is true, we ALWAYS open the modal, regardless of whether attributes/specs exist.
        if (skipQuickAdd) {
            setSelectedProductForOptions({
                product,
                basePrice,
                startRect,
                attributes: attributes as unknown as AttributeTemplate[],
                options: optionsMap,
                bindings: [],
                specifications,
                hasMultiSpec,
            });
            setOptionsModalOpen(true);
            return;
        }

        // CASE 2: Has Multi-Spec or Attributes -> Check if we need modal

        let selectedDefaultSpec: SpecWithAliases | undefined = undefined;

        if (hasMultiSpec) {
            // Check for default specification
            // We check for true (boolean) or 1 (integer) to be safe across serialization methods
            selectedDefaultSpec = specsWithAliases.find((s) => s.isDefault === true || s.isDefault === 1);

            // If no default specification is found, we MUST open the modal
            if (!selectedDefaultSpec) {
                setSelectedProductForOptions({
                    product,
                    basePrice,
                    startRect,
                    attributes: attributes as unknown as AttributeTemplate[],
                    options: optionsMap,
                    bindings: [],
                    specifications,
                    hasMultiSpec,
                });
                setOptionsModalOpen(true);
                return;
            }
            // If default spec exists, we continue to check attributes
        }

        if (hasMultiSpec || attributes.length > 0) {
          // Note: If hasMultiSpec is true here, we GUARANTEE we have selectedDefaultSpec

          // Try quick add logic for attributes only
          let canQuickAdd = true;
          const quickAddOptions: ItemAttributeSelection[] = [];

          if (canQuickAdd) {
            for (const attr of attributes as any[]) {
              const options = optionsMap.get(String(attr.id)) || [];

              // For now, we can't auto-select defaults without proper binding info
              // Skip attributes that don't have options
              if (options.length === 0) {
                if (attr.type?.includes('REQUIRED') || attr.type_?.includes('REQUIRED')) {
                  canQuickAdd = false;
                  break;
                }
                continue;
              }

              // Enforce Single Choice constraints: only take the first option
              const option = options[0];
              if (option) {
                quickAddOptions.push({
                  attribute_id: String(attr.id),
                  option_idx: 0,  // First option index
                  name: attr.name,
                  value: option.name,
                  price_modifier: option.price_modifier ?? 0,
                  attribute_name: attr.name,
                  option_name: option.name,
                });
              }
            }
          }

          if (canQuickAdd) {
            // Direct add with defaults (and potentially default spec)
            addToCartStore(product, quickAddOptions, 1, 0, undefined, selectedDefaultSpec);
            
            if (startRect && !performanceMode) {
                const id = `fly-${Date.now()}-${Math.random()}`;
                const targetX = 190;
                const targetY = window.innerHeight / 2;
                const imageForAnim = product.image
                  ? (/^https?:\/\//.test(product.image) ? product.image : convertFileSrc(product.image))
                  : DefaultImage;
        
                addAnimation({
                  id,
                  type: 'fly',
                  image: imageForAnim,
                  startRect,
                  targetX,
                  targetY,
                });
            }
            return;
          }

          // Cannot quick add -> Open Modal
          setSelectedProductForOptions({
            product,
            basePrice,
            startRect,
            attributes: attributes as unknown as AttributeTemplate[],
            options: optionsMap,
            bindings: [],
            specifications,
            hasMultiSpec,
          });
          setOptionsModalOpen(true);
          return;
        }

        // CASE 3: No Attributes + Not forcing modal -> Direct Add
        // (Fall through to outside try/catch)

      } catch (error) {
        console.error('Failed to fetch product attributes:', error);
        // Continue with normal add if fetch fails
      }

      // No attributes or fetch failed: add directly to cart
      if (!skipQuickAdd) {
        addToCartStore(product);
      }

      if (startRect && !performanceMode) {
        const id = `fly-${Date.now()}-${Math.random()}`;
        const targetX = 190;
        const targetY = window.innerHeight / 2;
        const imageForAnim = product.image
          ? (/^https?:\/\//.test(product.image) ? product.image : convertFileSrc(product.image))
          : DefaultImage;

        addAnimation({
          id,
          type: 'fly',
          image: imageForAnim,
          startRect,
          targetX,
          targetY,
        });
      }
    },
    [addToCartStore, addAnimation, performanceMode]
  );

  const handleOptionsConfirmed = useCallback(
    (
      selectedOptions: ItemAttributeSelection[],
      quantity: number,
      discount: number,
      authorizer?: { id: string; username: string },
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
        const imageForAnim = product.image
          ? (/^https?:\/\//.test(product.image) ? product.image : convertFileSrc(product.image))
          : DefaultImage;

        addAnimation({
          id,
          type: 'fly',
          image: imageForAnim,
          startRect,
          targetX,
          targetY,
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
      const { openCashDrawer } = await import('@/services/printService');
      await openCashDrawer(selectedPrinter || undefined);
      toast.success(t('app.action.cashDrawerOpened'));
    } catch (error) {
      console.error('Failed to open cash drawer:', error);
      toast.error(t('app.action.cashDrawerFailed'));
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
  const [exitDialog, setExitDialog] = useState({ open: false, title: '', description: '', isBlocking: false });

  const handleLogout = useCallback(() => {
    logout();
  }, [logout]);

  const handleSidebarCheckout = useCallback(() => {
    handleCheckoutStart(cart.length > 0 ? null : currentOrderKey);
  }, [handleCheckoutStart, cart.length, currentOrderKey]);

  const handleRequestExit = useCallback(() => {
    const eventStore = useOrderEventStore.getState();

    const active = eventStore.getActiveOrders();
    const retailActive = active.filter((o) => o.key.startsWith('RETAIL-'));

    retailActive.forEach((order) => {
      try {
        eventStore.voidOrder(order.key, 'Retail session cancelled on logout');
      } catch {}
    });

    const remaining = eventStore
      .getActiveOrders()
      .filter((o) => !o.key.startsWith('RETAIL-'));

    if (remaining && remaining.length > 0) {
      const names = remaining.map((o) => o.tableName || o.key).slice(0, 5).join('、');
      const moreText = remaining.length > 5 ? ` 等 ${remaining.length} 个桌台` : '';
      setExitDialog({
        open: true,
        title: t('app.logout.blocked'),
        description:
          (t('app.logout.description')) + `\n${names}${moreText}\n\n` +
          (t('app.logout.hint')),
        isBlocking: true,
      });
    } else {
      handleLogout();
    }
  }, [t, handleLogout]);

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
				<div className="flex flex-col relative z-30 w-[380px] shrink-0">
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
          <div className="shrink-0 bg-[#FF5E5E]">
            <CategoryNav
              selected={selectedCategory}
              onSelect={setSelectedCategory}
              categories={categories}
            />
          </div>


          {/* Product Grid */}
          <ProductGrid
            products={products}
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
        
      <EntityFormModal />

      <ConfirmDialog
        isOpen={exitDialog.open}
        title={exitDialog.title}
        description={exitDialog.description}
        variant={exitDialog.isBlocking ? "danger" : "warning"}
        confirmText={exitDialog.isBlocking ? (t('common.ok')) : undefined}
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
    </div>
  );
};
