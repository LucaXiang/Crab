import React, { useState, useMemo, useCallback } from 'react';
import { HeldOrder, PaymentRecord } from '@/core/domain/types';
import { Coins, CreditCard, ArrowLeft, Printer, Trash2, Split, Banknote, Utensils, ShoppingBag, Receipt, Check, Gift, Percent, TrendingUp, ClipboardList, Archive, UserCheck, UserX } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { Permission } from '@/core/domain/types';
import { useRetailServiceType, setRetailServiceType, toBackendServiceType } from '@/core/stores/order/useCheckoutStore';
import { OrderDiscountModal } from '../OrderDiscountModal';
import { OrderSurchargeModal } from '../OrderSurchargeModal';
import { formatCurrency, Currency } from '@/utils/currency';
import { openCashDrawer } from '@/core/services/order/paymentService';
import { completeOrder, updateOrderInfo, linkMember, unlinkMember } from '@/core/stores/order/commands';
import { CashPaymentModal } from './CashPaymentModal';
import { PaymentSuccessModal } from './PaymentSuccessModal';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { ConfirmDialog } from '@/shared/components';
import { MemberLinkModal } from '../MemberLinkModal';

type PaymentMode = 'ITEM_SPLIT' | 'AMOUNT_SPLIT' | 'PAYMENT_RECORDS' | 'COMP' | 'ORDER_DETAIL';

interface SelectModePageProps {
  order: HeldOrder;
  onComplete: () => void;
  onCancel?: () => void;
  onVoid?: () => void;
  onManageTable?: () => void;
  onNavigate: (mode: PaymentMode) => void;
}

