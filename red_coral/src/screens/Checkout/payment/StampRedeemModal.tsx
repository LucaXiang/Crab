/**
 * StampRedeemModal - Smart stamp redemption modal.
 *
 * Shows different options based on strategy and order state:
 * - Designated + matching item in order: "comp existing" + "add new" options
 * - Designated + no matching item: "add new" only
 * - Eco/Gen with excess + matching items: "match" + "select" options
 * - Eco/Gen without excess or no matching: "select" only
 */

import React from 'react';
import { X, Gift, ShoppingBasket, Target } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency';
import type { MemberStampProgressDetail } from '@/core/domain/types/api';
import type { CartItemSnapshot } from '@/core/domain/types/orderEvent';

interface StampRedeemModalProps {
  isOpen: boolean;
  activity: MemberStampProgressDetail;
  matchingItems: CartItemSnapshot[];
  hasExcess: boolean;
  effectiveStamps: number;
  orderBonus: number;
  isProcessing: boolean;
  onMatchRedeem: () => void;
  onSelectRedeem: () => void;
  onDirectRedeem: () => void;
  onClose: () => void;
}

export const StampRedeemModal: React.FC<StampRedeemModalProps> = ({
  isOpen,
  activity,
  matchingItems,
  hasExcess,
  effectiveStamps,
  orderBonus,
  isProcessing,
  onMatchRedeem,
  onSelectRedeem,
  onDirectRedeem,
  onClose,
}) => {
  const { t } = useI18n();
  if (!isOpen) return null;

  const sp = activity;
  const isDesignated = sp.reward_strategy === 'DESIGNATED';
  const hasDesignatedMatch = isDesignated && matchingItems.length > 0;
  const showMatchOption = !isDesignated && hasExcess && matchingItems.length > 0;
  const showSelectOption = !isDesignated && (sp.reward_targets?.length > 0);

  // For match mode preview: find which item would be comped
  const bestMatch = showMatchOption
    ? sp.reward_strategy === 'ECONOMIZADOR'
      ? matchingItems.reduce((a, b) => a.original_price <= b.original_price ? a : b)
      : matchingItems.reduce((a, b) => a.original_price >= b.original_price ? a : b)
    : hasDesignatedMatch
      ? matchingItems[0]
      : null;

  return (
    <div className="fixed inset-0 bg-black/60 z-50 flex items-center justify-center">
      <div className="bg-white rounded-2xl shadow-2xl w-[480px] flex flex-col">
        {/* Header */}
        <div className="p-5 border-b flex items-center justify-between">
          <div>
            <h3 className="text-lg font-bold text-gray-900">{sp.stamp_activity_name}</h3>
            <p className="text-sm text-gray-500 mt-0.5">
              {t('checkout.stamp.progress_detail', {
                effective: effectiveStamps,
                required: sp.stamps_required,
                bonus: orderBonus,
              })}
            </p>
          </div>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-lg transition-colors">
            <X size={20} />
          </button>
        </div>

        {/* Options */}
        <div className="p-5 space-y-3">
          {isDesignated ? (
            /* Designated: comp existing + add new (or just add new if no match) */
            <>
              {hasDesignatedMatch && bestMatch && (
                <button
                  onClick={() => { onMatchRedeem(); onClose(); }}
                  disabled={isProcessing}
                  className="w-full p-5 bg-emerald-50 hover:bg-emerald-100 border-2 border-emerald-200 hover:border-emerald-400 rounded-xl text-left transition-all disabled:opacity-50 flex items-center gap-4"
                >
                  <div className="w-12 h-12 bg-emerald-500 rounded-xl flex items-center justify-center flex-shrink-0">
                    <Target size={24} className="text-white" />
                  </div>
                  <div className="flex-1">
                    <div className="font-bold text-gray-900">{t('checkout.stamp.comp_existing')}</div>
                    <div className="text-sm text-gray-500 mt-0.5">
                      {t('checkout.stamp.comp_existing_desc', { name: bestMatch.name, price: formatCurrency(bestMatch.original_price) })}
                    </div>
                  </div>
                </button>
              )}
              <button
                onClick={() => { onDirectRedeem(); onClose(); }}
                disabled={isProcessing}
                className="w-full p-5 bg-violet-50 hover:bg-violet-100 border-2 border-violet-200 hover:border-violet-400 rounded-xl text-left transition-all disabled:opacity-50 flex items-center gap-4"
              >
                <div className="w-12 h-12 bg-violet-500 rounded-xl flex items-center justify-center flex-shrink-0">
                  <Gift size={24} className="text-white" />
                </div>
                <div className="flex-1">
                  <div className="font-bold text-gray-900">{t('checkout.stamp.add_new')}</div>
                  <div className="text-sm text-gray-500 mt-0.5">
                    {t('checkout.stamp.add_new_desc')}
                  </div>
                </div>
              </button>
            </>
          ) : (
            /* Eco/Gen: match and/or select */
            <>
              {showMatchOption && bestMatch && (
                <button
                  onClick={() => { onMatchRedeem(); onClose(); }}
                  disabled={isProcessing}
                  className="w-full p-5 bg-emerald-50 hover:bg-emerald-100 border-2 border-emerald-200 hover:border-emerald-400 rounded-xl text-left transition-all disabled:opacity-50 flex items-center gap-4"
                >
                  <div className="w-12 h-12 bg-emerald-500 rounded-xl flex items-center justify-center flex-shrink-0">
                    <Target size={24} className="text-white" />
                  </div>
                  <div className="flex-1">
                    <div className="font-bold text-gray-900">{t('checkout.stamp.match_redeem')}</div>
                    <div className="text-sm text-gray-500 mt-0.5">
                      {t('checkout.stamp.match_desc', {
                        name: bestMatch.name,
                        price: formatCurrency(bestMatch.original_price),
                      })}
                    </div>
                  </div>
                </button>
              )}
              {showSelectOption && (
                <button
                  onClick={() => { onSelectRedeem(); onClose(); }}
                  disabled={isProcessing}
                  className="w-full p-5 bg-blue-50 hover:bg-blue-100 border-2 border-blue-200 hover:border-blue-400 rounded-xl text-left transition-all disabled:opacity-50 flex items-center gap-4"
                >
                  <div className="w-12 h-12 bg-blue-500 rounded-xl flex items-center justify-center flex-shrink-0">
                    <ShoppingBasket size={24} className="text-white" />
                  </div>
                  <div className="flex-1">
                    <div className="font-bold text-gray-900">{t('checkout.stamp.select_redeem')}</div>
                    <div className="text-sm text-gray-500 mt-0.5">
                      {t('checkout.stamp.select_desc')}
                    </div>
                  </div>
                </button>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
};
