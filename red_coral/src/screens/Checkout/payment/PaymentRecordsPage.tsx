import React, { useState, useMemo, useCallback } from 'react';
import { HeldOrder } from '@/core/domain/types';
import { Receipt, ArrowLeft, Coins, CreditCard, Trash2, ChevronRight } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { Permission } from '@/core/domain/types';
import { formatCurrency } from '@/utils/currency';
import { cancelPayment } from '@/core/stores/order/commands';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { ConfirmDialog } from '@/shared/components';
import { SplitItemRow } from '../components';

interface PaymentRecordsPageProps {
  order: HeldOrder;
  onBack: () => void;
  onManageTable?: () => void;
}

export const PaymentRecordsPage: React.FC<PaymentRecordsPageProps> = ({ order, onBack, onManageTable }) => {
  const { t } = useI18n();

  const totalPaid = order.paid_amount;
  const remaining = order.remaining_amount;

  const activePayments = useMemo(() => {
    return [...(order.payments || [])]
      .filter(p => !p.cancelled)
      .sort((a, b) => b.timestamp - a.timestamp);
  }, [order.payments]);

  const [cancellingPaymentId, setCancellingPaymentId] = useState<string | null>(null);
  const [cancelConfirm, setCancelConfirm] = useState<{ paymentId: string; isSplit: boolean } | null>(null);
  const [cancelAuthorizer, setCancelAuthorizer] = useState<{ id: number; name: string } | null>(null);

  const handleCancelPayment = useCallback((paymentId: string, isSplit: boolean) => {
    setCancelConfirm({ paymentId, isSplit });
  }, []);

  const handleConfirmCancelPayment = useCallback(async () => {
    if (!cancelConfirm) return;

    const { paymentId } = cancelConfirm;
    setCancelConfirm(null);
    setCancellingPaymentId(paymentId);

    try {
      await cancelPayment(order.order_id, paymentId, undefined, cancelAuthorizer ?? undefined);
      toast.success(t('checkout.payment.cancel_success'));
    } catch (error) {
      logger.error('Failed to cancel payment', error);
      const message = error instanceof Error ? error.message : t('checkout.payment.cancel_failed');
      toast.error(message);
    } finally {
      setCancellingPaymentId(null);
      setCancelAuthorizer(null);
    }
  }, [cancelConfirm, order.order_id, cancelAuthorizer, t]);

  return (
    <>
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
              <Receipt size={24} className="text-teal-600" />
              {t('checkout.payment.records')}
            </h3>
            <button
              onClick={onBack}
              className="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium flex items-center gap-2 transition-all"
            >
              <ArrowLeft size={20} /> {t('common.action.back')}
            </button>
          </div>

          {/* Payment Records List */}
          <div className="flex-1 overflow-y-auto p-6 custom-scrollbar">
            <div className="space-y-4 max-w-3xl mx-auto">
              {activePayments.map((payment) => {
                const hasItems = payment.split_items && payment.split_items.length > 0;
                const effectiveSplitType = payment.split_type ?? (hasItems ? 'ITEM_SPLIT' : null);
                const isCash = /cash/i.test(payment.method);
                const isCancelling = cancellingPaymentId === payment.payment_id;

                const splitBadge = effectiveSplitType === 'ITEM_SPLIT'
                  ? { label: t('checkout.split.label'), bg: 'bg-indigo-100 text-indigo-600' }
                  : effectiveSplitType === 'AMOUNT_SPLIT'
                  ? { label: t('checkout.amount_split.title'), bg: 'bg-cyan-100 text-cyan-600' }
                  : effectiveSplitType === 'AA_SPLIT'
                  ? { label: t('checkout.aa_split.title'), bg: 'bg-cyan-100 text-cyan-600' }
                  : null;

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
                            {splitBadge && (
                              <span className={`text-xs px-2 py-0.5 rounded-full font-medium ${splitBadge.bg}`}>
                                {splitBadge.label}
                              </span>
                            )}
                          </div>
                          <div className="text-sm text-gray-400 mt-0.5 flex items-center gap-2">
                            <span>{new Date(payment.timestamp).toLocaleString([], {
                              month: 'short',
                              day: 'numeric',
                              hour: '2-digit',
                              minute: '2-digit',
                              hour12: false,
                            })}</span>
                            <span className="text-[0.625rem] text-emerald-600 bg-emerald-100 font-bold font-mono px-1.5 py-0.5 rounded">#{payment.payment_id.slice(-5)}</span>
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
                        <EscalatableGate
                          permission={Permission.ORDERS_REFUND}
                          mode="intercept"
                          description={t('checkout.payment.cancel')}
                          onAuthorized={(user) => {
                            setCancelAuthorizer({ id: user.id, name: user.display_name });
                            handleCancelPayment(payment.payment_id, effectiveSplitType !== null);
                          }}
                        >
                          <button
                            onClick={() => handleCancelPayment(payment.payment_id, effectiveSplitType !== null)}
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
                        </EscalatableGate>
                      </div>
                    </div>

                    {/* Split Items Detail */}
                    {payment.split_items && payment.split_items.length > 0 && (
                      <details className="mt-4 pt-4 border-t border-gray-100">
                        <summary className="text-xs text-gray-400 uppercase font-bold cursor-pointer select-none hover:text-gray-500 transition-colors list-none flex items-center gap-1">
                          <ChevronRight size={14} className="transition-transform [details[open]>&]:rotate-90" />
                          {t('checkout.payment.split_items')} ({payment.split_items.length})
                        </summary>
                        <div className="divide-y divide-gray-50 mt-2">
                          {payment.split_items.map((item, idx) => (
                            <SplitItemRow key={idx} item={item} />
                          ))}
                        </div>
                      </details>
                    )}
                  </div>
                );
              })}
            </div>
          </div>

          {/* Footer Summary */}
          <div className="p-6 bg-white border-t border-gray-200 shadow-up">
            <div className="max-w-3xl mx-auto flex justify-between items-center">
              <span className="text-gray-500 font-medium text-lg">{t('checkout.payment.total_paid')}</span>
              <span className="text-3xl font-bold text-blue-600">{formatCurrency(totalPaid)}</span>
            </div>
          </div>
        </div>
      </div>

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
    </>
  );
};
