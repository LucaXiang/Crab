import React from 'react';
import { CartItem, CheckoutMode } from '@/core/domain/types';
import { useCategories } from '@/core/stores/resources';
import { useLongPress } from '@/hooks/useLongPress';
import { CheckCircle } from 'lucide-react';
import { calculateItemFinalPrice, calculateOptionsModifier } from '@/utils/pricing';
import { Currency } from '@/utils/currency';
import { formatCurrency } from '@/utils/currency';


import { groupOptionsByAttribute } from '@/utils/formatting';

interface OrderItemsSummaryProps {
  items: CartItem[]; // The full list from order
  unpaidItems: CartItem[]; // The remaining unpaid items
  mode: CheckoutMode;
  selectedQuantities: Record<number, number>; // Keyed by index in unpaidItems
  onUpdateSelectedQty: (index: number, delta: number) => void;
  onEditItem: (item: CartItem) => void;
  t: (key: string) => string;
  surchargeExempt?: boolean;
  paidItemQuantities?: Record<string, number>;
}

interface ActiveItemRowProps {
  item: CartItem;
  remainingQty: number;
  originalIndex: number;
  mode: CheckoutMode;
  selectedQuantities: Record<number, number>;
  onUpdateSelectedQty: (index: number, delta: number) => void;
  onEditItem: (item: CartItem) => void;
  t: (key: string) => string;
  surchargeExempt?: boolean;
}

