import React, { useState, useCallback, useEffect, useMemo } from 'react';
import { HeldOrder, PaymentRecord } from '@/core/domain/types';
import { Coins, CreditCard, ArrowLeft, Printer, Trash2, Split, Minus, Plus, Banknote, Utensils, ShoppingBag } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { Permission } from '@/core/domain/types';
import { useRetailServiceType, setRetailServiceType } from '@/core/stores/order/useCheckoutStore';
import { formatCurrency } from '@/utils/currency';

// Stores & Services
import {
  usePaymentActions,
  usePaymentSession,
  usePaymentTotals,
} from '@/core/stores/order/usePaymentStore';
import {
  processCashPayment,
  processCardPayment,
  validatePaymentAmount,
  printOrderReceipt,
} from '@/core/services/order/paymentService';
import { completeOrder, ensureActiveOrder } from '@/core/stores/order/useOrderOperations';
import { useOrderEventStore } from '@/core/stores/order/useOrderEventStore';
import { useReceiptStore } from '@/core/stores/order/useReceiptStore';

// Components
import { CashPaymentModal } from './CashPaymentModal';
import { PaymentSuccessModal } from './PaymentSuccessModal';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';

interface PaymentFlowProps {
  order: HeldOrder;
  onComplete: () => void;
  onCancel?: () => void;
  onUpdateOrder?: (order: HeldOrder) => void;
  onVoid?: () => void;
  onManageTable?: () => void;
}

type PaymentMode = 'SELECT' | 'CASH' | 'CARD' | 'ITEM_SPLIT' | 'AMOUNT_SPLIT';

