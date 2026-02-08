import React, { useState, useMemo } from 'react';
import { HeldOrder, Permission } from '@/core/domain/types';
import { Gift, ArrowLeft, Minus, Plus, Undo2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { compItem, uncompItem } from '@/core/stores/order/useOrderOperations';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { useProductStore } from '@/features/product';
import { useCategoryStore } from '@/features/category';
import { useImageUrls } from '@/core/hooks';
import DefaultImage from '@/assets/reshot.svg';

interface CompItemModeProps {
  order: HeldOrder;
  totalPaid: number;
  remaining: number;
  onBack: () => void;
  onManageTable?: () => void;
}

const PRESET_REASONS = [
  'customer_complaint',
  'item_defect',
  'promotion',
  'staff_error',
] as const;

export const CompItemMode: React.FC<CompItemModeProps> = ({
  order,
  totalPaid,
  remaining,
  onBack,
  onManageTable,
}) => {
  const { t } = useI18n();

  // Selection state
  const [selectedInstanceId, setSelectedInstanceId] = useState<string | null>(null);
  const [compQty, setCompQty] = useState(1);
  const [reason, setReason] = useState('');
  const [reasonKey, setReasonKey] = useState<string | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);

  // Product data for images
  const products = useProductStore((state) => state.items);
  const categories = useCategoryStore((state) => state.items);

  const productInfoMap = useMemo(() => {
    const map = new Map<string, { image?: string; category?: number }>();
    order.items.forEach(item => {
      const product = products.find(p => p.id === item.id);
      map.set(item.instance_id, {
        image: product?.image,
        category: product?.category_id ?? undefined,
      });
    });
    return map;
  }, [order.items, products]);

  const productImageRefs = useMemo(() => {
    return order.items.map(item => productInfoMap.get(item.instance_id)?.image);
  }, [order.items, productInfoMap]);
  const imageUrls = useImageUrls(productImageRefs);

  // Compable items: unpaid, not comped, not removed
  const compableItems = useMemo(() => {
    return order.items.filter(item => {
      if (item._removed || item.is_comped) return false;
      const paidQty = order.paid_item_quantities?.[item.instance_id] || 0;
      const unpaidQty = item.quantity - paidQty;
      return unpaidQty > 0;
    });
  }, [order.items, order.paid_item_quantities]);

  // Already comped items
  const compedItems = useMemo(() => {
    return order.items.filter(item => !item._removed && item.is_comped);
  }, [order.items]);

  // Group compable items by category
  const itemsByCategory = useMemo(() => {
    const groups = new Map<number, typeof compableItems>();
    const uncategorized: typeof compableItems = [];

    compableItems.forEach(item => {
      const info = productInfoMap.get(item.instance_id);
      const categoryRef = info?.category;
      if (categoryRef != null) {
        if (!groups.has(categoryRef)) groups.set(categoryRef, []);
        groups.get(categoryRef)!.push(item);
      } else {
        uncategorized.push(item);
      }
    });

    const result: Array<{ categoryName: string; items: typeof compableItems }> = [];
    groups.forEach((items, categoryRef) => {
      const category = categories.find(c => c.id === categoryRef);
      result.push({ categoryName: category?.name || t('common.label.unknown_item'), items });
    });
    result.sort((a, b) => a.categoryName.localeCompare(b.categoryName));
    if (uncategorized.length > 0) {
      result.push({ categoryName: t('common.label.unknown_item'), items: uncategorized });
    }
    return result;
  }, [compableItems, productInfoMap, categories, t]);

  const selectedItem = useMemo(() => {
    return order.items.find(i => i.instance_id === selectedInstanceId) ?? null;
  }, [order.items, selectedInstanceId]);

  const maxCompQty = useMemo(() => {
    if (!selectedItem) return 0;
    const paidQty = order.paid_item_quantities?.[selectedItem.instance_id] || 0;
    return selectedItem.quantity - paidQty;
  }, [selectedItem, order.paid_item_quantities]);

  // Reset compQty when selection changes
  const handleSelect = (instanceId: string) => {
    setSelectedInstanceId(instanceId);
    setCompQty(1);
    setReason('');
    setReasonKey(null);
  };

  const handlePresetReason = (key: string) => {
    setReasonKey(key);
    setReason('');
  };

  const effectiveReason = reasonKey || reason.trim();
  const canConfirm = effectiveReason.length > 0 && compQty > 0 && selectedItem && !isProcessing;

  const handleConfirmComp = async (authorizer: { id: number; name: string }) => {
    if (!selectedItem || !canConfirm) return;
    setIsProcessing(true);
    try {
      await compItem(
        order.order_id,
        selectedItem.instance_id,
        compQty,
        effectiveReason,
        authorizer,
      );
      toast.success(t('checkout.comp.badge'));
      setSelectedInstanceId(null);
      setReason('');
      setCompQty(1);
    } catch (err) {
      logger.error('Comp item failed', err);
      toast.error(String(err));
    } finally {
      setIsProcessing(false);
    }
  };

  const handleUncomp = async (instanceId: string, authorizer: { id: number; name: string }) => {
    setIsProcessing(true);
    try {
      await uncompItem(order.order_id, instanceId, authorizer);
      toast.success(t('checkout.comp.uncomp'));
    } catch (err) {
      logger.error('Uncomp item failed', err);
      toast.error(String(err));
    } finally {
      setIsProcessing(false);
    }
  };

  return (
    <div className="h-full flex bg-gray-50/50 backdrop-blur-xl">
      <OrderSidebar
        order={order}
        totalPaid={totalPaid}
        remaining={remaining}
        onManage={onManageTable}
      />

      <div className="flex-1 flex flex-col h-full overflow-hidden relative">
        {/* Background Decor */}
        <div className="absolute top-[-20%] left-[-10%] w-[600px] h-[600px] bg-emerald-100/50 rounded-full mix-blend-multiply filter blur-[100px] opacity-50 pointer-events-none" />
        <div className="absolute bottom-[-20%] right-[-10%] w-[600px] h-[600px] bg-green-100/50 rounded-full mix-blend-multiply filter blur-[100px] opacity-50 pointer-events-none" />

        {/* Header */}
        <div className="p-6 bg-white/80 backdrop-blur-md border-b border-gray-200/50 shadow-sm flex justify-between items-center z-10 shrink-0">
          <h3 className="font-bold text-gray-800 text-2xl flex items-center gap-3">
            <div className="p-2 bg-emerald-600 rounded-xl text-white shadow-lg shadow-emerald-500/30">
              <Gift size={24} />
            </div>
            {t('checkout.comp.title')}
          </h3>
          <button
            onClick={onBack}
            className="px-5 py-2.5 bg-white border border-gray-200 hover:bg-gray-50 hover:border-gray-300 text-gray-700 rounded-xl font-medium flex items-center gap-2 transition-all shadow-sm"
          >
            <ArrowLeft size={20} /> {t('common.action.back')}
          </button>
        </div>

        <div className="flex-1 flex overflow-hidden z-10">
          {/* Center: Item Selection */}
          <div className="flex-1 flex flex-col border-r border-gray-200/60 bg-white/50 backdrop-blur-sm min-w-0">
            <div className="flex-1 overflow-y-auto p-6 custom-scrollbar">
              {/* Compable Items */}
              {itemsByCategory.length === 0 && compedItems.length === 0 && (
                <div className="h-full flex flex-col items-center justify-center text-gray-400 space-y-4">
                  <Gift size={48} className="opacity-30" />
                  <p className="text-lg font-medium">{t('checkout.comp.no_items')}</p>
                </div>
              )}

              {itemsByCategory.map(({ categoryName, items }) => (
                <div key={categoryName} className="mb-8 last:mb-0">
                  <h4 className="text-sm font-bold text-gray-400 uppercase tracking-wider mb-4 ml-1">
                    {categoryName}
                  </h4>
                  <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 2xl:grid-cols-4 gap-4">
                    {items.map((item) => {
                      const paidQty = order.paid_item_quantities?.[item.instance_id] || 0;
                      const unpaidQty = item.quantity - paidQty;
                      const unitPrice = item.unit_price;
                      const imageRef = productInfoMap.get(item.instance_id)?.image;
                      const imageSrc = imageRef ? (imageUrls.get(imageRef) || DefaultImage) : DefaultImage;
                      const isSelected = selectedInstanceId === item.instance_id;

                      return (
                        <div
                          key={item.instance_id}
                          onClick={() => handleSelect(item.instance_id)}
                          className={`
                            relative group cursor-pointer rounded-2xl border transition-all duration-200 overflow-hidden
                            ${isSelected
                              ? 'border-emerald-500 ring-2 ring-emerald-500/20 bg-emerald-50/50'
                              : 'border-gray-200 bg-white hover:border-emerald-300 hover:shadow-lg hover:shadow-emerald-500/10'
                            }
                          `}
                        >
                          {isSelected && (
                            <div className="absolute top-3 right-3 z-10 bg-emerald-600 text-white w-8 h-8 flex items-center justify-center rounded-full font-bold text-sm shadow-lg animate-in zoom-in duration-200">
                              <Gift size={16} />
                            </div>
                          )}

                          <div className="p-3">
                            <div className="w-full aspect-square rounded-xl bg-gray-100 overflow-hidden relative mb-3">
                              <img src={imageSrc} alt={item.name} className="w-full h-full object-cover" onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }} />
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
                              <span className="text-xs text-gray-400">x{unpaidQty}</span>
                            </div>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              ))}

              {/* Already Comped Items */}
              {compedItems.length > 0 && (
                <div className="mb-8">
                  <h4 className="text-sm font-bold text-emerald-500 uppercase tracking-wider mb-4 ml-1 flex items-center gap-2">
                    <Gift size={14} />
                    {t('checkout.comp.badge')}
                  </h4>
                  <div className="grid grid-cols-2 lg:grid-cols-3 xl:grid-cols-3 2xl:grid-cols-4 gap-4">
                    {compedItems.map((item) => {
                      const imageRef = productInfoMap.get(item.instance_id)?.image;
                      const imageSrc = imageRef ? (imageUrls.get(imageRef) || DefaultImage) : DefaultImage;

                      return (
                        <div
                          key={item.instance_id}
                          className="relative rounded-2xl border border-emerald-200 bg-emerald-50/50 overflow-hidden"
                        >
                          <div className="absolute top-3 right-3 z-10 bg-emerald-100 text-emerald-700 px-2 py-0.5 rounded-full text-xs font-bold">
                            {t('checkout.comp.badge')}
                          </div>

                          <div className="p-3">
                            <div className="w-full aspect-square rounded-xl bg-gray-100 overflow-hidden relative mb-3 opacity-60">
                              <img src={imageSrc} alt={item.name} className="w-full h-full object-cover" onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }} />
                              <span className="absolute top-2 left-2 text-[0.6rem] text-blue-600 bg-white/90 backdrop-blur-sm font-bold font-mono px-1.5 py-0.5 rounded border border-blue-200/50">
                                #{item.instance_id.slice(-5)}
                              </span>
                            </div>
                            <div className="font-bold text-sm text-gray-600 leading-snug line-clamp-2" title={item.name}>
                              {item.name}
                            </div>
                            <div className="flex items-center justify-between mt-2">
                              <div>
                                <div className="text-sm text-emerald-600 font-medium">{formatCurrency(0)}</div>
                                {item.original_price != null && (
                                  <div className="text-xs text-gray-400 line-through">{formatCurrency(item.original_price)}</div>
                                )}
                              </div>
                              <EscalatableGate
                                permission={Permission.ORDERS_COMP}
                                mode="intercept"
                                description={t('checkout.comp.uncomp_auth_required')}
                                onAuthorized={(user) => handleUncomp(item.instance_id, { id: user.id, name: user.display_name })}
                              >
                                <button
                                  disabled={isProcessing}
                                  className="p-2 text-gray-400 hover:text-amber-600 hover:bg-amber-50 rounded-lg transition-colors disabled:opacity-50"
                                  title={t('checkout.comp.uncomp')}
                                >
                                  <Undo2 size={18} />
                                </button>
                              </EscalatableGate>
                            </div>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              )}
            </div>
          </div>

          {/* Right: Comp Details Panel */}
          <div className="w-[400px] flex flex-col bg-white border-l border-gray-200 shadow-xl z-20">
            <div className="p-6 bg-emerald-50 border-b border-emerald-100">
              <h4 className="font-bold text-gray-800 text-lg">{t('checkout.comp.title')}</h4>
              <div className="text-sm text-gray-500 mt-1">{t('checkout.comp.desc')}</div>
            </div>

            {!selectedItem ? (
              <div className="flex-1 flex flex-col items-center justify-center text-gray-400 space-y-4 p-6">
                <div className="w-16 h-16 rounded-full bg-gray-100 flex items-center justify-center">
                  <Gift size={32} className="opacity-50" />
                </div>
                <p className="text-sm font-medium">{t('checkout.comp.select_item')}</p>
              </div>
            ) : (
              <>
                <div className="flex-1 overflow-y-auto p-6 space-y-6">
                  {/* Selected Item Info */}
                  <div className="p-4 bg-gray-50 rounded-xl border border-gray-100">
                    <div className="font-bold text-gray-800 text-lg">{selectedItem.name}</div>
                    {selectedItem.selected_specification?.is_multi_spec && (
                      <div className="text-sm text-gray-500 mt-1">{t('pos.cart.spec')}: {selectedItem.selected_specification.name}</div>
                    )}
                    <div className="text-sm text-gray-500 mt-1">
                      {formatCurrency(selectedItem.unit_price)} x {maxCompQty}
                    </div>
                  </div>

                  {/* Quantity Selector */}
                  {maxCompQty > 1 && (
                    <div className="space-y-2">
                      <label className="text-sm font-bold text-gray-600">{t('checkout.comp.quantity_label')}</label>
                      <div className="flex items-center gap-4 justify-center">
                        <button
                          onClick={() => setCompQty(q => Math.max(1, q - 1))}
                          disabled={compQty <= 1}
                          className="w-12 h-12 rounded-xl bg-gray-100 hover:bg-gray-200 flex items-center justify-center disabled:opacity-30 text-gray-600 transition-colors"
                        >
                          <Minus size={20} />
                        </button>
                        <div className="text-3xl font-bold text-gray-800 w-16 text-center tabular-nums">
                          {compQty}
                        </div>
                        <button
                          onClick={() => setCompQty(q => Math.min(maxCompQty, q + 1))}
                          disabled={compQty >= maxCompQty}
                          className="w-12 h-12 rounded-xl bg-emerald-100 hover:bg-emerald-200 flex items-center justify-center disabled:opacity-30 text-emerald-700 transition-colors"
                        >
                          <Plus size={20} />
                        </button>
                      </div>
                      <div className="text-xs text-center text-gray-400">{compQty} / {maxCompQty}</div>
                    </div>
                  )}

                  {/* Preset Reasons */}
                  <div className="space-y-2">
                    <label className="text-sm font-bold text-gray-600">{t('checkout.comp.reason_label')}</label>
                    <div className="grid grid-cols-2 gap-2">
                      {PRESET_REASONS.map((key) => {
                        const isActive = reasonKey === key;
                        return (
                          <button
                            key={key}
                            onClick={() => handlePresetReason(key)}
                            className={`p-3 rounded-xl border-2 text-left transition-all text-sm font-medium ${
                              isActive
                                ? 'border-emerald-500 bg-emerald-50 text-emerald-700'
                                : 'border-gray-100 hover:border-emerald-200 hover:bg-gray-50 text-gray-600'
                            }`}
                          >
                            {t(`checkout.comp.preset.${key}`)}
                          </button>
                        );
                      })}
                    </div>
                    <textarea
                      value={reason}
                      onChange={(e) => { setReason(e.target.value); setReasonKey(null); }}
                      placeholder={reasonKey ? t(`checkout.comp.preset.${reasonKey}`) : t('checkout.comp.reason_placeholder')}
                      className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-emerald-500 focus:border-transparent resize-none text-sm"
                      rows={2}
                    />
                  </div>
                </div>

                {/* Confirm Button */}
                <div className="p-6 bg-white border-t border-gray-200 shadow-[0_-4px_20px_rgba(0,0,0,0.05)]">
                  <div className="flex justify-between items-end mb-4">
                    <span className="text-gray-500 font-medium">{t('checkout.comp.quantity_label')}</span>
                    <span className="text-2xl font-bold text-emerald-600 tabular-nums">
                      {compQty} x {formatCurrency(selectedItem.unit_price)}
                    </span>
                  </div>
                  <EscalatableGate
                    permission={Permission.ORDERS_COMP}
                    mode="intercept"
                    description={t('checkout.comp.auth_required')}
                    onAuthorized={(user) => handleConfirmComp({ id: user.id, name: user.display_name })}
                  >
                    <button
                      disabled={!canConfirm}
                      className="w-full py-4 bg-emerald-500 hover:bg-emerald-600 text-white rounded-xl font-bold text-lg shadow-lg shadow-emerald-500/30 hover:shadow-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                    >
                      <Gift size={20} />
                      {t('checkout.comp.confirm')}
                    </button>
                  </EscalatableGate>
                </div>
              </>
            )}
          </div>
        </div>
      </div>

    </div>
  );
};