export const SelectModePage: React.FC<SelectModePageProps> = ({ order, onComplete, onCancel, onVoid, onManageTable, onNavigate }) => {
  const { t } = useI18n();
  const serviceType = useRetailServiceType();

  const totalPaid = order.paid_amount;
  const remaining = order.remaining_amount;
  const isPaidInFull = remaining <= 0;

  const activePayments = useMemo(() => {
    return [...(order.payments || [])]
      .filter(p => !p.cancelled)
      .sort((a, b) => b.timestamp - a.timestamp);
  }, [order.payments]);

  const isAALocked = !!(order.aa_total_shares && order.aa_total_shares > 0);

  const [isProcessing, setIsProcessing] = useState(false);
  const [showCashModal, setShowCashModal] = useState(false);
  const [successModal, setSuccessModal] = useState<{
    isOpen: boolean;
    type: 'NORMAL' | 'CASH';
    change?: number;
    onClose: () => void;
    onPrint?: () => void;
    autoCloseDelay: number;
  } | null>(null);

  const [showRetailCancelConfirm, setShowRetailCancelConfirm] = useState(false);
  const [showCompleteConfirm, setShowCompleteConfirm] = useState(false);
  const [showDiscountModal, setShowDiscountModal] = useState(false);
  const [showSurchargeModal, setShowSurchargeModal] = useState(false);
  const [showMemberModal, setShowMemberModal] = useState(false);

  const handleComplete = useCallback(() => {
    requestAnimationFrame(() => {
      onComplete();
    });
  }, [onComplete]);

  const handleBackClick = useCallback(() => {
    if (order.is_retail) {
      setShowRetailCancelConfirm(true);
    } else {
      onCancel?.();
    }
  }, [order.is_retail, onCancel]);

  const handleManualComplete = useCallback(async () => {
    setIsProcessing(true);
    try {
      await completeOrder(order.order_id, [], order.is_retail ? toBackendServiceType(serviceType) : null);
      setSuccessModal({
        isOpen: true,
        type: 'NORMAL',
        onClose: handleComplete,
        autoCloseDelay: order.is_retail ? 0 : 5000,
      });
    } catch (error) {
      logger.error('Manual complete failed', error);
      toast.error(t('checkout.payment.failed'));
    } finally {
      setIsProcessing(false);
    }
  }, [order, handleComplete, serviceType, t]);

  const handleFullCashPayment = useCallback(() => {
    setShowCashModal(true);
  }, []);

  const handleConfirmFullCash = useCallback(
    async (tenderedAmount: number) => {
      if (Currency.lt(tenderedAmount, remaining)) {
        toast.error(t('settings.payment.amount_insufficient'));
        return;
      }

      setIsProcessing(true);
      try {
        await openCashDrawer();

        const payment: PaymentRecord = {
          payment_id: `pay-${Date.now()}`,
          method: 'CASH',
          amount: remaining,
          timestamp: Date.now(),
          tendered: tenderedAmount,
          change: Currency.sub(tenderedAmount, remaining).toNumber(),
        };

        await completeOrder(order.order_id, [payment], order.is_retail ? toBackendServiceType(serviceType) : null);
        const is_retail = order.is_retail;

        setShowCashModal(false);
        setSuccessModal({
          isOpen: true,
          type: 'CASH',
          change: payment.change ?? undefined,
          onClose: handleComplete,
          autoCloseDelay: is_retail ? 0 : 10000,
        });
      } catch (error) {
        logger.error('Cash payment failed', error);
        toast.error(t('checkout.payment.failed'));
      } finally {
        setIsProcessing(false);
      }
    },
    [remaining, order, handleComplete, t, serviceType]
  );

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

      await completeOrder(order.order_id, [payment], order.is_retail ? toBackendServiceType(serviceType) : null);
      const is_retail = order.is_retail;

      setSuccessModal({
        isOpen: true,
        type: 'NORMAL',
        onClose: handleComplete,
        autoCloseDelay: is_retail ? 0 : 5000,
      });
    } catch (error) {
      logger.error('Card payment failed', error);
      toast.error(t('checkout.payment.failed'));
    } finally {
      setIsProcessing(false);
    }
  }, [remaining, order, handleComplete, t, serviceType]);

  const handlePrintPrePayment = useCallback(async () => {
    try {
      if (!order.receipt_number) {
        toast.error(t('checkout.error.no_receipt_number'));
        return;
      }

      await updateOrderInfo(order.order_id, {
        is_pre_payment: true,
      });

      toast.success(t('settings.payment.receipt_print_success'));
    } catch (error) {
      logger.error('Pre-payment print failed', error);
      toast.error(t('settings.payment.receipt_print_failed'));
    }
  }, [order, t]);

  return (
    <>
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
                {isPaidInFull && (
                  <button onClick={() => setShowCompleteConfirm(true)} disabled={isProcessing} className="px-4 py-2 bg-green-100 hover:bg-green-200 text-green-700 rounded-lg font-medium transition-colors flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed">
                    <Check size={20} />
                    {t('checkout.complete_order')}
                  </button>
                )}
                <EscalatableGate
                  permission={Permission.CASH_DRAWER_OPEN}
                  mode="intercept"
                  description={t('app.action.open_cash_drawer')}
                  onAuthorized={() => {
                    openCashDrawer();
                    toast.success(t('app.action.cash_drawer_opened'));
                  }}
                >
                  <button className="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium transition-colors flex items-center gap-2">
                    <Archive size={20} />
                    {t('app.action.open_cash_drawer')}
                  </button>
                </EscalatableGate>
                {onVoid && !order.is_retail && (
                  <EscalatableGate
                    permission={Permission.ORDERS_VOID}
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
                <div className="text-2xl font-bold text-gray-900 mt-1 tabular-nums">{formatCurrency(order.total)}</div>
              </div>
              <div className="p-4 bg-blue-50 rounded-xl">
                <div className="text-xs text-gray-600 uppercase font-bold">{t('checkout.amount.paid')}</div>
                <div className="text-2xl font-bold text-blue-600 mt-1 tabular-nums">{formatCurrency(totalPaid)}</div>
              </div>
              <div className={`p-4 rounded-xl ${isPaidInFull ? 'bg-green-50' : 'bg-red-50'}`}>
                <div className="text-xs text-gray-600 uppercase font-bold">{t('checkout.amount.remaining')}</div>
                <div className={`text-2xl font-bold mt-1 tabular-nums ${isPaidInFull ? 'text-green-600' : 'text-red-600'}`}>{formatCurrency(remaining)}</div>
              </div>
            </div>
          </div>

          <div className="flex-1 p-8 overflow-y-auto space-y-6">
            {/* Payment Method Buttons */}
            <div className="grid grid-cols-3 gap-6">
              <button onClick={handleFullCashPayment} disabled={isPaidInFull || isProcessing} className="h-40 bg-green-500 hover:bg-green-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <Coins size={48} />
                <div className="text-2xl font-bold">{t('checkout.method.cash')}</div>
                <div className="text-sm opacity-90">{t('checkout.method.cash_desc')}</div>
              </button>
              <button onClick={handleFullCardPayment} disabled={isPaidInFull || isProcessing} className="h-40 bg-blue-500 hover:bg-blue-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <CreditCard size={48} />
                <div className="text-2xl font-bold">{t('checkout.method.card')}</div>
                <div className="text-sm opacity-90">{t('checkout.method.card_desc')}</div>
              </button>
              <button onClick={() => onNavigate('ITEM_SPLIT')} disabled={isPaidInFull || isProcessing || order.has_amount_split || isAALocked} className="h-40 bg-indigo-500 hover:bg-indigo-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <Split size={48} />
                <div className="text-2xl font-bold">{t('checkout.split.title')}</div>
                <div className="text-sm opacity-90">{isAALocked ? t('checkout.aa_split.locked') : order.has_amount_split ? t('checkout.amount_split.item_split_disabled') : t('checkout.split.desc')}</div>
              </button>
            </div>

            {/* Amount Split & Payment Records Buttons */}
            <div className="grid grid-cols-3 gap-6">
              <button onClick={() => onNavigate('AMOUNT_SPLIT')} disabled={isPaidInFull || isProcessing} className="h-40 bg-cyan-600 hover:bg-cyan-700 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <Banknote size={48} />
                <div className="text-2xl font-bold">{isAALocked ? t('checkout.aa_split.title') : t('checkout.amount_split.title')}</div>
                <div className="text-sm opacity-90">{isAALocked ? t('checkout.aa_split.desc') : t('checkout.amount_split.desc')}</div>
              </button>

              <button
                onClick={() => onNavigate('PAYMENT_RECORDS')}
                disabled={activePayments.length === 0}
                className="h-40 bg-teal-600 hover:bg-teal-700 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
              >
                <Receipt size={48} />
                <div className="text-2xl font-bold">{t('checkout.payment.records')}</div>
                <div className="text-sm opacity-90">{activePayments.length} {t('checkout.payment.record_count')} · {formatCurrency(totalPaid)}</div>
              </button>
              <button
                onClick={() => onNavigate('ORDER_DETAIL')}
                className="h-40 bg-amber-500 hover:bg-amber-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all flex flex-col items-center justify-center gap-4"
              >
                <ClipboardList size={48} />
                <div className="text-2xl font-bold">{t('checkout.order_detail.title')}</div>
                <div className="text-sm opacity-90">{t('checkout.order_detail.desc')}</div>
              </button>
            </div>

            {/* Member Management */}
            <div className="grid grid-cols-3 gap-6">
              {order.member_id ? (
                <button
                  onClick={async () => {
                    try {
                      await unlinkMember(order.order_id);
                      toast.success(t('checkout.member.unlinked'));
                    } catch (e) {
                      toast.error(t('checkout.member.unlink_failed'));
                    }
                  }}
                  disabled={isProcessing}
                  className="h-40 bg-rose-500 hover:bg-rose-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
                >
                  <UserX size={48} />
                  <div className="text-2xl font-bold">{t('checkout.member.unlink')}</div>
                  <div className="text-sm opacity-90">{order.member_name}</div>
                </button>
              ) : (
                <button
                  onClick={() => setShowMemberModal(true)}
                  disabled={isProcessing}
                  className="h-40 bg-teal-500 hover:bg-teal-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
                >
                  <UserCheck size={48} />
                  <div className="text-2xl font-bold">{t('checkout.member.link')}</div>
                  <div className="text-sm opacity-90">{t('checkout.member.link_desc')}</div>
                </button>
              )}
            </div>

            {/* Order Adjustments: Comp, Discount, Surcharge */}
            <div className="grid grid-cols-3 gap-6">
              <button
                onClick={() => onNavigate('COMP')}
                disabled={isPaidInFull || isProcessing}
                className="h-40 bg-emerald-500 hover:bg-emerald-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
              >
                <Gift size={48} />
                <div className="text-2xl font-bold">{t('checkout.comp.title')}</div>
                <div className="text-sm opacity-90">{t('checkout.comp.desc')}</div>
              </button>
              <button
                onClick={() => setShowDiscountModal(true)}
                disabled={isProcessing}
                className="h-40 bg-orange-500 hover:bg-orange-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
              >
                <Percent size={48} />
                <div className="text-2xl font-bold">{t('checkout.order_discount.title')}</div>
                <div className="text-sm opacity-90">{t('checkout.order_discount.desc')}</div>
              </button>
              <button
                onClick={() => setShowSurchargeModal(true)}
                disabled={isProcessing}
                className="h-40 bg-purple-500 hover:bg-purple-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
              >
                <TrendingUp size={48} />
                <div className="text-2xl font-bold">{t('checkout.order_surcharge.title')}</div>
                <div className="text-sm opacity-90">{t('checkout.order_surcharge.desc')}</div>
              </button>
            </div>
          </div>
        </div>
      </div>

      {/* Manual Complete Confirmation Dialog */}
      <ConfirmDialog
        isOpen={showCompleteConfirm}
        title={t('checkout.complete_order')}
        description={t('checkout.complete_order_confirm')}
        confirmText={t('checkout.complete_order')}
        onConfirm={() => {
          setShowCompleteConfirm(false);
          handleManualComplete();
        }}
        onCancel={() => setShowCompleteConfirm(false)}
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
        amountDue={remaining}
        isProcessing={isProcessing}
        onConfirm={handleConfirmFullCash}
        onCancel={() => setShowCashModal(false)}
      />

      <OrderDiscountModal
        isOpen={showDiscountModal}
        order={order}
        onClose={() => setShowDiscountModal(false)}
      />

      <OrderSurchargeModal
        isOpen={showSurchargeModal}
        order={order}
        onClose={() => setShowSurchargeModal(false)}
      />

      <MemberLinkModal
        isOpen={showMemberModal}
        orderId={order.order_id}
        onClose={() => setShowMemberModal(false)}
      />
    </>
  );
};
