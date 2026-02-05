import React, { useState, useMemo } from 'react';
import { X, Search, Check, Layers, Tag, Package } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useProductStore } from '@/core/stores/resources';
import { useCategoryStore } from '@/features/category/store';
import { useTagStore } from '@/features/tag/store';
import { formatCurrency } from '@/utils/currency';
import type { ProductScope } from '@/core/domain/types/api';

interface TargetPickerProps {
  isOpen: boolean;
  productScope: ProductScope;
  selectedTarget: string | null;
  onSelect: (target: string) => void;
  onClose: () => void;
}

export const TargetPicker: React.FC<TargetPickerProps> = ({
  isOpen,
  productScope,
  selectedTarget,
  onSelect,
  onClose,
}) => {
  const { t } = useI18n();
  const products = useProductStore(state => state.items);
  const categories = useCategoryStore(state => state.items);
  const tags = useTagStore(state => state.items);

  const [searchQuery, setSearchQuery] = useState('');

  // Build and filter items based on scope and search
  const items = useMemo(() => {
    type Item = {
      id: string;
      name: string;
      icon: React.ElementType;
      subtitle?: string;
      image?: string;
    };

    let allItems: Item[] = [];

    switch (productScope) {
      case 'CATEGORY':
        allItems = categories.map(c => ({
          id: c.id,
          name: c.name,
          icon: Layers,
        }));
        break;
      case 'TAG':
        allItems = tags.map(tg => ({
          id: tg.id,
          name: tg.name,
          icon: Tag,
        }));
        break;
      case 'PRODUCT':
        allItems = products
          .filter(p => p.is_active)
          .map(p => ({
            id: p.id,
            name: p.name,
            icon: Package,
            subtitle: formatCurrency(p.specs?.[0]?.price ?? 0),
            image: p.image,
          }));
        break;
    }

    if (!searchQuery.trim()) return allItems;
    const q = searchQuery.toLowerCase();
    return allItems.filter(item => item.name.toLowerCase().includes(q));
  }, [productScope, categories, tags, products, searchQuery]);

  // Early return AFTER all hooks to comply with React's rules of hooks
  if (!isOpen) return null;

  const getTitle = () => {
    switch (productScope) {
      case 'CATEGORY':
        return t('settings.price_rule.picker.select_category');
      case 'TAG':
        return t('settings.price_rule.picker.select_tag');
      case 'PRODUCT':
        return t('settings.price_rule.picker.select_product');
      default:
        return '';
    }
  };

  const getPlaceholder = () => {
    switch (productScope) {
      case 'CATEGORY':
        return t('settings.price_rule.picker.search_category');
      case 'TAG':
        return t('settings.price_rule.picker.search_tag');
      case 'PRODUCT':
        return t('settings.price_rule.picker.search_product');
      default:
        return '';
    }
  };

  const handleSelect = (id: string) => {
    onSelect(id);
    onClose();
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-md max-h-[80vh] overflow-hidden animate-in zoom-in-95 flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-200 shrink-0">
          <h3 className="text-lg font-bold text-gray-900">{getTitle()}</h3>
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
              placeholder={getPlaceholder()}
              className="w-full pl-10 pr-4 py-2.5 bg-gray-100 rounded-xl text-sm outline-none focus:ring-2 focus:ring-teal-500 focus:bg-white transition-all"
            />
          </div>
        </div>

        {/* Items */}
        <div className="flex-1 overflow-y-auto p-4 space-y-2">
          {items.length === 0 ? (
            <div className="text-center py-12 text-gray-400">
              {t('common.empty.no_results')}
            </div>
          ) : (
            items.map(item => {
              const Icon = item.icon;
              const isSelected = selectedTarget === item.id;

              return (
                <button
                  key={item.id}
                  onClick={() => handleSelect(item.id)}
                  className={`
                    w-full flex items-center gap-4 p-4 rounded-xl transition-all
                    ${isSelected
                      ? 'bg-teal-50 ring-2 ring-teal-500'
                      : 'bg-gray-50 hover:bg-gray-100'
                    }
                  `}
                >
                  <div
                    className={`
                      w-12 h-12 rounded-xl flex items-center justify-center shrink-0 overflow-hidden
                      ${isSelected ? 'bg-teal-500 text-white' : 'bg-white text-gray-500'}
                    `}
                  >
                    {'image' in item && item.image ? (
                      <img
                        src={item.image}
                        alt={item.name}
                        className="w-full h-full object-cover"
                      />
                    ) : (
                      <Icon size={24} />
                    )}
                  </div>
                  <div className="flex-1 text-left min-w-0">
                    <div className="font-medium text-gray-900 truncate">{item.name}</div>
                    {item.subtitle && (
                      <div className="text-sm text-gray-500">{item.subtitle}</div>
                    )}
                  </div>
                  {isSelected && (
                    <div className="w-6 h-6 rounded-full bg-teal-500 flex items-center justify-center shrink-0">
                      <Check size={14} className="text-white" />
                    </div>
                  )}
                </button>
              );
            })
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
