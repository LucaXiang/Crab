import React, { useState, useCallback, useMemo } from 'react';
import { HeldOrder, PaymentRecord } from '@/core/domain/types';
import { Coins, CreditCard, ArrowLeft, Printer, Trash2, Split, Minus, Plus, Banknote, Utensils, ShoppingBag, Receipt, ImageOff } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { Permission } from '@/core/domain/types';
import { useRetailServiceType, setRetailServiceType } from '@/core/stores/order/useCheckoutStore';
import { formatCurrency } from '@/utils/currency';
import { Currency } from '@/utils/currency';

// Services & Operations
import { openCashDrawer, printOrderReceipt } from '@/core/services/order/paymentService';
import { completeOrder, splitOrder, updateOrderInfo } from '@/core/stores/order/useOrderOperations';
import { useOrderCommands } from '@/core/stores/order/useOrderCommands';

// Components
import { CashPaymentModal } from './CashPaymentModal';
import { PaymentSuccessModal } from './PaymentSuccessModal';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { ConfirmDialog } from '@/shared/components';
import { SplitItemRow } from '../components';
import { useProductStore } from '@/features/product';
import { useCategoryStore } from '@/features/category';
import { useImageUrls } from '@/core/hooks';
import DefaultImage from '@/assets/reshot.svg';

interface PaymentFlowProps {
  order: HeldOrder;
  onComplete: () => void;
  onCancel?: () => void;
  onVoid?: () => void;
  onManageTable?: () => void;
}

type PaymentMode = 'SELECT' | 'ITEM_SPLIT' | 'PAYMENT_RECORDS';