export const PaymentFlow: React.FC<PaymentFlowProps> = ({ order, onComplete, onCancel, onUpdateOrder, onVoid, onManageTable }) => {
  const { t } = useI18n();
  const serviceType = useRetailServiceType();
  const orderKey = order.key;

  // Payment Store
  const paymentActions = usePaymentActions();
  const session = usePaymentSession(orderKey);
  // Calculate totals including previous split payments (stored in order.paidAmount)
  const { totalPaid: sessionPaid } = usePaymentTotals(orderKey, order.total);

  const totalPaid = sessionPaid + (order.paidAmount || 0);
  // Calculate remaining manually to ensure we account for split payments
  const remaining = Math.max(0, order.total - totalPaid);
  const isPaidInFull = remaining <= 0.01;

  // Local State
  const [mode, setMode] = useState<PaymentMode>('SELECT');
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

  // 初始化支付会话
  useEffect(() => {
    if (!session) {
      paymentActions.initSession(orderKey);
    }
  }, [orderKey, session, paymentActions]);

  // Defer completion to avoid "update during render" errors
  const handleComplete = useCallback(() => {
    requestAnimationFrame(() => {
      onComplete();
    });
  }, [onComplete]);

  // Unified Payment Success Handler
  const handlePaymentSuccess = useCallback(async (payment: PaymentRecord, context?: { isCash: boolean, tendered?: number }) => {
    paymentActions.addPayment(orderKey, payment);

    // Check for completion
    const newTotalPaid = totalPaid + payment.amount;
    const validation = validatePaymentAmount(newTotalPaid, order.total);

    if (validation.isValid) {
      try {
        const completed = completeOrder(order, [payment]);
        const isRetail = order.isRetail;

        if (context?.isCash && context.tendered !== undefined) {
          setSuccessModal({
            isOpen: true,
            type: 'CASH',
            change: context.tendered - payment.amount,
            onClose: handleComplete,
            onPrint: isRetail ? async () => {
              await printOrderReceipt(completed);
              toast.success(t('settings.payment.receiptPrintSuccess'));
            } : undefined,
            autoCloseDelay: isRetail ? 0 : 10000 // 10s for cash, 0 for retail manual close
          });
        } else {
          setSuccessModal({
            isOpen: true,
            type: 'NORMAL',
            onClose: handleComplete,
            onPrint: isRetail ? async () => {
              await printOrderReceipt(completed);
              toast.success(t('settings.payment.receiptPrintSuccess'));
            } : undefined,
            autoCloseDelay: isRetail ? 0 : 5000 // 5s for normal, 0 for retail manual close
          });
        }
      } catch (error) {
        console.error('Order completion failed:', error);
        toast.error(t('checkout.error.completionFailed'));
      }
    } else {
      // Partial payment
      // Ensure order exists so payment is attached to something real
      ensureActiveOrder(order);

      if (context?.isCash && context.tendered !== undefined) {
        // Partial cash payment (e.g. split) - still show change if desired? 
        // Assuming user wants to see change for any cash transaction.
        // But usually partial payment keeps the flow open.
        // We can show modal and then just close modal (stay in flow).
        setSuccessModal({
          isOpen: true,
          type: 'CASH',
          change: context.tendered - payment.amount,
          onClose: () => setSuccessModal(null),
          autoCloseDelay: 10000
        });
      } else {
        toast.success(t('checkout.payment.partial'));
      }
    }
  }, [order, totalPaid, paymentActions, orderKey, handleComplete, t]);

  /**
   * 处理现金支付（全额）
   */
  const handleFullCashPayment = useCallback(async () => {
    setPaymentContext('FULL');
    setShowCashModal(true);
  }, []);

  /**
   * 确认现金支付（全额）
   */
  const handleConfirmFullCash = useCallback(
    async (tenderedAmount: number) => {
      setIsProcessing(true);
      try {
        const payment = await processCashPayment({
          amount: remaining,
          tenderedAmount,
        });

        await handlePaymentSuccess(payment, { isCash: true, tendered: tenderedAmount });
        setShowCashModal(false);
      } catch (error) {
        console.error('Cash payment failed:', error);
        const errorKey = error instanceof Error ? error.message : '';
        if (errorKey === 'PAYMENT_AMOUNT_INSUFFICIENT') {
          toast.error(t('settings.payment.amountInsufficient'));
        } else if (errorKey === 'RECEIPT_PRINT_FAILED') {
          toast.error(t('settings.payment.receiptPrintFailed'));
        } else {
          toast.error(t('checkout.payment.failed'));
        }
      } finally {
        setIsProcessing(false);
      }
    },
    [remaining, handlePaymentSuccess, t]
  );

  /**
   * 处理刷卡支付（全额）
   */
  const handleFullCardPayment = useCallback(async () => {
    setIsProcessing(true);
    try {
      const payment = await processCardPayment({ amount: remaining });
      await handlePaymentSuccess(payment, { isCash: false });
    } catch (error) {
      console.error('Card payment failed:', error);
      const errorKey = error instanceof Error ? error.message : '';
      if (errorKey === 'PAYMENT_AMOUNT_MUST_BE_POSITIVE') {
        toast.error(t('settings.payment.amountMustBePositive'));
      } else {
        toast.error(t('checkout.payment.failed'));
      }
    } finally {
      setIsProcessing(false);
    }
  }, [remaining, handlePaymentSuccess, t]);


  const handlePrintPrePayment = useCallback(async () => {
    try {
      const store = useOrderEventStore.getState();
      const existingOrder = store.getOrder(order.key);
      let currentOrder = existingOrder ? { ...existingOrder } : { ...order };

      if (!currentOrder.receiptNumber || !currentOrder.receiptNumber.startsWith('FAC')) {
        const receiptStore = useReceiptStore.getState();
        const newReceiptNumber = receiptStore.generateReceiptNumber();
        currentOrder.receiptNumber = newReceiptNumber;

        // Update CartStore for persistence across back navigation
        import('@/core/stores/cart/useCartStore').then((module) => {
          module.useCartStore.getState().setReceiptNumber(newReceiptNumber);
        });

        if (onUpdateOrder) onUpdateOrder(currentOrder);
      }

      // Ensure active order in event store
      ensureActiveOrder(currentOrder);

      // Update info in store (Always mark as pre-payment printed)
      // Always include receiptNumber so the timeline event contains it for display
      const infoToUpdate: any = {
        isPrePayment: true,
        receiptNumber: currentOrder.receiptNumber
      };

      store.updateOrderInfo(currentOrder.key, infoToUpdate);

      // Update local object for printing
      currentOrder.isPrePayment = true;

      await printOrderReceipt(currentOrder);
      toast.success(t('settings.payment.receiptPrintSuccess'));
    } catch (error) {
      console.error('Pre-payment print failed:', error);
      toast.error(t('settings.payment.receiptPrintFailed'));
    }
  }, [order, onUpdateOrder, t]);

  const handleSplitPayment = useCallback(
    async (method: 'CASH' | 'CARD', cashDetails?: { tendered: number }) => {
      if (!order || isProcessingSplit) return false;

      const itemsToSplit = (Object.entries(splitItems) as [string, number][])
        .filter(([_, qty]) => qty > 0)
        .map(([instanceId, qty]) => {
          const originalItem = order.items.find((i) => i.instanceId === instanceId);
          return {
            instanceId,
            quantity: qty,
            name: originalItem?.name || t('common.unknownItem'),
            price: originalItem?.price || 0,
            selectedOptions: originalItem?.selectedOptions,
          };
        });

      if (itemsToSplit.length === 0) return false;

      setIsProcessingSplit(true);
      const eventStore = useOrderEventStore.getState();

      try {
        let total = 0;
        itemsToSplit.forEach((splitItem) => {
          total += splitItem.price * splitItem.quantity;
        });

        const payment = {
          method,
          amount: total,
          tip: 0,
        };

        eventStore.addSplitEvent(order.key, {
          splitAmount: payment.amount,
          items: itemsToSplit.map((i) => ({
            instanceId: i.instanceId,
            name: i.name,
            quantity: i.quantity,
            price: i.price,
            selectedOptions: i.selectedOptions,
          })),
          paymentMethod: method,
          tendered: cashDetails?.tendered,
          change: cashDetails ? cashDetails.tendered - payment.amount : undefined,
        });

        const updatedOrder = eventStore.getOrder(order.key);
        if (updatedOrder && onUpdateOrder) {
          onUpdateOrder(updatedOrder);

          const isFullyPaid =
            updatedOrder.total > 0 &&
            (updatedOrder.paidAmount ?? 0) >= updatedOrder.total - 0.01;

          if (isFullyPaid) {
            try {
              const completed = await completeOrder(updatedOrder, []);
              const isRetail = updatedOrder.isRetail;

              if (method === 'CASH' && cashDetails?.tendered !== undefined) {
                setSuccessModal({
                  isOpen: true,
                  type: 'CASH',
                  change: cashDetails.tendered - payment.amount,
                  onClose: handleComplete,
                  onPrint: isRetail ? async () => {
                    await printOrderReceipt(completed);
                    toast.success(t('settings.payment.receiptPrintSuccess'));
                  } : undefined,
                  autoCloseDelay: isRetail ? 0 : 10000,
                });
              } else {
                setSuccessModal({
                  isOpen: true,
                  type: 'NORMAL',
                  onClose: handleComplete,
                  onPrint: isRetail ? async () => {
                    await printOrderReceipt(completed);
                    toast.success(t('settings.payment.receiptPrintSuccess'));
                  } : undefined,
                  autoCloseDelay: isRetail ? 0 : 5000,
                });
              }
            } catch (error) {
              console.error('Auto-complete failed:', error);
            }
          } else if (method === 'CASH' && cashDetails?.tendered !== undefined) {
            setSuccessModal({
              isOpen: true,
              type: 'CASH',
              change: cashDetails.tendered - payment.amount,
              onClose: () => setSuccessModal(null),
              autoCloseDelay: 10000,
            });
          }
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
    [order, isProcessingSplit, splitItems, onUpdateOrder, t, handleComplete]
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
      const item = order.items.find(i => i.instanceId === instanceId);
      if (item) {
        total += (item.price || 0) * qty;
      }
    });
    return total;
  }, [splitItems, order]);

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
          onUpdateOrder={onUpdateOrder}
          onManage={onManageTable}
        />
        <div className="flex-1 flex flex-col bg-gray-50">
          <div className="p-6 bg-white border-b border-gray-200 shadow-sm flex justify-between items-center">
            <h3 className="font-bold text-gray-800 text-xl flex items-center gap-2">
              <Split size={24} className="text-purple-600" />
              {t('checkout.split.title')}
            </h3>
            <button onClick={() => { setMode('SELECT'); setSplitItems({}); }} className="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium flex items-center gap-2 transition-all">
              <ArrowLeft size={20} /> {t('common.back')}
            </button>
          </div>

          <div className="flex-1 overflow-y-auto p-6 custom-scrollbar">
            <div className="space-y-3 max-w-3xl mx-auto">
              {order.items.map(item => {
                const currentSplitQty = splitItems[item.instanceId] || 0;
                // Calculate remaining quantity available for split
                // (Total quantity - Already Paid quantity)
                const paidQty = (order.paidItemQuantities && order.paidItemQuantities[item.instanceId]) || 0;
                const maxQty = Math.max(0, item.quantity - paidQty);

                return (
                  <div key={item.instanceId} className={`bg-white p-4 rounded-xl border border-gray-200 shadow-sm flex items-center justify-between ${maxQty === 0 ? 'opacity-60 bg-gray-50' : ''}`}>
                    <div className="flex-1">
                      <div className="font-bold text-gray-800 text-lg">
                        {item.name}
                        {paidQty > 0 && <span className="text-xs text-green-600 ml-2 font-medium">({t('common.paidQuantity', { qty: paidQty.toString() })})</span>}
                      </div>
                      {item.selectedOptions && item.selectedOptions.length > 0 && (
                        <div className="text-sm text-gray-500">
                          {item.selectedOptions.map(opt => opt.option_name || opt.value).join(', ')}
                        </div>
                      )}
                      <div className="text-gray-500">{formatCurrency(item.price || 0)}</div>
                    </div>

                    <div className="flex items-center gap-4 bg-gray-50 rounded-lg p-1.5 border border-gray-100">
                      <button
                        onClick={() => setSplitItems(prev => ({ ...prev, [item.instanceId]: Math.max(0, (prev[item.instanceId] || 0) - 1) }))}
                        disabled={currentSplitQty <= 0}
                        className="w-10 h-10 flex items-center justify-center rounded-lg bg-white border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-50 transition-colors shadow-sm"
                      >
                        <Minus size={18} />
                      </button>
                      <span className="w-10 text-center font-bold text-gray-800 text-lg">{currentSplitQty}</span>
                      <button
                        onClick={() => setSplitItems(prev => ({ ...prev, [item.instanceId]: Math.min(maxQty, (prev[item.instanceId] || 0) + 1) }))}
                        disabled={currentSplitQty >= maxQty || maxQty === 0}
                        className="w-10 h-10 flex items-center justify-center rounded-lg bg-white border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-50 transition-colors shadow-sm"
                      >
                        <Plus size={18} />
                      </button>
                    </div>
                  </div>
                );
              })}
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
                  {t('checkout.split.payCash')}
                </button>
                <button
                  onClick={() => handleSplitPayment('CARD')}
                  disabled={splitTotal <= 0 || isProcessingSplit}
                  className="flex items-center justify-center gap-3 py-4 bg-blue-600 text-white rounded-xl font-bold text-lg hover:bg-blue-700 hover:shadow-lg transition-all disabled:opacity-50 disabled:cursor-not-allowed active:scale-[0.99]"
                >
                  <CreditCard size={24} />
                  {t('checkout.split.payCard')}
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
          onUpdateOrder={onUpdateOrder}
          onManage={onManageTable}
        />
        <div className="flex-1 flex flex-col bg-gray-50">
          <div className="p-6 bg-white border-b border-gray-200 shadow-sm">
            <div className="flex items-center justify-between mb-4">
              <h2 className="text-2xl font-bold text-gray-800">{t('checkout.payment.method')}</h2>
              <div className="flex gap-2 items-center">
                {order.isRetail && (
                  <div className="flex bg-gray-100 p-1 rounded-lg h-[40px] items-center mr-2">
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
                      {t('checkout.orderType.dineIn')}
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
                      {t('checkout.orderType.takeout')}
                    </button>
                  </div>
                )}
                {!order.isRetail && (
                  <button onClick={handlePrintPrePayment} className="px-4 py-2 bg-blue-100 hover:bg-blue-200 text-blue-700 rounded-lg font-medium transition-colors flex items-center gap-2">
                    <Printer size={20} />
                    {t('checkout.prePayment.receipt')}
                  </button>
                )}
                {onVoid && !order.isRetail && (
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
                  <button onClick={onCancel} className="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium transition-colors flex items-center gap-2">
                    <ArrowLeft size={20} />
                    {t('common.back')}
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

          <div className="flex-1 p-8 grid grid-cols-3 gap-6 overflow-y-auto">
            <button onClick={handleFullCashPayment} disabled={isPaidInFull || isProcessing} className="h-40 bg-gradient-to-br from-green-500 to-green-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
              <Coins size={48} />
              <div className="text-2xl font-bold">{t('checkout.method.cash')}</div>
              <div className="text-sm opacity-90">{t('checkout.method.cashDesc')}</div>
            </button>
            <button onClick={handleFullCardPayment} disabled={isPaidInFull || isProcessing} className="h-40 bg-gradient-to-br from-blue-500 to-blue-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
              <CreditCard size={48} />
              <div className="text-2xl font-bold">{t('checkout.method.card')}</div>
              <div className="text-sm opacity-90">{t('checkout.method.cardDesc')}</div>
            </button>
            <button onClick={() => setMode('ITEM_SPLIT')} disabled={isPaidInFull || isProcessing} className="h-40 bg-gradient-to-br from-purple-500 to-purple-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
              <Split size={48} />
              <div className="text-2xl font-bold">{t('checkout.split.title')}</div>
              <div className="text-sm opacity-90">{t('checkout.split.desc')}</div>
            </button>
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
      default:
        return renderSelectMode();
    }
  };

  return (
    <>
      <div className="h-full">{renderContent()}</div>
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
