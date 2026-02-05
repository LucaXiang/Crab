import React, { useState, useMemo } from 'react';
import { X, Search, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useProductStore } from '@/core/stores/resources';
import { useCategoryStore } from '@/features/category/store';
import { formatCurrency } from '@/utils/currency';
import type { Product } from '@/core/domain/types/api';

interface ProductPickerProps {
  isOpen: boolean;
  selectedProductId: string | null;
  onSelect: (product: Product) => void;
  onClose: () => void;
}

export const ProductPicker: React.FC<ProductPickerProps> = ({
  isOpen,
  selectedProductId,
  onSelect,
  onClose,
}) => {
  const { t } = useI18n();
  const products = useProductStore(state => state.items);
  const categories = useCategoryStore(state => state.items);

  const [searchQuery, setSearchQuery] = useState('');
  const [selectedCategoryId, setSelectedCategoryId] = useState<string | null>(null);

  if (!isOpen) return null;

  // Filter products
  const filteredProducts = useMemo(() => {
    let result = products.filter(p => p.is_active);

    // Filter by category
    if (selectedCategoryId) {
      result = result.filter(p => p.category === selectedCategoryId);
    }

    // Filter by search
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        p =>
          p.name.toLowerCase().includes(q) ||
          (p.external_id?.toString().includes(q))
      );
    }

    return result;
  }, [products, selectedCategoryId, searchQuery]);

  // Active categories (that have products)
  const activeCategories = useMemo(() => {
    const categoryIds = new Set(products.filter(p => p.is_active).map(p => p.category));
    return categories.filter(c => categoryIds.has(c.id));
  }, [categories, products]);

  const handleSelect = (product: Product) => {
    onSelect(product);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl max-h-[85vh] overflow-hidden animate-in zoom-in-95 flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-200 shrink-0">
          <h3 className="text-lg font-bold text-gray-900">
            {t('settings.price_rule.picker.select_product')}
          </h3>
          <button
            onClick={onClose}
            className="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Search */}
        <div className="px-5 py-3 border-b border-gray-100 shrink-0">
          <div className="relative">
            <Search
              size={18}
              className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"
            />
            <input
              type="text"
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              placeholder={t('settings.price_rule.picker.search_product')}
              className="w-full pl-10 pr-4 py-2.5 bg-gray-100 rounded-xl text-sm outline-none focus:ring-2 focus:ring-teal-500 focus:bg-white transition-all"
            />
          </div>
        </div>

        {/* Category filter */}
        <div className="px-5 py-3 border-b border-gray-100 shrink-0 overflow-x-auto">
          <div className="flex items-center gap-2">
            <button
              onClick={() => setSelectedCategoryId(null)}
              className={`
                px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-colors
                ${!selectedCategoryId
                  ? 'bg-teal-500 text-white'
                  : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                }
              `}
            >
              {t('common.all')}
            </button>
            {activeCategories.map(cat => (
              <button
                key={cat.id}
                onClick={() => setSelectedCategoryId(cat.id)}
                className={`
                  px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-colors
                  ${selectedCategoryId === cat.id
                    ? 'bg-teal-500 text-white'
                    : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                  }
                `}
              >
                {cat.name}
              </button>
            ))}
          </div>
        </div>

        {/* Product grid */}
        <div className="flex-1 overflow-y-auto p-4">
          {filteredProducts.length === 0 ? (
            <div className="text-center py-12 text-gray-400">
              {t('common.empty.no_results')}
            </div>
          ) : (
            <div className="grid grid-cols-2 sm:grid-cols-3 gap-3">
              {filteredProducts.map(product => {
                const isSelected = product.id === selectedProductId;
                const basePrice = product.specs?.[0]?.price ?? 0;

                return (
                  <button
                    key={product.id}
                    onClick={() => handleSelect(product)}
                    className={`
                      relative flex flex-col items-center p-4 rounded-xl transition-all text-center
                      ${isSelected
                        ? 'bg-teal-50 ring-2 ring-teal-500'
                        : 'bg-gray-50 hover:bg-gray-100'
                      }
                    `}
                  >
                    {/* Product image or placeholder */}
                    <div
                      className={`
                        w-16 h-16 rounded-xl mb-2 flex items-center justify-center text-2xl
                        ${isSelected ? 'bg-teal-100' : 'bg-white'}
                      `}
                    >
                      {product.image ? (
                        <img
                          src={product.image}
                          alt={product.name}
                          className="w-full h-full object-cover rounded-xl"
                        />
                      ) : (
                        <span className="text-gray-300">
                          {product.name.charAt(0)}
                        </span>
                      )}
                    </div>

                    {/* Product name */}
                    <div className="font-medium text-gray-900 text-sm truncate w-full">
                      {product.name}
                    </div>

                    {/* Price */}
                    <div className="text-xs text-gray-500 mt-0.5">
                      {formatCurrency(basePrice)}
                    </div>

                    {/* Selected indicator */}
                    {isSelected && (
                      <div className="absolute top-2 right-2 w-5 h-5 rounded-full bg-teal-500 flex items-center justify-center">
                        <Check size={12} className="text-white" />
                      </div>
                    )}
                  </button>
                );
              })}
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-5 py-4 border-t border-gray-200 shrink-0">
          <button
            onClick={onClose}
            className="w-full py-3 bg-gray-100 text-gray-700 rounded-xl font-medium hover:bg-gray-200 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
        </div>
      </div>
    </div>
  );
};