const ActiveItemRow: React.FC<ActiveItemRowProps> = ({
  item,
  remainingQty,
  originalIndex,
  mode,
  selectedQuantities,
  onUpdateSelectedQty,
  onEditItem,
  t,
  surchargeExempt,
}) => {
  const isSelected = selectedQuantities[originalIndex] > 0;
  const currentQty = selectedQuantities[originalIndex] || 0;
  const optionsModifier = calculateOptionsModifier(item.selectedOptions).toNumber();
  const basePrice = (item.originalPrice ?? item.price) + optionsModifier;
  const unitPrice = calculateItemFinalPrice({
    ...item,
    surcharge: surchargeExempt ? 0 : item.surcharge,
  }).toNumber();
  const hasDiscount = (item.discountPercent || 0) > 0 || basePrice !== unitPrice;
  const isSelectMode = mode === 'SELECT';

  const clickHandlers = useLongPress(
    () => {},
    () => {
      if (isSelectMode) {
        onEditItem(item);
      }
    },
    { delay: 500, isPreventDefault: false }
  );

  return (
    <div
      {...clickHandlers}
      className={`
        group relative bg-white border rounded-xl p-4 transition-all duration-200
        ${isSelected
          ? 'border-blue-500 ring-1 ring-blue-500 shadow-md bg-blue-50/5'
          : 'border-gray-200 hover:border-blue-300 hover:shadow-md'
        }
        ${isSelectMode ? 'cursor-pointer hover:bg-gray-50' : ''}
      `}
    >
      <div className="flex items-center justify-between gap-4">
        <div className="flex-1 min-w-0 flex flex-col justify-center">
          <div className="flex items-center gap-2 mb-1">
            <div className="font-bold text-gray-900 text-base truncate">
              {item.name} {item.selectedSpecification ? `(${item.selectedSpecification.name})` : ''}
              {item.selectedOptions && item.selectedOptions.length > 0 && (
                <div className="text-xs font-normal text-gray-500 mt-0.5 truncate">
                  {groupOptionsByAttribute(item.selectedOptions).map(g => `${g.attributeName}: ${g.optionNames.join(', ')}`).join(' | ')}
                </div>
            )}
            </div>
            {item.discountPercent ? (
              <span className="text-[10px] font-bold bg-red-100 text-red-600 px-1.5 py-0.5 rounded-full">
                -{item.discountPercent}%
              </span>
            ) : null}
            {!surchargeExempt && item.surcharge ? (
              <span className="text-[10px] font-bold bg-purple-100 text-purple-600 px-1.5 py-0.5 rounded-full">
                +{formatCurrency(item.surcharge)}
              </span>
            ) : null}
          </div>

          <div className="flex items-center justify-between w-full">
            <div className="text-sm text-gray-500 flex items-center gap-2 flex-wrap">
              <span className="font-medium text-gray-400">x{remainingQty}</span>
              <span className="w-1 h-1 bg-gray-300 rounded-full" />
              {hasDiscount ? (
                <>
                  <span className="line-through text-xs text-gray-400">{formatCurrency(basePrice)}</span>
                  <span className="font-semibold text-gray-700">{formatCurrency(unitPrice)}</span>
                </>
              ) : (
                <span className="font-semibold text-gray-700">{formatCurrency(unitPrice)}</span>
              )}
            </div>
            {item.externalId && (
              <div className="text-[10px] text-white bg-gray-900/85 font-bold font-mono px-1.5 py-0.5 rounded ml-2 backdrop-blur-[1px]">
                {item.externalId}
              </div>
            )}
          </div>
        </div>

        <div className="flex items-center gap-5">
          {!isSelectMode && (
            <div
              className={`
                flex items-center bg-gray-50 rounded-lg p-1 border transition-colors
                ${isSelected ? 'border-blue-200 bg-blue-50' : 'border-gray-200 group-hover:border-gray-300'}
              `}
              onClick={(e) => e.stopPropagation()}
            >
              <button
                onClick={() => onUpdateSelectedQty(originalIndex, -1)}
                className={`
                  w-8 h-8 flex items-center justify-center rounded-md transition-all font-bold
                  ${currentQty > 0
                    ? 'bg-white shadow-sm text-gray-700 hover:text-blue-600 hover:scale-105 active:scale-95'
                    : 'text-gray-300 cursor-not-allowed'}
                `}
                disabled={currentQty <= 0}
              >
                -
              </button>
              <div className="w-14 text-center">
                <span className={`font-bold ${currentQty > 0 ? 'text-blue-600' : 'text-gray-800'}`}>
                  {currentQty}
                </span>
                <span className="text-gray-400 text-xs mx-1">/</span>
                <span className="text-gray-500 text-xs">
                  {remainingQty}
                </span>
              </div>
              <button
                onClick={() => onUpdateSelectedQty(originalIndex, 1)}
                className={`
                  w-8 h-8 flex items-center justify-center rounded-md transition-all font-bold
                  ${currentQty < remainingQty
                    ? 'bg-white shadow-sm text-gray-700 hover:text-blue-600 hover:scale-105 active:scale-95'
                    : 'text-gray-300 cursor-not-allowed'}
                `}
                disabled={currentQty >= remainingQty}
              >
                +
              </button>
            </div>
          )}

          <div className="text-right min-w-[90px]">
            <div className="text-[10px] text-gray-400 font-bold uppercase tracking-wider mb-0.5">{t('pos.cart.subtotal')}</div>
            <div className={`font-bold text-lg ${isSelected ? 'text-blue-600' : 'text-gray-900'}`}>
              {formatCurrency(Currency.mul(unitPrice, remainingQty).toNumber())}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

export const OrderItemsSummary: React.FC<OrderItemsSummaryProps> = ({
  items,
  unpaidItems,
  mode,
  selectedQuantities,
  onUpdateSelectedQty,
  onEditItem,
  t,
  surchargeExempt,
  paidItemQuantities
}) => {
  const categories = useCategories();

  // 1. Separate into Active (Unpaid) and Completed (Paid) lists
  const activeItems: { item: CartItem; remainingQty: number; originalIndex: number }[] = [];
  const paidItemsMap: Record<string, { item: CartItem; paidQty: number }> = {};

  items.forEach((item) => {
    // Calculate paid quantity first to decide if we should process this item
    // If paidItemQuantities is provided, use it as the source of truth (supports soft-deleted items)
    let paidQty = 0;
    if (paidItemQuantities && item.instanceId && paidItemQuantities[item.instanceId] !== undefined) {
      paidQty = paidItemQuantities[item.instanceId];
    }

    // If item is removed and has no paid quantity, skip it completely
    if (item._removed && paidQty <= 0) return;

    // Find corresponding unpaid item to know how much is left
    // We match by instanceId if available, or id
    const unpaidIdx = unpaidItems.findIndex(u =>
      item.instanceId
        ? u.instanceId === item.instanceId
        : (u.id === item.id )
    );

    const remainingItem = unpaidIdx !== -1 ? unpaidItems[unpaidIdx] : null;
    const remainingQty = remainingItem ? remainingItem.quantity : 0;
    
    // Fallback calculation for non-split/legacy if not using paidItemQuantities
    // Only apply fallback if item is NOT removed (removed items shouldn't imply paid unless explicit)
    if ((!paidItemQuantities || !item.instanceId || paidItemQuantities[item.instanceId] === undefined) && !item._removed) {
      paidQty = item.quantity - remainingQty;
    }

    // Add to Active List if there's remaining quantity (and not removed)
    if (remainingQty > 0 && !item._removed) {
      activeItems.push({ item, remainingQty, originalIndex: unpaidIdx });
    }

    // Add to Paid Map if there's paid quantity
    if (paidQty > 0) {
      const effectiveUnitPrice = calculateItemFinalPrice({
            ...item,
            surcharge: surchargeExempt ? 0 : item.surcharge,
          });

          // Group Key
          const key = `${item.id}-${effectiveUnitPrice.toFixed(2)}`;

          if (!paidItemsMap[key]) {
            paidItemsMap[key] = { item, paidQty: 0 };
          }
          paidItemsMap[key].paidQty += paidQty;
        }
  });

  const completedItems = Object.values(paidItemsMap);

  // Sorting Logic
  const getCategoryIndex = (catName: string) => {
    const index = categories.findIndex(c => c.name === catName);
    return index === -1 ? 9999 : index;
  };

  const sortItems = (aItem: CartItem, bItem: CartItem) => {
    // CartItem doesn't have category/sortOrder - these are product-level properties
    // Use productId as tie-breaker or just sort by name

    // 1. Fallback: External ID (if available)
    const extIdA = aItem.externalId ? parseInt(String(aItem.externalId), 10) || 0 : 0;
    const extIdB = bItem.externalId ? parseInt(String(bItem.externalId), 10) || 0 : 0;
    if (extIdA !== extIdB) {
      return extIdA - extIdB;
    }

    // 2. Fallback: Name
    return aItem.name.localeCompare(bItem.name);
  };

  activeItems.sort((a, b) => sortItems(a.item, b.item));
  completedItems.sort((a, b) => sortItems(a.item, b.item));

  return (
    <div className="space-y-6">
      {/* Active Items */}
      <div className="space-y-3">
        {activeItems.map(({ item, remainingQty, originalIndex }, idx) => (
          <ActiveItemRow
            key={`active-${idx}`}
            item={item}
            remainingQty={remainingQty}
            originalIndex={originalIndex}
            mode={mode}
            selectedQuantities={selectedQuantities}
            onUpdateSelectedQty={onUpdateSelectedQty}
            onEditItem={onEditItem}
            t={t}
            surchargeExempt={surchargeExempt}
          />
        ))}
      </div>

      {/* Completed Items */}
      {completedItems.length > 0 && (
        <div className="border-t border-gray-100 pt-6 mt-2">
          <div className="text-xs font-bold text-gray-400 uppercase tracking-wider mb-4 flex items-center gap-2">
            <CheckCircle size={14} className="text-green-500" />
            {t('checkout.items.paid')}
          </div>
          <div className="space-y-3">
            {completedItems.map(({ item, paidQty }, idx) => {
              const optionsModifier = calculateOptionsModifier(item.selectedOptions).toNumber();
              const basePrice = (item.originalPrice ?? item.price) + optionsModifier;
              const unitPricePaid = calculateItemFinalPrice({
                ...item,
                surcharge: surchargeExempt ? 0 : item.surcharge,
              }).toNumber();
              const hasDiscount = (item.discountPercent || 0) > 0 || basePrice !== unitPricePaid;

              return (
                <div 
                  key={`paid-${idx}`}
                  className="group relative bg-gray-50/80 border border-gray-200 rounded-xl p-4 transition-all duration-200"
                >
                  <div className="flex items-center justify-between gap-4">
                    {/* Item Info */}
                    <div className="flex-1 min-w-0 flex flex-col justify-center">
                      <div className="flex items-center gap-2 mb-1">
                        <div className="font-bold text-gray-700 text-base truncate decoration-gray-400">
                          {item.name}
                          {item.selectedOptions && item.selectedOptions.length > 0 && (
                            <div className="text-xs font-normal text-gray-500 mt-0.5 space-y-0.5">
                              {groupOptionsByAttribute(item.selectedOptions).map((group, idx) => (
                                <div key={idx} className="flex items-center gap-1">
                                  <span>{group.attributeName}: {group.optionNames.join(', ')}</span>
                                  {group.totalPrice !== 0 && (
                                    <span className={group.totalPrice > 0 ? 'text-orange-600' : 'text-green-600'}>
                                      ({group.totalPrice > 0 ? '+' : ''}{formatCurrency(group.totalPrice)})
                                    </span>
                                  )}
                                </div>
                              ))}
                            </div>
                          )}
                        </div>
                         {/* Tags */}
                        {item.discountPercent ? (
                          <span className="text-[10px] font-bold bg-gray-200 text-gray-600 px-1.5 py-0.5 rounded-full">
                            -{item.discountPercent}%
                          </span>
                        ) : null}
                      </div>
                      
                      <div className="flex items-center justify-between w-full">
                        <div className="text-sm text-gray-500 flex items-center gap-2 flex-wrap">
                          <span className="font-medium text-green-600">Paid x{paidQty}</span>
                          <span className="w-1 h-1 bg-gray-300 rounded-full" />
                          {hasDiscount ? (
                            <>
                              <span className="line-through text-xs text-gray-400">{formatCurrency(basePrice)}</span>
                              <span className="font-semibold text-gray-600">{formatCurrency(unitPricePaid)}</span>
                            </>
                          ) : (
                            <span className="font-semibold text-gray-600">{formatCurrency(unitPricePaid)}</span>
                          )}
                        </div>
                        {item.externalId && (
                          <div className="text-[10px] text-white bg-gray-900/85 font-bold font-mono px-1.5 py-0.5 rounded ml-2 backdrop-blur-[1px]">
                            {item.externalId}
                          </div>
                        )}
                      </div>
                    </div>

                    {/* Subtotal */}
                    <div className="text-right min-w-[90px]">
                      <div className="text-[10px] text-gray-400 font-bold uppercase tracking-wider mb-0.5">{t('pos.cart.subtotal')}</div>
                      <div className="font-bold text-lg text-gray-700">
                        {formatCurrency(Currency.mul(unitPricePaid, paidQty).toNumber())}
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      )}
    </div>
  );
};
