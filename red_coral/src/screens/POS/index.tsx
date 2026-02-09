import React, { useCallback, useMemo, useState } from 'react';
import { logger } from '@/utils/logger';
import { useCanManageMenu } from '@/hooks/usePermission';
import { ProductModal } from '@/features/product/ProductModal';
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
import { Product } from '@/core/domain/types';

// i18n
import { useI18n } from '@/hooks/useI18n';

// Stores
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
} from '@/core/stores/order';
import { useShallow } from 'zustand/react/shallow';
import { useDraftOrderStore } from '@/core/stores/order/useDraftOrderStore';
import { useCheckoutStore } from '@/core/stores/order/useCheckoutStore';
import * as orderOps from '@/core/stores/order/commands';
import {
  useScreen,
  useViewMode,
  useModalStates,
  useReceiptPrinter,
  useUIActions,
  usePOSUIActions,
} from '@/core/stores/ui';
import {
  useSettingsModal,
} from '@/core/stores/settings';

import { ConfirmDialog } from '@/shared/components/ConfirmDialog';

// Hooks
import { useOrderHandlers } from '@/hooks/useOrderHandlers';
import { useDraftHandlers } from '@/hooks/useDraftHandlers';
import { useRetailOrderRecovery } from '@/hooks/useRetailOrderRecovery';

// Local Hooks
import { useProductFiltering, useAddToCart, useLogoutFlow } from './hooks';

export const POSScreen: React.FC = () => {
  const { t } = useI18n();

  // Permissions & Modal
  const canManageProducts = useCanManageMenu();
  const { openModal } = useSettingsModal();

  const handleLongPressProduct = useCallback((product: Product) => {
    if (canManageProducts) {
      openModal('PRODUCT', 'EDIT', product);
    } else {
      toast.error(t('auth.unauthorized.message'));
    }
  }, [canManageProducts, openModal, t]);

  // Product Filtering
  const { filteredProducts, isProductLoading, categories, selectedCategory } = useProductFiltering();
  const { setSelectedCategory } = usePOSUIActions();

  // Preload core resources on first mount
  const coreDataReady = usePreloadCoreData();

  // Cart Store
  const cart = useCart();
  const { clearCart, setCart } = useCartActions();

  // Order Store
  const heldOrders = useHeldOrders();
  const draftOrders = useDraftOrders();
  const currentOrderKey = useCurrentOrderKey();
  const checkoutOrder = useCheckoutOrder();
  const { saveDraft, restoreDraft, deleteDraft } = useDraftOrderStore(
    useShallow((s) => ({ saveDraft: s.saveDraft, restoreDraft: s.restoreDraft, deleteDraft: s.deleteDraft }))
  );
  const { setCheckoutOrder, setCurrentOrderKey } = useCheckoutStore(
    useShallow((s) => ({ setCheckoutOrder: s.setCheckoutOrder, setCurrentOrderKey: s.setCurrentOrderKey }))
  );
  const handleTableSelectStore = orderOps.handleTableSelect;

  // UI Store
  const screen = useScreen();
  const viewMode = useViewMode();
	const { showTableScreen, showDraftModal } = useModalStates();
	const [manageTableId, setManageTableId] = useState<number | null>(null);

  // Add to Cart (with product options modal)
  const {
    addToCart,
    optionsModalOpen,
    selectedProductForOptions,
    handleOptionsConfirmed,
    closeOptionsModal,
  } = useAddToCart();

  const {
    setScreen,
    setViewMode,
    setShowTableScreen,
    setShowDraftModal,
  } = useUIActions();
  const selectedPrinter = useReceiptPrinter();

  // DB Status
  const isDbOnline = useHealthCheck();

  // Custom Hooks
  const orderHandlers = useOrderHandlers({
    handleTableSelectStore,
    voidOrder: orderOps.voidOrder,
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

  // Recover unfinished retail orders (after crash/power loss)
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

  // Logout Flow
  const {
    exitDialog,
    showCloseShiftModal,
    currentShift,
    handleRequestExit,
    handleCloseShiftSuccess,
    handleDismissExitDialog,
    handleConfirmExitDialog,
    setShowCloseShiftModal,
  } = useLogoutFlow();

  const handleOpenCashDrawer = useCallback(async () => {
    try {
      const { openCashDrawer } = await import('@/infrastructure/print/printService');
      await openCashDrawer(selectedPrinter || undefined);
      toast.success(t('app.action.cash_drawer_opened'));
    } catch (error) {
      logger.error('Failed to open cash drawer', error);
      toast.error(t('app.action.cash_drawer_failed'));
    }
  }, [t, selectedPrinter]);

  const handleManageTableWithId = useCallback(() => {
    setManageTableId(typeof currentOrderKey === 'number' ? currentOrderKey : null);
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

  const handleSidebarCheckout = useCallback(() => {
    handleCheckoutStart(cart.length > 0 ? null : currentOrderKey);
  }, [handleCheckoutStart, cart.length, currentOrderKey]);

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
        onConfirm={handleConfirmExitDialog}
        onCancel={handleDismissExitDialog}
      />

      {/* Product Options Modal */}
      {selectedProductForOptions && (
        <ProductOptionsModal
          isOpen={optionsModalOpen}
          onClose={closeOptionsModal}
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

      {/* Close Shift Modal (before logout) */}
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
