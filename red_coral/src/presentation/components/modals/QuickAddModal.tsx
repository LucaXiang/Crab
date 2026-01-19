import React, { useState, useMemo, useEffect } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { X, ShoppingBag, Plus, Trash2, Minus, Search } from 'lucide-react';
import { formatCurrency } from '@/utils/formatCurrency';
import { CartItem } from '@/core/domain/types';
import { v4 as uuidv4 } from 'uuid';
import { useCategories, useCategoryStore } from '@/core/stores/resources/useCategoryStore';
import { useProducts, useProductStore } from '@/core/stores/resources/useProductStore';
import { ProductCard } from '@/presentation/components/ProductCard';
import clsx from 'clsx';
import { mergeItemsIntoList } from '@/core/services/order/eventReducer';

interface QuickAddModalProps {
  onClose: () => void;
  onConfirm: (items: CartItem[]) => void;
}

export const QuickAddModal: React.FC<QuickAddModalProps> = ({ onClose, onConfirm }) => {
  const { t } = useI18n();

  // Use new resources stores
  const categories = useCategories();
  const products = useProducts();

  const [selectedCategoryId, setSelectedCategoryId] = useState<string>('all');
  const [searchQuery, setSearchQuery] = useState('');
  const [tempItems, setTempItems] = useState<CartItem[]>([]);

  // Ensure data is loaded
  useEffect(() => {
    useCategoryStore.getState().fetchAll();
    useProductStore.getState().fetchAll();
  }, []);

  // Convert store Product to display format
  interface ProductForDisplay {
    id: string;
    name: string;
    price?: number;
    image?: string | null;
    category?: string | null;
  }
  const domainProducts: ProductForDisplay[] = useMemo(() => {
    return products.map((p: any): ProductForDisplay => ({
      id: String(p.id),
      name: p.name,
      image: p.image,
      category: p.category ?? p.category_id ?? null,
      price: p.price ?? 0,
    }));
  }, [products]);

  // Filter products
  const filteredProducts = useMemo(() => {
    let result = domainProducts;

    // Category filter
    if (selectedCategoryId !== 'all') {
      const category = categories.find(c => c.id === selectedCategoryId);
      if (category) {
        result = result.filter(p => p.category === category.name);
      }
    }

    // Search filter
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      result = result.filter(p =>
        p.name.toLowerCase().includes(q)
      );
    }

    return result;
  }, [domainProducts, selectedCategoryId, searchQuery, categories]);

  const handleAddProduct = (product: ProductForDisplay) => {
    // Convert domain Product to CartItem
    const newItem: CartItem = {
      id: String(product.id),
      productId: String(product.id),
      name: product.name,
      price: product.price ?? 0,
      quantity: 1,
      instanceId: uuidv4(),
      selectedOptions: [],
    };

    setTempItems(prev => {
      // Use the centralized merge logic
      return mergeItemsIntoList(prev, [newItem]);
    });
  };

  const updateQuantity = (index: number, delta: number) => {
    setTempItems(prev => {
      const newItems = [...prev];
      const item = newItems[index];
      const newQty = item.quantity + delta;
      
      if (newQty <= 0) {
        return prev.filter((_, i) => i !== index);
      }
      
      newItems[index] = { ...item, quantity: newQty };
      return newItems;
    });
  };

  const totalAmount = tempItems.reduce((sum, item) => sum + (item.price * item.quantity), 0);

  const handleConfirm = () => {
    if (tempItems.length > 0) {
      onConfirm(tempItems);
    }
    onClose();
  };

  return (
    <div className="fixed inset-0 z-60 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div 
        className="bg-white rounded-2xl shadow-2xl w-full max-w-6xl h-[85vh] overflow-hidden flex flex-col animate-in zoom-in-95 duration-200"
        onClick={e => e.stopPropagation()}
      >
        {/* Header */}
        <div className="p-4 border-b border-gray-100 flex justify-between items-center bg-gray-50 shrink-0">
          <h2 className="text-xl font-bold text-gray-800 flex items-center gap-2">
            <ShoppingBag className="text-red-500" />
            {t('pos.quickAdd.title')}
          </h2>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-200 rounded-full transition-colors">
            <X size={24} className="text-gray-500" />
          </button>
        </div>

        {/* Main Content */}
        <div className="flex flex-1 overflow-hidden">
          {/* Left Side: Product Selection (70%) */}
          <div className="w-[70%] flex flex-col border-r border-gray-100 bg-gray-50/50">
            {/* Filter Bar */}
            <div className="p-4 bg-white border-b border-gray-100 space-y-4">
              {/* Search */}
              <div className="relative">
                <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" size={20} />
                <input
                  type="text"
                  placeholder={t('pos.searchProduct')}
                  className="w-full pl-10 pr-4 py-2 bg-gray-100 border-none rounded-lg focus:ring-2 focus:ring-red-500/20 text-gray-800 placeholder:text-gray-400"
                  value={searchQuery}
                  onChange={e => setSearchQuery(e.target.value)}
                />
              </div>

              {/* Categories */}
              <div className="flex gap-2 overflow-x-auto pb-2 scrollbar-hide">
                <button
                  onClick={() => setSelectedCategoryId('all')}
                  className={clsx(
                    "px-4 py-2 rounded-full text-sm font-bold whitespace-nowrap transition-all",
                    selectedCategoryId === 'all'
                      ? "bg-gray-900 text-white shadow-md"
                      : "bg-white text-gray-600 border border-gray-200 hover:bg-gray-50"
                  )}
                >
                  {t('pos.allCategories')}
                </button>
                {categories.map(cat => (
                  <button
                    key={cat.id}
                    onClick={() => setSelectedCategoryId(cat.id)}
                    className={clsx(
                      "px-4 py-2 rounded-full text-sm font-bold whitespace-nowrap transition-all",
                      selectedCategoryId === cat.id
                        ? "bg-red-500 text-white shadow-md shadow-red-500/20"
                        : "bg-white text-gray-600 border border-gray-200 hover:bg-gray-50"
                    )}
                  >
                    {cat.name}
                  </button>
                ))}
              </div>
            </div>

            {/* Product Grid */}
            <div className="flex-1 overflow-y-auto p-4">
              {filteredProducts.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-gray-400">
                  <p className="text-lg">{t('pos.quickAdd.noProducts')}</p>
                </div>
              ) : (
                <div className="grid grid-cols-3 lg:grid-cols-4 gap-3">
                  {filteredProducts.map(product => (
                    <ProductCard
                      key={product.id}
                      product={product as any}
                      onAdd={() => handleAddProduct(product)}
                    />
                  ))}
                </div>
              )}
            </div>
          </div>

          {/* Right Side: Cart (30%) */}
          <div className="w-[30%] flex flex-col bg-white">
            <div className="p-4 border-b border-gray-100 bg-gray-50">
              <h3 className="font-bold text-gray-700">{t('pos.quickAdd.selectedItems')} ({tempItems.length})</h3>
            </div>

            <div className="flex-1 overflow-y-auto p-4 space-y-3">
              {tempItems.length === 0 ? (
                <div className="h-full flex flex-col items-center justify-center text-gray-400">
                  <ShoppingBag size={48} className="mb-2 opacity-20" />
                  <p>{t('pos.quickAdd.selectPrompt')}</p>
                </div>
              ) : (
                tempItems.map((item, index) => (
                  <div key={item.instanceId || index} className="flex flex-col p-3 bg-white border border-gray-100 rounded-xl shadow-sm gap-2">
                    <div className="flex justify-between items-start">
                      <span className="font-bold text-gray-800 line-clamp-2">{item.name}</span>
                      <span className="font-bold text-gray-900">{formatCurrency(item.price * item.quantity)}</span>
                    </div>

                    <div className="flex justify-between items-center">
                      <span className="text-xs text-gray-500">{formatCurrency(item.price)} {t('pos.quickAdd.perUnit')}</span>
                      <div className="flex items-center gap-3 bg-gray-50 rounded-lg p-1">
                        <button 
                          onClick={() => updateQuantity(index, -1)}
                          className="p-1 hover:bg-white rounded-md shadow-sm text-gray-600 transition-all"
                        >
                          {item.quantity === 1 ? <Trash2 size={14} className="text-red-500" /> : <Minus size={14} />}
                        </button>
                        <span className="font-bold w-6 text-center text-sm">{item.quantity}</span>
                        <button 
                          onClick={() => updateQuantity(index, 1)}
                          className="p-1 hover:bg-white rounded-md shadow-sm text-gray-600 transition-all"
                        >
                          <Plus size={14} />
                        </button>
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>

            {/* Footer */}
            <div className="p-4 border-t border-gray-100 bg-white shrink-0">
              <div className="flex justify-between items-center mb-4">
                <span className="text-gray-500">{t('checkout.amount.total')}</span>
                <span className="text-2xl font-bold text-gray-900">{formatCurrency(totalAmount)}</span>
              </div>
              <button
                onClick={handleConfirm}
                disabled={tempItems.length === 0}
                className="w-full py-3 bg-red-500 hover:bg-red-600 disabled:opacity-50 disabled:cursor-not-allowed text-white rounded-xl font-bold text-lg shadow-lg shadow-red-500/30 transition-all active:scale-[0.98]"
              >
                {t('pos.quickAdd.confirm')}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};
