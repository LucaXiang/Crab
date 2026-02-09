import React, { useState, useMemo, useCallback } from 'react';
import { HeldOrder } from '@/core/domain/types';
import { ArrowLeft, Split, Minus, Plus, Banknote, ShoppingBag, CreditCard } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { useRetailServiceType, toBackendServiceType } from '@/core/stores/order/useCheckoutStore';
import { formatCurrency, Currency } from '@/utils/currency';
import { openCashDrawer } from '@/core/services/order/paymentService';
import { completeOrder, splitByItems } from '@/core/stores/order/commands';
import { CashPaymentModal } from './CashPaymentModal';
import { PaymentSuccessModal } from './PaymentSuccessModal';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { useProductStore } from '@/features/product';
import { useCategoryStore } from '@/features/category';
import { useImageUrls } from '@/core/hooks';
import DefaultImage from '@/assets/reshot.svg';

interface ItemSplitPageProps {
  order: HeldOrder;
  onBack: () => void;
  onComplete: () => void;
  onManageTable?: () => void;
}

export const ItemSplitPage: React.FC<ItemSplitPageProps> = ({ order, onBack, onComplete, onManageTable }) => {
  const { t } = useI18n();
  const serviceType = useRetailServiceType();

  const totalPaid = order.paid_amount;
  const remaining = order.remaining_amount;

  const [splitItems, setSplitItems] = useState<Record<string, number>>({});
  const [isProcessingSplit, setIsProcessingSplit] = useState(false);
  const [showCashModal, setShowCashModal] = useState(false);
  const [successModal, setSuccessModal] = useState<{
    isOpen: boolean;
    type: 'NORMAL' | 'CASH';
    change?: number;
    onClose: () => void;
    onPrint?: () => void;
    autoCloseDelay: number;
  } | null>(null);

  const handleComplete = useCallback(() => {
    requestAnimationFrame(() => {
      onComplete();
    });
  }, [onComplete]);

  const splitTotal = useMemo(() => {
    if (!order) return 0;
    let total = 0;
    (Object.entries(splitItems) as [string, number][]).forEach(([instanceId, qty]) => {
      const item = order.items.find(i => i.instance_id === instanceId);
      if (item) {
        total = Currency.add(total, Currency.mul(item.unit_price, qty)).toNumber();
      }
    });
    return total;
  }, [splitItems, order]);

  const products = useProductStore((state) => state.items);
  const categories = useCategoryStore((state) => state.items);

  const productInfoMap = useMemo(() => {
    const map = new Map<string, { image?: string; category?: string }>();
    order.items.forEach(item => {
      const product = products.find(p => p.id === item.id);
      map.set(item.instance_id, {
        image: product?.image,
        category: product?.category_id != null ? String(product.category_id) : undefined,
      });
    });
    return map;
  }, [order.items, products]);

  const productImageRefs = useMemo(() => {
    return order.items.map(item => productInfoMap.get(item.instance_id)?.image);
  }, [order.items, productInfoMap]);
  const imageUrls = useImageUrls(productImageRefs);

  const itemsByCategory = useMemo(() => {
    const groups = new Map<string, typeof order.items>();
    const uncategorized: typeof order.items = [];

    order.items.forEach(item => {
      if (item.is_comped) return;
      const paidQty = (order.paid_item_quantities && order.paid_item_quantities[item.instance_id]) || 0;
      if (item.quantity - paidQty <= 0) return;

      const info = productInfoMap.get(item.instance_id);
      const categoryRef = info?.category;

      if (categoryRef) {
        if (!groups.has(categoryRef)) {
          groups.set(categoryRef, []);
        }
        groups.get(categoryRef)!.push(item);
      } else {
        uncategorized.push(item);
      }
    });

    const result: Array<{ categoryId: string | null; categoryName: string; items: typeof order.items }> = [];

    groups.forEach((items, categoryRef) => {
      const category = categories.find(c => c.id === Number(categoryRef));
      result.push({
        categoryId: categoryRef,
        categoryName: category?.name || t('common.label.unknown_item'),
        items,
      });
    });

    const extIdMap = new Map(products.map(p => [p.id, p.external_id]));
    for (const group of result) {
      group.items.sort((a, b) => {
        const extA = extIdMap.get(a.id) ?? Number.MAX_SAFE_INTEGER;
        const extB = extIdMap.get(b.id) ?? Number.MAX_SAFE_INTEGER;
        if (extA !== extB) return extA - extB;
        return a.name.localeCompare(b.name);
      });
    }

    const categoryMap = new Map(categories.map(c => [c.id, c]));
    result.sort((a, b) => {
      const sortA = a.categoryId ? (categoryMap.get(Number(a.categoryId))?.sort_order ?? 0) : Number.MAX_SAFE_INTEGER;
      const sortB = b.categoryId ? (categoryMap.get(Number(b.categoryId))?.sort_order ?? 0) : Number.MAX_SAFE_INTEGER;
      return sortA - sortB;
    });

    if (uncategorized.length > 0) {
      result.push({
        categoryId: null,
        categoryName: t('common.label.unknown_item'),
        items: uncategorized,
      });
    }

    return result;
  }, [order.items, order.paid_item_quantities, productInfoMap, categories, products, t]);

  const [selectedCategory, setSelectedCategory] = useState<string | 'ALL'>('ALL');

  const allCategories = useMemo(() => {
    const cats = new Set<string>();
    itemsByCategory.forEach(g => cats.add(g.categoryName));
    return Array.from(cats);
  }, [itemsByCategory]);

  const filteredItemsByCategory = useMemo(() => {
    if (selectedCategory === 'ALL') return itemsByCategory;
    return itemsByCategory.filter(g => g.categoryName === selectedCategory);
  }, [itemsByCategory, selectedCategory]);

  const handleSplitPayment = useCallback(
    async (method: 'CASH' | 'CARD', cashDetails?: { tendered: number }) => {
      if (!order || isProcessingSplit) return false;

      const itemsToSplit = (Object.entries(splitItems) as [string, number][])
        .filter(([_, qty]) => qty > 0)
        .map(([instanceId, qty]) => {
          const originalItem = order.items.find((i) => i.instance_id === instanceId);
          return {
            instance_id: instanceId,
            quantity: qty,
            name: originalItem?.name || t('common.label.unknown_item'),
            price: originalItem?.price || 0,
            unit_price: originalItem?.unit_price ?? 0,
          };
        });

      if (itemsToSplit.length === 0) return false;

      setIsProcessingSplit(true);

      try {
        let total = 0;
        itemsToSplit.forEach((splitItem) => {
          total += splitItem.unit_price * splitItem.quantity;
        });

        if (method === 'CASH') {
          await openCashDrawer();
        }

        await splitByItems(
          order.order_id,
          itemsToSplit.map((i) => ({
            instance_id: i.instance_id,
            name: i.name,
            quantity: i.quantity,
            unit_price: i.unit_price,
          })),
          method,
          method === 'CASH' ? cashDetails?.tendered : undefined,
        );

        const willComplete = Currency.sub(remaining, total).toNumber() <= 0.01;

        if (willComplete) {
          await completeOrder(order.order_id, [], order.is_retail ? toBackendServiceType(serviceType) : null);
        }

        if (method === 'CASH' && cashDetails?.tendered !== undefined) {
          setSuccessModal({
            isOpen: true,
            type: 'CASH',
            change: Currency.sub(cashDetails.tendered, total).toNumber(),
            onClose: willComplete ? handleComplete : () => setSuccessModal(null),
            autoCloseDelay: willComplete && order.is_retail ? 0 : 10000,
          });
        } else if (willComplete) {
          setSuccessModal({
            isOpen: true,
            type: 'NORMAL',
            onClose: handleComplete,
            autoCloseDelay: order.is_retail ? 0 : 10000,
          });
        }

        if (!willComplete) {
          setSplitItems({});
        }
        return true;
      } catch (err) {
        logger.error('Split failed', err);
        toast.error(`${t('checkout.split.failed')}: ${err}`);
        return false;
      } finally {
        setIsProcessingSplit(false);
      }
    },
    [order, isProcessingSplit, splitItems, remaining, t, handleComplete, serviceType]
  );

  const handleConfirmSplitCash = useCallback(
    async (tenderedAmount: number) => {
      const success = await handleSplitPayment('CASH', { tendered: tenderedAmount });
      if (success) {
        setShowCashModal(false);
      }
    },
    [handleSplitPayment]
  );

  return (
    <>
      <div className="h-full flex bg-gray-50/50 backdrop-blur-xl">
        {successModal && (
          <PaymentSuccessModal
            isOpen={successModal.isOpen}
            type={successModal.type}
            change={successModal.change}
            onClose={successModal.onClose}
            autoCloseDelay={successModal.autoCloseDelay}
            onPrint={successModal.onPrint}
          />
        )}
        <OrderSidebar
          order={order}
          totalPaid={totalPaid}
          remaining={remaining}
          onManage={onManageTable}
        />

        <div className="flex-1 flex flex-col h-full overflow-hidden relative">
           {/* Background Decor */}
           <div className="absolute top-[-20%] left-[-10%] w-[600px] h-[600px] bg-indigo-100/50 rounded-full mix-blend-multiply filter blur-[100px] opacity-50 pointer-events-none"></div>
           <div className="absolute bottom-[-20%] right-[-10%] w-[600px] h-[600px] bg-blue-100/50 rounded-full mix-blend-multiply filter blur-[100px] opacity-50 pointer-events-none"></div>

          {/* Header */}
          <div className="p-6 bg-white/80 backdrop-blur-md border-b border-gray-200/50 shadow-sm flex justify-between items-center z-10 shrink-0">
            <h3 className="font-bold text-gray-800 text-2xl flex items-center gap-3">
              <div className="p-2 bg-indigo-500 rounded-xl text-white shadow-lg shadow-indigo-500/30">
                <Split size={24} />
              </div>
              {t('checkout.split.title')}
            </h3>
            <button onClick={() => { onBack(); }} className="px-5 py-2.5 bg-white border border-gray-200 hover:bg-gray-50 hover:border-gray-300 text-gray-700 rounded-xl font-medium flex items-center gap-2 transition-all shadow-sm">
              <ArrowLeft size={20} /> {t('common.action.back')}
            </button>
          </div>

          <div className="flex-1 flex overflow-hidden z-10">
              {/* Left Side: Item Selection */}
              <div className="flex-1 flex flex-col border-r border-gray-200/60 bg-white/50 backdrop-blur-sm min-w-0">
                  {/* Category Filter */}
                  <div className="p-4 overflow-x-auto whitespace-nowrap custom-scrollbar border-b border-gray-100">
                      <div className="flex gap-3">
                          <button
                            onClick={() => setSelectedCategory('ALL')}
                            className={`px-6 py-2.5 rounded-full text-sm font-bold transition-all ${
                                selectedCategory === 'ALL'
                                ? 'bg-gray-900 text-white shadow-lg shadow-gray-900/20'
                                : 'bg-white border border-gray-200 text-gray-600 hover:bg-gray-50'
                            }`}
                          >
                              {t('common.label.all')}
                          </button>
                          {allCategories.map(cat => (
                              <button
                                key={cat}
                                onClick={() => setSelectedCategory(cat)}
                                className={`px-6 py-2.5 rounded-full text-sm font-bold transition-all ${
                                    selectedCategory === cat
                                    ? 'bg-indigo-500 text-white shadow-lg shadow-indigo-500/20'
                                    : 'bg-white border border-gray-200 text-gray-600 hover:bg-gray-50'
                                }`}
                              >
                                  {cat}
                              </button>
                          ))}
                      </div>
                  </div>

                  {/* Items Grid */}
                  <div className="flex-1 overflow-y-auto p-6 custom-scrollbar">
                      {filteredItemsByCategory.map(({ categoryId, categoryName, items }) => (
                          <div key={categoryId || 'uncategorized'} className="mb-8 last:mb-0">
                              <h4 className="text-sm font-bold text-gray-400 uppercase tracking-wider mb-4 ml-1">{categoryName}</h4>
                              <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 2xl:grid-cols-4 gap-4">
                                  {items.map((item) => {
                                      const currentSplitQty = splitItems[item.instance_id] || 0;
                                      const paidQty = (order.paid_item_quantities && order.paid_item_quantities[item.instance_id]) || 0;
                                      const maxQty = item.quantity - paidQty;
                                      const unitPrice = item.unit_price;
                                      const imageRef = productInfoMap.get(item.instance_id)?.image;
                                      const imageSrc = imageRef ? (imageUrls.get(imageRef) || DefaultImage) : DefaultImage;
                                      const isSelected = currentSplitQty > 0;
                                      const isFullySelected = currentSplitQty === maxQty;

                                      return (
                                          <div
                                            key={item.instance_id}
                                            onClick={() => {
                                                if (currentSplitQty < maxQty) {
                                                    setSplitItems(prev => ({ ...prev, [item.instance_id]: (prev[item.instance_id] || 0) + 1 }));
                                                }
                                            }}
                                            className={`
                                                relative group cursor-pointer rounded-2xl border transition-all duration-200 overflow-hidden
                                                ${isSelected
                                                    ? 'border-indigo-500 ring-2 ring-indigo-500/20 bg-indigo-50/50'
                                                    : 'border-gray-200 bg-white hover:border-indigo-300 hover:shadow-lg hover:shadow-indigo-500/10'
                                                }
                                            `}
                                          >
                                              <div className="p-3">
                                                  <div className="w-full aspect-square rounded-xl bg-gray-100 overflow-hidden relative mb-3">
                                                      <img src={imageSrc} alt={item.name} className="w-full h-full object-cover" onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }} />
                                                      {isFullySelected && <div className="absolute inset-0 bg-black/40 flex items-center justify-center"><div className="text-white text-xs font-bold">{t('common.label.all')}</div></div>}
                                                      <span className="absolute top-2 left-2 text-[0.6rem] text-blue-600 bg-white/90 backdrop-blur-sm font-bold font-mono px-1.5 py-0.5 rounded border border-blue-200/50">
                                                        #{item.instance_id.slice(-5)}
                                                      </span>
                                                  </div>
                                                  <div className="font-bold text-sm text-gray-800 leading-snug line-clamp-2" title={item.name}>
                                                      {item.name}
                                                  </div>
                                                  {item.selected_specification?.is_multi_spec && (
                                                    <div className="text-xs text-gray-400 mt-0.5 truncate">{item.selected_specification.name}</div>
                                                  )}
                                                  <div className="flex items-center justify-between mt-2">
                                                      <span className="text-sm font-medium text-gray-500">{formatCurrency(unitPrice)}</span>
                                                      <span className="text-xs text-gray-400">
                                                          {t('checkout.split.remaining')} <span className="text-gray-700 font-bold">{maxQty - currentSplitQty}</span>/{maxQty}
                                                      </span>
                                                  </div>
                                              </div>

                                          </div>
                                      );
                                  })}
                              </div>
                          </div>
                      ))}
                  </div>
              </div>

              {/* Right Side: Summary & Pay */}
              <div className="w-[400px] flex flex-col bg-white border-l border-gray-200 shadow-xl z-20">
                  <div className="p-6 bg-gray-50 border-b border-gray-200">
                      <h4 className="font-bold text-gray-800 text-lg">{t('checkout.split.new_order')}</h4>
                      <div className="text-sm text-gray-500 mt-1">{Object.values(splitItems).reduce((a, b) => a + b, 0)} {t('checkout.split.available')}</div>
                  </div>

                  <div className="flex-1 overflow-y-auto p-4 custom-scrollbar">
                      {Object.keys(splitItems).length === 0 ? (
                          <div className="h-full flex flex-col items-center justify-center text-gray-400 space-y-4">
                              <div className="w-16 h-16 rounded-full bg-gray-100 flex items-center justify-center">
                                  <ShoppingBag size={32} className="opacity-50" />
                              </div>
                              <p className="text-sm font-medium">{t('checkout.split.desc')}</p>
                          </div>
                      ) : (
                          <div className="space-y-3">
                              {Object.entries(splitItems)
                                .filter(([, qty]) => qty > 0)
                                .sort(([idA], [idB]) => {
                                  const itemA = order.items.find(i => i.instance_id === idA);
                                  const itemB = order.items.find(i => i.instance_id === idB);
                                  if (!itemA || !itemB) return 0;
                                  const catA = categories.find(c => c.id === Number(productInfoMap.get(idA)?.category));
                                  const catB = categories.find(c => c.id === Number(productInfoMap.get(idB)?.category));
                                  const sortA = catA?.sort_order ?? 0;
                                  const sortB = catB?.sort_order ?? 0;
                                  if (sortA !== sortB) return sortA - sortB;
                                  const extA = products.find(p => p.id === itemA.id)?.external_id ?? Number.MAX_SAFE_INTEGER;
                                  const extB = products.find(p => p.id === itemB.id)?.external_id ?? Number.MAX_SAFE_INTEGER;
                                  if (extA !== extB) return extA - extB;
                                  return itemA.name.localeCompare(itemB.name);
                                })
                                .map(([instanceId, qty]) => {
                                  const item = order.items.find(i => i.instance_id === instanceId);
                                  if (!item) return null;
                                  const unitPrice = item.unit_price;

                                  return (
                                      <div key={instanceId} className="flex items-center gap-3 p-3 bg-white border border-gray-100 rounded-xl shadow-sm animate-in slide-in-from-right-4 duration-300">
                                          <div className="w-10 h-10 rounded-lg bg-gray-100 shrink-0 overflow-hidden">
                                              <img src={imageUrls.get(productInfoMap.get(instanceId)?.image ?? '') || DefaultImage} alt={item.name} className="w-full h-full object-cover" onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }} />
                                          </div>
                                          <div className="flex-1 min-w-0">
                                              <div className="text-sm font-bold text-gray-800 truncate">{item.name}</div>
                                              {item.selected_specification?.is_multi_spec && (
                                                <div className="text-xs text-gray-400">{t('pos.cart.spec')}: {item.selected_specification.name}</div>
                                              )}
                                              <div className="text-xs text-gray-500">{formatCurrency(unitPrice)}</div>
                                          </div>
                                          <div className="flex items-center gap-2">
                                              <button
                                                onClick={() => setSplitItems(prev => ({ ...prev, [instanceId]: Math.max(0, qty - 1) }))}
                                                className="w-7 h-7 flex items-center justify-center rounded-full bg-gray-100 hover:bg-gray-200 text-gray-600 transition-colors"
                                              >
                                                  <Minus size={14} />
                                              </button>
                                              <span className="text-sm font-bold w-4 text-center">{qty}</span>
                                              <button
                                                onClick={() => {
                                                    const paidQty = (order.paid_item_quantities && order.paid_item_quantities[instanceId]) || 0;
                                                    const maxQty = item.quantity - paidQty;
                                                    if (qty < maxQty) {
                                                        setSplitItems(prev => ({ ...prev, [instanceId]: qty + 1 }));
                                                    }
                                                }}
                                                className="w-7 h-7 flex items-center justify-center rounded-full bg-gray-100 hover:bg-gray-200 text-gray-600 transition-colors"
                                              >
                                                  <Plus size={14} />
                                              </button>
                                          </div>
                                      </div>
                                  );
                              })}
                          </div>
                      )}
                  </div>

                  <div className="p-6 bg-white border-t border-gray-200 shadow-[0_-4px_20px_rgba(0,0,0,0.05)]">
                      <div className="flex justify-between items-end mb-6">
                          <span className="text-gray-500 font-medium">{t('checkout.split.total')}</span>
                          <span className="text-3xl font-bold text-gray-900 tabular-nums">{formatCurrency(splitTotal)}</span>
                      </div>

                      <div className="grid grid-cols-2 gap-3">
                          <button
                              onClick={() => setShowCashModal(true)}
                              disabled={splitTotal <= 0 || isProcessingSplit}
                              className="py-4 bg-emerald-500 hover:bg-emerald-600 text-white rounded-xl font-bold text-lg shadow-lg shadow-emerald-500/30 hover:shadow-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                          >
                              <Banknote size={20} />
                              {t('checkout.split.pay_cash')}
                          </button>
                          <button
                              onClick={() => handleSplitPayment('CARD')}
                              disabled={splitTotal <= 0 || isProcessingSplit}
                              className="py-4 bg-blue-600 hover:bg-blue-700 text-white rounded-xl font-bold text-lg shadow-lg shadow-blue-600/30 hover:shadow-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                          >
                              <CreditCard size={20} />
                              {t('checkout.split.pay_card')}
                          </button>
                      </div>
                  </div>
              </div>
          </div>
        </div>
      </div>

      <CashPaymentModal
        isOpen={showCashModal}
        amountDue={splitTotal}
        isProcessing={isProcessingSplit}
        onConfirm={handleConfirmSplitCash}
        onCancel={() => setShowCashModal(false)}
      />
    </>
  );
};
