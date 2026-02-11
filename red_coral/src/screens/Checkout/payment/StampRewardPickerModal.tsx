/**
 * StampRewardPickerModal - Product picker for stamp selection mode.
 *
 * Filters products by reward_targets (categories/products),
 * lets user pick one, then calls redeemStamp with the product_id.
 */

import React, { useMemo } from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency';
import { useProducts } from '@/features/product';
import { useCategories } from '@/features/category';
import type { StampRewardTarget, Product } from '@/core/domain/types/api';

interface StampRewardPickerModalProps {
  isOpen: boolean;
  activityName: string;
  rewardTargets: StampRewardTarget[];
  rewardQuantity: number;
  onSelect: (productId: number) => void;
  onClose: () => void;
}

export const StampRewardPickerModal: React.FC<StampRewardPickerModalProps> = ({
  isOpen,
  activityName,
  rewardTargets,
  rewardQuantity,
  onSelect,
  onClose,
}) => {
  const { t } = useI18n();
  const products = useProducts();
  const categories = useCategories();

  const targetCategoryIds = useMemo(
    () => rewardTargets.filter(rt => rt.target_type === 'CATEGORY').map(rt => rt.target_id),
    [rewardTargets],
  );
  const targetProductIds = useMemo(
    () => rewardTargets.filter(rt => rt.target_type === 'PRODUCT').map(rt => rt.target_id),
    [rewardTargets],
  );

  // Group eligible products by category
  const groupedProducts = useMemo(() => {
    const eligible = products.filter(p =>
      p.is_active && (
        targetProductIds.includes(p.id) ||
        targetCategoryIds.includes(p.category_id)
      ),
    );

    const groups = new Map<string, Product[]>();
    for (const p of eligible) {
      const cat = categories.find(c => c.id === p.category_id);
      const catName = cat?.name ?? t('common.uncategorized');
      const list = groups.get(catName) ?? [];
      list.push(p);
      groups.set(catName, list);
    }
    return groups;
  }, [products, categories, targetProductIds, targetCategoryIds, t]);

  if (!isOpen) return null;

  const getDefaultPrice = (p: Product) => {
    const spec = p.specs?.find(s => s.is_default) ?? p.specs?.[0];
    return spec?.price ?? 0;
  };

  return (
    <div className="fixed inset-0 bg-black/60 z-60 flex items-center justify-center">
      <div className="bg-white rounded-2xl shadow-2xl w-[640px] max-h-[80vh] flex flex-col">
        {/* Header */}
        <div className="p-4 border-b flex items-center justify-between">
          <div>
            <h3 className="text-lg font-bold text-gray-900">
              {t('checkout.stamp.select_reward')}
            </h3>
            <p className="text-sm text-gray-500">
              {activityName} · {t('checkout.stamp.reward_quantity', { count: rewardQuantity })}
            </p>
          </div>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-lg transition-colors">
            <X size={20} />
          </button>
        </div>

        {/* Product list */}
        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {groupedProducts.size === 0 && (
            <div className="text-center text-gray-400 py-8">
              {t('checkout.stamp.no_reward_products')}
            </div>
          )}
          {[...groupedProducts.entries()].map(([catName, prods]) => (
            <div key={catName}>
              <h4 className="text-xs font-bold text-gray-400 uppercase mb-2">{catName}</h4>
              <div className="grid grid-cols-3 gap-2">
                {prods.map(p => {
                  const price = getDefaultPrice(p);
                  return (
                    <button
                      key={p.id}
                      onClick={() => onSelect(p.id)}
                      className="p-3 bg-gray-50 hover:bg-violet-50 hover:ring-2 hover:ring-violet-300 rounded-xl text-left transition-all"
                    >
                      <div className="text-sm font-medium text-gray-900 truncate">{p.name}</div>
                      <div className="text-xs text-gray-500 mt-1">{formatCurrency(price)}</div>
                    </button>
                  );
                })}
              </div>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
