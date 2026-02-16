import React from 'react';
import { useI18n } from '@/hooks/useI18n';
import { X, ShoppingBag } from 'lucide-react';
import { formatCurrency } from '@/utils/currency';
import { CartItem as CartItemType } from '@/core/domain/types';
import { ProductGrid } from '@/screens/POS/components/ProductGrid';
import { CategoryNav } from '@/presentation/components/CategoryNav';
import { CartList } from '@/presentation/components/cart/CartList';
import { CartItemDetailModal } from '@/presentation/components/modals/CartItemDetailModal';
import { ProductOptionsModal } from '@/presentation/components/modals/ProductOptionsModal';
import { useQuickAddCart } from './useQuickAddCart';


interface QuickAddModalProps {
  onClose: () => void;
  onConfirm: (items: CartItemType[]) => void;
}

export const QuickAddModal: React.FC<QuickAddModalProps> = ({ onClose, onConfirm }) => {
  const { t } = useI18n();

  const {
    categories,
    isLoading,
    filteredProducts,
    tempItems,
    editingItem,
    optionsModalOpen,
    selectedProductForOptions,
    totalAmount,
    selectedCategory,
    setSelectedCategory,
    handleAddProduct,
    handleOptionsConfirmed,
    handleQuantityChange,
    handleItemClick,
    handleUpdateItem,
    handleRemoveItem,
    handleConfirm,
    closeOptionsModal,
    closeEditingItem,
  } = useQuickAddCart({ onClose, onConfirm });

  return (
    <div className="fixed inset-0 z-60 bg-black/60 backdrop-blur-md flex items-center justify-center p-4 sm:p-6 animate-in fade-in duration-200">
      {/* Cart Item Edit Modal */}
      {editingItem && (
        <CartItemDetailModal
          item={editingItem}
          onClose={closeEditingItem}
          onUpdate={handleUpdateItem}
          onRemove={handleRemoveItem}
        />
      )}

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