export const PaymentFlow: React.FC<PaymentFlowProps> = ({ order, onComplete, onCancel, onVoid, onManageTable }) => {
  const { t } = useI18n();
  const serviceType = useRetailServiceType();
  const { cancelPayment } = useOrderCommands();

  // Calculate payment state from order (server state)
  const totalPaid = order.paid_amount;
  const remaining = Math.max(0, order.total - totalPaid);
  const isPaidInFull = remaining <= 0.01;

  // Get active (non-cancelled) payments
  const activePayments = useMemo(() => {
    return (order.payments || []).filter(p => !p.cancelled);
  }, [order.payments]);

  // Local State
  const [mode, setMode] = useState<PaymentMode>('SELECT');
  const [cancellingPaymentId, setCancellingPaymentId] = useState<string | null>(null);
  const [cancelConfirm, setCancelConfirm] = useState<{ paymentId: string; isSplit: boolean } | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);
  const [showCashModal, setShowCashModal] = useState(false);
  const [paymentContext, setPaymentContext] = useState<'FULL' | 'SPLIT'>('FULL');
  const [successModal, setSuccessModal] = useState<{
    isOpen: boolean;
    type: 'NORMAL' | 'CASH';
    change?: number;
    onClose: () => void;
    onPrint?: () => void;
    autoCloseDelay: number;
  } | null>(null);

  // Split Bill State
  const [splitItems, setSplitItems] = useState<Record<string, number>>({});
  const [isProcessingSplit, setIsProcessingSplit] = useState(false);

  // Retail order cancel confirmation
  const [showRetailCancelConfirm, setShowRetailCancelConfirm] = useState(false);

  // Defer completion to avoid "update during render" errors
  const handleComplete = useCallback(() => {
    requestAnimationFrame(() => {
      onComplete();
    });
  }, [onComplete]);

  /**
   * 处理返回按钮点击
   * 零售订单需要确认作废
   */
  const handleBackClick = useCallback(() => {
    if (order.is_retail) {
      setShowRetailCancelConfirm(true);
    } else {
      onCancel?.();
    }
  }, [order.is_retail, onCancel]);

  /**
   * 打开取消支付确认对话框
   */
  const handleCancelPayment = useCallback((paymentId: string, isSplit: boolean) => {
    setCancelConfirm({ paymentId, isSplit });
  }, []);

  /**
   * 确认取消支付记录
   */
  const handleConfirmCancelPayment = useCallback(async () => {
    if (!cancelConfirm) return;

    const { paymentId } = cancelConfirm;
    setCancelConfirm(null);
    setCancellingPaymentId(paymentId);

    try {
      const response = await cancelPayment(order.order_id, paymentId);
      if (response.success) {
        toast.success(t('checkout.payment.cancel_success'));
      } else {
        toast.error(response.error?.message || t('checkout.payment.cancel_failed'));
      }
    } catch (error) {
      console.error('Failed to cancel payment:', error);
      toast.error(t('checkout.payment.cancel_failed'));
    } finally {
      setCancellingPaymentId(null);
    }
  }, [cancelConfirm, order.order_id, cancelPayment, t]);

  /**
   * 处理现金全额支付
   */
  const handleFullCashPayment = useCallback(() => {
    setPaymentContext('FULL');
    setShowCashModal(true);
  }, []);

  /**
   * 确认现金支付
   */
  const handleConfirmFullCash = useCallback(
    async (tenderedAmount: number) => {
      if (Currency.lt(tenderedAmount, remaining)) {
        toast.error(t('settings.payment.amount_insufficient'));
        return;
      }

      setIsProcessing(true);
      try {
        // Open cash drawer
        await openCashDrawer();

        // Create payment record
        const payment: PaymentRecord = {
          payment_id: `pay-${Date.now()}`,
          method: 'CASH',
          amount: remaining,
          timestamp: Date.now(),
          tendered: tenderedAmount,
          change: Currency.sub(tenderedAmount, remaining).toNumber(),
        };

        // Complete order via backend (fire & forget)
        await completeOrder(order.order_id, order.receipt_number!, [payment]);
        const is_retail = order.is_retail;

        setShowCashModal(false);
        setSuccessModal({
          isOpen: true,
          type: 'CASH',
          change: payment.change,
          onClose: handleComplete,
          onPrint: is_retail ? async () => {
            await printOrderReceipt(order);
            toast.success(t('settings.payment.receipt_print_success'));
          } : undefined,
          autoCloseDelay: is_retail ? 0 : 10000,
        });
      } catch (error) {
        console.error('Cash payment failed:', error);
        toast.error(t('checkout.payment.failed'));
      } finally {
        setIsProcessing(false);
      }
    },
    [remaining, order, handleComplete, t]
  );

  /**
   * 处理刷卡全额支付
   */
  const handleFullCardPayment = useCallback(async () => {
    if (Currency.lte(remaining, 0)) {
      toast.error(t('settings.payment.amount_must_be_positive'));
      return;
    }

    setIsProcessing(true);
    try {
      const payment: PaymentRecord = {
        payment_id: `pay-${Date.now()}`,
        method: 'CARD',
        amount: remaining,
        timestamp: Date.now(),
      };

      // Complete order via backend (fire & forget)
      await completeOrder(order.order_id, order.receipt_number!, [payment]);
      const is_retail = order.is_retail;

      setSuccessModal({
        isOpen: true,
        type: 'NORMAL',
        onClose: handleComplete,
        onPrint: is_retail ? async () => {
          await printOrderReceipt(order);
          toast.success(t('settings.payment.receipt_print_success'));
        } : undefined,
        autoCloseDelay: is_retail ? 0 : 5000,
      });
    } catch (error) {
      console.error('Card payment failed:', error);
      toast.error(t('checkout.payment.failed'));
    } finally {
      setIsProcessing(false);
    }
  }, [remaining, order, handleComplete, t]);

  /**
   * 打印预付款收据
   */
  const handlePrintPrePayment = useCallback(async () => {
    try {
      // Receipt number is already set by server at OpenTable time
      if (!order.receipt_number) {
        toast.error('Order has no receipt number');
        return;
      }

      await updateOrderInfo(order.order_id, {
        is_pre_payment: true,
        receipt_number: order.receipt_number,
      });

      // Print with current order (WebSocket will update if needed)
      const orderToPrint = { ...order, is_pre_payment: true };
      await printOrderReceipt(orderToPrint);
      toast.success(t('settings.payment.receipt_print_success'));
    } catch (error) {
      console.error('Pre-payment print failed:', error);
      toast.error(t('settings.payment.receipt_print_failed'));
    }
  }, [order, t]);

  /**
   * 分账支付处理
   * Fire & forget - UI updates via WebSocket
   */
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
            unit_price: originalItem?.unit_price ?? originalItem?.price ?? 0,
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

        // Server calculates amount from items (server-authoritative)
        // Fire & forget - UI updates via WebSocket
        await splitOrder(order.order_id, {
          items: itemsToSplit.map((i) => ({
            instance_id: i.instance_id,
            name: i.name,
            quantity: i.quantity,
            unit_price: i.unit_price,
          })),
          paymentMethod: method,
          tendered: cashDetails?.tendered,
          change: cashDetails ? cashDetails.tendered - total : undefined,
        });

        // Show success modal for cash payments
        if (method === 'CASH' && cashDetails?.tendered !== undefined) {
          setSuccessModal({
            isOpen: true,
            type: 'CASH',
            change: cashDetails.tendered - total,
            onClose: () => setSuccessModal(null),
            autoCloseDelay: 10000,
          });
        }

        setMode('ITEM_SPLIT');
        setSplitItems({});
        return true;
      } catch (err) {
        console.error('Split failed:', err);
        toast.error(`${t('checkout.split.failed')}: ${err}`);
        return false;
      } finally {
        setIsProcessingSplit(false);
      }
    },
    [order, isProcessingSplit, splitItems, t]
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

  const handleCashModalConfirm = useCallback(
    (tenderedAmount: number) => {
      if (paymentContext === 'FULL') {
        handleConfirmFullCash(tenderedAmount);
      } else {
        handleConfirmSplitCash(tenderedAmount);
      }
    },
    [paymentContext, handleConfirmFullCash, handleConfirmSplitCash]
  );

  const splitTotal = useMemo(() => {
    if (!order) return 0;
    let total = 0;
    (Object.entries(splitItems) as [string, number][]).forEach(([instanceId, qty]) => {
      const item = order.items.find(i => i.instance_id === instanceId);
      if (item) {
        // Use server-authoritative unit_price
        const unitPrice = item.unit_price ?? item.price;
        total += unitPrice * qty;
      }
    });
    return total;
  }, [splitItems, order]);

  // Get product and category info for split mode
  const products = useProductStore((state) => state.items);
  const categories = useCategoryStore((state) => state.items);

  // Build product info map (image + category)
  const productInfoMap = useMemo(() => {
    const map = new Map<string, { image?: string; category?: string }>();
    order.items.forEach(item => {
      const product = products.find(p => p.id === item.id);
      map.set(item.instance_id, {
        image: product?.image,
        category: product?.category,
      });
    });
    return map;
  }, [order.items, products]);

  // Get all image refs for batch loading
  const productImageRefs = useMemo(() => {
    return order.items.map(item => productInfoMap.get(item.instance_id)?.image);
  }, [order.items, productInfoMap]);
  const imageUrls = useImageUrls(productImageRefs);

  // Group items by category for split mode (only unpaid items)
  const itemsByCategory = useMemo(() => {
    const groups = new Map<string, typeof order.items>();
    const uncategorized: typeof order.items = [];

    order.items.forEach(item => {
      // Skip fully paid items
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

    // Convert to array with category info
    const result: Array<{ categoryId: string | null; categoryName: string; items: typeof order.items }> = [];

    groups.forEach((items, categoryRef) => {
      const category = categories.find(c => c.id === categoryRef);
      result.push({
        categoryId: categoryRef,
        categoryName: category?.name || t('common.label.unknown_item'),
        items,
      });
    });

    // Sort by category name
    result.sort((a, b) => a.categoryName.localeCompare(b.categoryName));

    // Add uncategorized at the end
    if (uncategorized.length > 0) {
      result.push({
        categoryId: null,
        categoryName: t('common.label.unknown_item'),
        items: uncategorized,
      });
    }

    return result;
  }, [order.items, order.paid_item_quantities, productInfoMap, categories, t]);

  const renderSplitMode = () => {
    return (
      <div className="h-full flex">
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
        <div className="flex-1 flex flex-col bg-gray-50">
          <div className="p-6 bg-white border-b border-gray-200 shadow-sm flex justify-between items-center">
            <h3 className="font-bold text-gray-800 text-xl flex items-center gap-2">
              <Split size={24} className="text-purple-600" />
              {t('checkout.split.title')}
            </h3>
            <button onClick={() => { setMode('SELECT'); setSplitItems({}); }} className="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium flex items-center gap-2 transition-all">
              <ArrowLeft size={20} /> {t('common.action.back')}
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-6 custom-scrollbar">
            <div className="space-y-6">
              {itemsByCategory.map(({ categoryId, categoryName, items }) => (
                <div key={categoryId || 'uncategorized'}>
                  {/* Category Header */}
                  <div className="sticky top-0 bg-gray-50 py-2 z-10">
                    <h4 className="text-lg font-bold text-gray-700 border-b-2 border-purple-200 pb-2">
                      {categoryName}
                      <span className="ml-2 text-sm font-normal text-gray-400">({items.length})</span>
                    </h4>
                  </div>

                  {/* Items Grid */}
                  <div className="grid grid-cols-3 lg:grid-cols-4 xl:grid-cols-5 gap-3 mt-3">
                    {items.map((item) => {
                      const currentSplitQty = splitItems[item.instance_id] || 0;
                      const paidQty = (order.paid_item_quantities && order.paid_item_quantities[item.instance_id]) || 0;
                      const maxQty = item.quantity - paidQty;
                      const unitPrice = item.unit_price ?? item.price;
                      const imageRef = productInfoMap.get(item.instance_id)?.image;
                      const imageSrc = imageRef ? (imageUrls.get(imageRef) || DefaultImage) : DefaultImage;

                      return (
                        <div
                          key={item.instance_id}
                          className="bg-white rounded-lg border border-gray-200 overflow-hidden"
                        >
                          {/* Image */}
                          <div className="aspect-[4/3] bg-gray-100">
                            {imageRef ? (
                              <img
                                src={imageSrc}
                                alt={item.name}
                                className="w-full h-full object-cover"
                                onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }}
                              />
                            ) : (
                              <div className="w-full h-full flex items-center justify-center text-gray-300">
                                <ImageOff size={32} />
                              </div>
                            )}
                          </div>

                          {/* Info */}
                          <div className="p-2">
                            <div className="text-sm font-medium text-gray-800 truncate" title={item.name}>
                              {item.name}
                            </div>
                            <div className="flex items-center justify-between text-xs mt-0.5">
                              <span className="text-gray-600">{formatCurrency(unitPrice)}</span>
                              <span className="text-gray-400">剩{maxQty}</span>
                            </div>
                          </div>

                          {/* Quantity Controls */}
                          <div className="flex items-center justify-between px-2 pb-2">
                            <button
                              onClick={() => setSplitItems(prev => ({ ...prev, [item.instance_id]: Math.max(0, (prev[item.instance_id] || 0) - 1) }))}
                              disabled={currentSplitQty <= 0}
                              className="w-7 h-7 flex items-center justify-center rounded bg-gray-100 text-gray-600 hover:bg-gray-200 disabled:opacity-30"
                            >
                              <Minus size={14} />
                            </button>
                            <span className={`text-sm font-bold ${currentSplitQty > 0 ? 'text-blue-600' : 'text-gray-400'}`}>
                              {currentSplitQty}
                            </span>
                            <button
                              onClick={() => setSplitItems(prev => ({ ...prev, [item.instance_id]: Math.min(maxQty, (prev[item.instance_id] || 0) + 1) }))}
                              disabled={currentSplitQty >= maxQty || maxQty === 0}
                              className="w-7 h-7 flex items-center justify-center rounded bg-gray-100 text-gray-600 hover:bg-gray-200 disabled:opacity-30"
                            >
                              <Plus size={14} />
                            </button>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          </div>

          <div className="p-6 bg-white border-t border-gray-200 shadow-[0_-4px_6px_-1px_rgba(0,0,0,0.05)]">
            <div className="max-w-3xl mx-auto space-y-4">
              <div className="flex justify-between items-center">
                <span className="text-gray-500 font-medium text-lg">{t('checkout.split.total')}</span>
                <span className="text-3xl font-bold text-gray-900">{formatCurrency(splitTotal)}</span>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <button
                  onClick={() => {
                    setPaymentContext('SPLIT');
                    setShowCashModal(true);
                  }}
                  disabled={splitTotal <= 0 || isProcessingSplit}
                  className="flex items-center justify-center gap-3 py-4 bg-green-600 text-white rounded-xl font-bold text-lg hover:bg-green-700 hover:shadow-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.99]"
                >
                  <Banknote size={24} />
                  {t('checkout.split.pay_cash')}
                </button>
                <button
                  onClick={() => handleSplitPayment('CARD')}
                  disabled={splitTotal <= 0 || isProcessingSplit}
                  className="flex items-center justify-center gap-3 py-4 bg-blue-600 text-white rounded-xl font-bold text-lg hover:bg-blue-700 hover:shadow-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.99]"
                >
                  <CreditCard size={24} />
                  {t('checkout.split.pay_card')}
                </button>
              </div>
            </div>
          </div>
        </div>
      </div>
    );
  };

  const renderSelectMode = () => {
    return (
      <div className="h-full flex">
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
        <div className="flex-1 flex flex-col bg-gray-50">
          <div className="p-6 bg-white border-b border-gray-200 shadow-sm">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-2xl font-bold text-gray-800">{t('checkout.payment.method')}</h2>
              <div className="flex gap-2 items-center">
                {order.is_retail && (
                  <div className="flex bg-gray-100 p-1 rounded-lg h-[2.5rem] items-center mr-2">
                    <button
                      onClick={() => setRetailServiceType('dineIn')}
                      className={`
                        flex items-center gap-2 px-3 h-full rounded-md text-sm font-medium transition-all
                        ${serviceType === 'dineIn'
                          ? 'bg-white text-gray-900 shadow-sm ring-1 ring-black/5'
                          : 'text-gray-500 hover:text-gray-700'}
                      `}
                    >
                      <Utensils size={16} />
                      {t('checkout.order_type.dine_in')}
                    </button>
                    <button
                      onClick={() => setRetailServiceType('takeout')}
                      className={`
                        flex items-center gap-2 px-3 h-full rounded-md text-sm font-medium transition-all
                        ${serviceType === 'takeout'
                          ? 'bg-white text-gray-900 shadow-sm ring-1 ring-black/5'
                          : 'text-gray-500 hover:text-gray-700'}
                      `}
                    >
                      <ShoppingBag size={16} />
                      {t('checkout.order_type.takeout')}
                    </button>
                  </div>
                )}
                {!order.is_retail && (
                  <button onClick={handlePrintPrePayment} className="px-4 py-2 bg-blue-100 hover:bg-blue-200 text-blue-700 rounded-lg font-medium transition-colors flex items-center gap-2">
                    <Printer size={20} />
                    {t('checkout.pre_payment.receipt')}
                  </button>
                )}
                {onVoid && !order.is_retail && (
                  <EscalatableGate
                    permission={Permission.VOID_ORDER}
                    mode="intercept"
                    description={t('checkout.void.title')}
                    onAuthorized={() => {
                      if (onVoid) onVoid();
                    }}
                  >
                    <button onClick={onVoid} className="px-4 py-2 bg-red-100 hover:bg-red-200 text-red-700 rounded-lg font-medium transition-colors flex items-center gap-2">
                      <Trash2 size={20} />
                      {t('checkout.void.title')}
                    </button>
                  </EscalatableGate>
                )}
                {onCancel && (
                  <button onClick={handleBackClick} className="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium transition-colors flex items-center gap-2">
                    <ArrowLeft size={20} />
                    {t('common.action.back')}
                  </button>
                )}
              </div>
            </div>
            {/* Summary Grid */}
            <div className="grid grid-cols-3 gap-4">
              <div className="p-4 bg-gray-50 rounded-xl">
                <div className="text-xs text-gray-500 uppercase font-bold">{t('checkout.amount.total')}</div>
                <div className="text-2xl font-bold text-gray-900 mt-1">{formatCurrency(order.total)}</div>
              </div>
              <div className="p-4 bg-blue-50 rounded-xl">
                <div className="text-xs text-gray-600 uppercase font-bold">{t('checkout.amount.paid')}</div>
                <div className="text-2xl font-bold text-blue-600 mt-1">{formatCurrency(totalPaid)}</div>
              </div>
              <div className={`p-4 rounded-xl ${isPaidInFull ? 'bg-green-50' : 'bg-red-50'}`}>
                <div className="text-xs text-gray-600 uppercase font-bold">{t('checkout.amount.remaining')}</div>
                <div className={`text-2xl font-bold mt-1 ${isPaidInFull ? 'text-green-600' : 'text-red-600'}`}>{formatCurrency(remaining)}</div>
              </div>
            </div>

          </div>

          <div className="flex-1 p-8 overflow-y-auto space-y-6">
            {/* Payment Method Buttons */}
            <div className="grid grid-cols-3 gap-6">
              <button onClick={handleFullCashPayment} disabled={isPaidInFull || isProcessing} className="h-40 bg-gradient-to-br from-green-500 to-green-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <Coins size={48} />
                <div className="text-2xl font-bold">{t('checkout.method.cash')}</div>
                <div className="text-sm opacity-90">{t('checkout.method.cash_desc')}</div>
              </button>
              <button onClick={handleFullCardPayment} disabled={isPaidInFull || isProcessing} className="h-40 bg-gradient-to-br from-blue-500 to-blue-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <CreditCard size={48} />
                <div className="text-2xl font-bold">{t('checkout.method.card')}</div>
                <div className="text-sm opacity-90">{t('checkout.method.card_desc')}</div>
              </button>
              <button onClick={() => setMode('ITEM_SPLIT')} disabled={isPaidInFull || isProcessing} className="h-40 bg-gradient-to-br from-purple-500 to-purple-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <Split size={48} />
                <div className="text-2xl font-bold">{t('checkout.split.title')}</div>
                <div className="text-sm opacity-90">{t('checkout.split.desc')}</div>
              </button>
            </div>

            {/* Payment Records Entry Card */}
            {activePayments.length > 0 && (
              <div className="grid grid-cols-3 gap-6">
                <button
                  onClick={() => setMode('PAYMENT_RECORDS')}
                  className="h-40 bg-gradient-to-br from-amber-500 to-orange-500 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all flex flex-col items-center justify-center gap-4"
                >
                  <Receipt size={48} />
                  <div className="text-2xl font-bold">{t('checkout.payment.records')}</div>
                  <div className="text-sm opacity-90">{activePayments.length} {t('checkout.payment.record_count')} · {formatCurrency(totalPaid)}</div>
                </button>
              </div>
            )}
          </div>
        </div>
      </div>
    );
  };

  const renderPaymentRecordsMode = () => {
    return (
      <div className="h-full flex">
        <OrderSidebar
          order={order}
          totalPaid={totalPaid}
          remaining={remaining}
          onManage={onManageTable}
        />
        <div className="flex-1 flex flex-col bg-gray-50">
          {/* Header */}
          <div className="p-6 bg-white border-b border-gray-200 shadow-sm flex justify-between items-center">
            <h3 className="font-bold text-gray-800 text-xl flex items-center gap-2">
              <Receipt size={24} className="text-amber-600" />
              {t('checkout.payment.records')}
            </h3>
            <button
              onClick={() => setMode('SELECT')}
              className="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium flex items-center gap-2 transition-all"
            >
              <ArrowLeft size={20} /> {t('common.action.back')}
            </button>
          </div>

          {/* Payment Records List */}
          <div className="flex-1 overflow-y-auto p-6 custom-scrollbar">
            <div className="space-y-4 max-w-3xl mx-auto">
              {activePayments.map((payment) => {
                const isSplit = payment.payment_id.startsWith('split-');
                const isCash = /cash/i.test(payment.method);
                const isCancelling = cancellingPaymentId === payment.payment_id;

                return (
                  <div
                    key={payment.payment_id}
                    className="bg-white p-5 rounded-xl border border-gray-200 shadow-sm"
                  >
                    {/* Header Row */}
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-4">
                        <div className={`p-3 rounded-xl ${isCash ? 'bg-green-100 text-green-600' : 'bg-blue-100 text-blue-600'}`}>
                          {isCash ? <Coins size={24} /> : <CreditCard size={24} />}
                        </div>
                        <div>
                          <div className="font-bold text-gray-800 text-lg flex items-center gap-2">
                            {isCash ? t('checkout.method.cash') : payment.method}
                            {isSplit && (
                              <span className="text-xs bg-purple-100 text-purple-600 px-2 py-0.5 rounded-full font-medium">
                                {t('checkout.split.label')}
                              </span>
                            )}
                          </div>
                          <div className="text-sm text-gray-400 mt-0.5">
                            {new Date(payment.timestamp).toLocaleString([], {
                              month: 'short',
                              day: 'numeric',
                              hour: '2-digit',
                              minute: '2-digit',
                              hour12: false,
                            })}
                          </div>
                        </div>
                      </div>

                      <div className="flex items-center gap-6">
                        <div className="text-right">
                          <div className="font-bold text-gray-800 text-2xl">
                            {formatCurrency(payment.amount)}
                          </div>
                          {isCash && payment.tendered != null && payment.tendered > payment.amount && (
                            <div className="text-sm text-gray-400 mt-0.5">
                              {t('checkout.payment.tendered')}: {formatCurrency(payment.tendered)} → {t('checkout.payment.change')}: {formatCurrency(payment.change ?? 0)}
                            </div>
                          )}
                        </div>
                        <button
                          onClick={() => handleCancelPayment(payment.payment_id, isSplit)}
                          disabled={isCancelling}
                          className="p-3 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-xl transition-colors disabled:opacity-50"
                          title={t('checkout.payment.cancel')}
                        >
                          {isCancelling ? (
                            <div className="w-5 h-5 border-2 border-gray-300 border-t-gray-600 rounded-full animate-spin" />
                          ) : (
                            <Trash2 size={20} />
                          )}
                        </button>
                      </div>
                    </div>

                    {/* Split Items Detail */}
                    {payment.split_items && payment.split_items.length > 0 && (
                      <div className="mt-4 pt-4 border-t border-gray-100">
                        <div className="text-xs text-gray-400 uppercase font-bold mb-2">{t('checkout.payment.split_items')}</div>
                        <div className="divide-y divide-gray-50">
                          {payment.split_items.map((item, idx) => (
                            <SplitItemRow key={idx} item={item} />
                          ))}
                        </div>
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* Footer Summary */}
          <div className="p-6 bg-white border-t border-gray-200 shadow-[0_-4px_6px_-1px_rgba(0,0,0,0.05)]">
            <div className="max-w-3xl mx-auto flex justify-between items-center">
              <span className="text-gray-500 font-medium text-lg">{t('checkout.payment.total_paid')}</span>
              <span className="text-3xl font-bold text-blue-600">{formatCurrency(totalPaid)}</span>
            </div>
          </div>
        </div>
      </div>
    );
  };

  const renderContent = () => {
    switch (mode) {
      case 'SELECT':
        return renderSelectMode();
      case 'ITEM_SPLIT':
        return renderSplitMode();
      case 'PAYMENT_RECORDS':
        return renderPaymentRecordsMode();
      default:
        return renderSelectMode();
    }
  };

  return (
    <>
      <div className="h-full">{renderContent()}</div>

      {/* Cancel Payment Confirmation Dialog */}
      <ConfirmDialog
        isOpen={cancelConfirm !== null}
        title={t('checkout.payment.cancel')}
        description={cancelConfirm?.isSplit ? t('checkout.payment.confirm_cancel_split') : t('checkout.payment.confirm_cancel')}
        confirmText={t('checkout.payment.cancel')}
        onConfirm={handleConfirmCancelPayment}
        onCancel={() => setCancelConfirm(null)}
        variant="danger"
      />

      {/* Retail Order Cancel Confirmation Dialog */}
      <ConfirmDialog
        isOpen={showRetailCancelConfirm}
        title={t('checkout.retail.cancel_title')}
        description={t('checkout.retail.cancel_description')}
        confirmText={t('checkout.retail.cancel_confirm')}
        onConfirm={() => {
          setShowRetailCancelConfirm(false);
          onCancel?.();
        }}
        onCancel={() => setShowRetailCancelConfirm(false)}
        variant="danger"
      />

      <CashPaymentModal
        isOpen={showCashModal}
        amountDue={paymentContext === 'SPLIT' ? splitTotal : remaining}
        isProcessing={isProcessing || isProcessingSplit}
        onConfirm={handleCashModalConfirm}
        onCancel={() => setShowCashModal(false)}
      />
    </>
  );
};
