import React, { useState, useMemo, useCallback, useRef, useEffect } from 'react';
import { HeldOrder } from '@/core/domain/types';
import { Coins, CreditCard, ArrowLeft, Printer, Trash2, Split, Banknote, Utensils, ShoppingBag, Receipt, Check, Gift, Percent, TrendingUp, ClipboardList, Archive, UserCheck, Stamp, X, Crown, LayoutGrid, Tag, MoreHorizontal } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { CommandFailedError } from '@/core/stores/order/commands/sendCommand';
import { commandErrorMessage } from '@/utils/error/commandError';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { Permission, Table, Zone } from '@/core/domain/types';
import { setRetailServiceType } from '@/core/stores/order/useCheckoutStore';
import { useActiveOrdersStore } from '@/core/stores/order/useActiveOrdersStore';
import { mergeOrders } from '@/core/stores/order/commands';
import { createCommand } from '@/core/stores/order/commandUtils';
import { sendCommand, ensureSuccess } from '@/core/stores/order/commands/sendCommand';
import { TableSelectionScreen } from '@/screens/TableSelection';
import { clearPendingRetailOrder } from '@/core/stores/order/retailOrderTracker';
import { OrderDiscountModal } from '../OrderDiscountModal';
import { OrderSurchargeModal } from '../OrderSurchargeModal';
import { formatCurrency } from '@/utils/currency';
import { openCashDrawer } from '@/core/services/order/paymentService';
import { CashPaymentModal } from './CashPaymentModal';
import { PaymentSuccessModal } from './PaymentSuccessModal';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { ConfirmDialog } from '@/shared/components';
import { MemberLinkModal } from '../MemberLinkModal';
import { StampRewardPickerModal } from './StampRewardPickerModal';
import { StampRedeemModal } from './StampRedeemModal';
import { getMatchingItems, getDesignatedMatchingItems } from '@/utils/stampMatching';
import { useStampProgress } from './useStampProgress';
import { usePaymentActions } from './usePaymentActions';
import { KitchenReprintModal } from '../KitchenReprintModal';
import { LabelReprintModal } from '../LabelReprintModal';

type PaymentMode = 'ITEM_SPLIT' | 'AMOUNT_SPLIT' | 'PAYMENT_RECORDS' | 'COMP' | 'ORDER_DETAIL' | 'MEMBER_DETAIL';

interface SelectModePageProps {
  order: HeldOrder;
  onComplete: () => void;
  onCancel?: () => void;
  onVoid?: () => void;
  onManageTable?: () => void;
  onNavigate: (mode: PaymentMode) => void;
}

const MoreActionsDropdown: React.FC<{
  onKitchenReprint: () => void;
  onLabelReprint: () => void;
  onOpenCashDrawer: () => void;
}> = ({ onKitchenReprint, onLabelReprint, onOpenCashDrawer }) => {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [open]);

  return (
    <div className="relative" ref={ref}>
      <button
        onClick={() => setOpen(!open)}
        className="px-4 py-2 bg-gray-100 hover:bg-gray-200 text-gray-700 rounded-lg font-medium transition-colors flex items-center gap-2"
      >
        <MoreHorizontal size={20} />
      </button>
      {open && (
        <div className="absolute right-0 top-full mt-1 bg-white rounded-lg shadow-xl border border-gray-200 py-1 z-50 min-w-[200px]">
          <button
            onClick={() => { setOpen(false); onKitchenReprint(); }}
            className="w-full px-4 py-3 text-left hover:bg-amber-50 text-gray-700 flex items-center gap-3 transition-colors"
          >
            <Printer size={18} className="text-amber-600" />
            {t('checkout.kitchen_reprint.tab_kitchen')}
          </button>
          <button
            onClick={() => { setOpen(false); onLabelReprint(); }}
            className="w-full px-4 py-3 text-left hover:bg-amber-50 text-gray-700 flex items-center gap-3 transition-colors"
          >
            <Tag size={18} className="text-amber-600" />
            {t('checkout.label_reprint.tab_label')}
          </button>
          <EscalatableGate
            permission={Permission.CASH_DRAWER_OPEN}
            mode="intercept"
            description={t('app.action.open_cash_drawer')}
            onAuthorized={() => { setOpen(false); onOpenCashDrawer(); }}
          >
            <button
              className="w-full px-4 py-3 text-left hover:bg-gray-50 text-gray-700 flex items-center gap-3 transition-colors"
            >
              <Archive size={18} className="text-gray-500" />
              {t('app.action.open_cash_drawer')}
            </button>
          </EscalatableGate>
        </div>
      )}
    </div>
  );
};

export const SelectModePage: React.FC<SelectModePageProps> = ({ order, onComplete, onCancel, onVoid, onManageTable, onNavigate }) => {
  const { t } = useI18n();

  const totalPaid = order.paid_amount;
  const remaining = order.remaining_amount;
  const isPaidInFull = remaining <= 0;

  const activePayments = useMemo(() => {
    return [...(order.payments || [])]
      .filter(p => !p.cancelled)
      .sort((a, b) => b.timestamp - a.timestamp);
  }, [order.payments]);

  const isAALocked = !!(order.aa_total_shares && order.aa_total_shares > 0);

  // Extracted hooks
  const stamp = useStampProgress(order);
  const payment = usePaymentActions(order, onComplete);

  const [showRetailCancelConfirm, setShowRetailCancelConfirm] = useState(false);
  const [showCompleteConfirm, setShowCompleteConfirm] = useState(false);
  const [showDiscountModal, setShowDiscountModal] = useState(false);
  const [showSurchargeModal, setShowSurchargeModal] = useState(false);
  const [showMemberModal, setShowMemberModal] = useState(false);
  const [showMergeTableModal, setShowMergeTableModal] = useState(false);
  const [showKitchenReprintModal, setShowKitchenReprintModal] = useState(false);
  const [showLabelReprintModal, setShowLabelReprintModal] = useState(false);

  const handleBackClick = useCallback(() => {
    if (order.is_retail) {
      setShowRetailCancelConfirm(true);
    } else {
      onCancel?.();
    }
  }, [order.is_retail, onCancel]);

  const handleMergeToTable = useCallback(async (table: Table, guestCount: number, zone?: Zone) => {
    setShowMergeTableModal(false);

    try {
      const store = useActiveOrdersStore.getState();
      const targetSnapshot = store.getOrderByTable(table.id);

      if (targetSnapshot && targetSnapshot.status === 'ACTIVE') {
        await mergeOrders(order.order_id, targetSnapshot.order_id);
      } else {
        const openCmd = createCommand({
          type: 'OPEN_TABLE',
          table_id: table.id,
          table_name: table.name,
          zone_id: zone?.id ?? null,
          zone_name: zone?.name ?? null,
          guest_count: guestCount,
          is_retail: false,
        });
        const openRes = await sendCommand(openCmd);
        ensureSuccess(openRes, 'Open table for merge');
        const newOrderId = openRes.order_id;
        if (!newOrderId) {
          throw new Error('OPEN_TABLE succeeded but no order_id returned');
        }
        await mergeOrders(order.order_id, newOrderId);
      }

      clearPendingRetailOrder();
      toast.success(t('checkout.merge_to_table.success'));
      onComplete();
    } catch (error) {
      logger.error('Merge to table failed', error);
      toast.error(
        error instanceof CommandFailedError
          ? commandErrorMessage(error.code)
          : t('checkout.merge_to_table.failed')
      );
    }
  }, [order.order_id, onComplete, t]);

  return (
    <>
      <div className="h-full flex">
        {payment.successModal && (
          <PaymentSuccessModal
            isOpen={payment.successModal.isOpen}
            type={payment.successModal.type}
            change={payment.successModal.change}
            onClose={payment.successModal.onClose}
            autoCloseDelay={payment.successModal.autoCloseDelay}
            onPrint={payment.successModal.onPrint}
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
                        ${payment.serviceType === 'dineIn'
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
                        ${payment.serviceType === 'takeout'
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
                  <button onClick={payment.handlePrintPrePayment} className="px-4 py-2 bg-blue-100 hover:bg-blue-200 text-blue-700 rounded-lg font-medium transition-colors flex items-center gap-2">
                    <Printer size={20} />
                    {t('checkout.pre_payment.receipt')}
                  </button>
                )}
                {isPaidInFull && (
                  <button onClick={() => setShowCompleteConfirm(true)} disabled={payment.isProcessing} className="px-4 py-2 bg-green-100 hover:bg-green-200 text-green-700 rounded-lg font-medium transition-colors flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed">
                    <Check size={20} />
                    {t('checkout.complete_order')}
                  </button>
                )}
                <MoreActionsDropdown
                  onKitchenReprint={() => setShowKitchenReprintModal(true)}
                  onLabelReprint={() => setShowLabelReprintModal(true)}
                  onOpenCashDrawer={() => {
                    openCashDrawer();
                    toast.success(t('app.action.cash_drawer_opened'));
                  }}
                />
                {order.is_retail && (
                  <button
                    onClick={() => setShowMergeTableModal(true)}
                    disabled={payment.isProcessing}
                    className="px-4 py-2 bg-primary-100 hover:bg-primary-200 text-primary-700 rounded-lg font-medium transition-colors flex items-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
                  >
                    <LayoutGrid size={20} />
                    {t('checkout.merge_to_table.button')}
                  </button>
                )}
                {onVoid && (
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

          <div className="flex-1 p-8 overflow-y-auto">
            {/* Single unified 3-column grid */}
            <div className="grid grid-cols-3 gap-6">
              {/* Row 1: Payment Methods */}
              <button onClick={payment.handleFullCashPayment} disabled={isPaidInFull || payment.isProcessing} className="h-40 bg-green-500 hover:bg-green-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <Coins size={48} />
                <div className="text-2xl font-bold">{t('checkout.method.cash')}</div>
                <div className="text-sm opacity-90">{t('checkout.method.cash_desc')}</div>
              </button>
              <button onClick={payment.handleFullCardPayment} disabled={isPaidInFull || payment.isProcessing} className="h-40 bg-blue-500 hover:bg-blue-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <CreditCard size={48} />
                <div className="text-2xl font-bold">{t('checkout.method.card')}</div>
                <div className="text-sm opacity-90">{t('checkout.method.card_desc')}</div>
              </button>
              <button onClick={() => onNavigate('ITEM_SPLIT')} disabled={isPaidInFull || payment.isProcessing || order.has_amount_split || isAALocked} className="h-40 bg-indigo-500 hover:bg-indigo-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <Split size={48} />
                <div className="text-2xl font-bold">{t('checkout.split.title')}</div>
                <div className="text-sm opacity-90">{isAALocked ? t('checkout.aa_split.locked') : order.has_amount_split ? t('checkout.amount_split.item_split_disabled') : t('checkout.split.desc')}</div>
              </button>

              {/* Row 2: Amount Split, Records, Detail */}
              <button onClick={() => onNavigate('AMOUNT_SPLIT')} disabled={isPaidInFull || payment.isProcessing} className="h-40 bg-cyan-600 hover:bg-cyan-700 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
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
                className="h-40 bg-sky-500 hover:bg-sky-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all flex flex-col items-center justify-center gap-4"
              >
                <ClipboardList size={48} />
                <div className="text-2xl font-bold">{t('checkout.order_detail.title')}</div>
                <div className="text-sm opacity-90">{t('checkout.order_detail.desc')}</div>
              </button>

              {/* Row 3: Comp, Discount, Surcharge */}
              <button
                onClick={() => onNavigate('COMP')}
                disabled={isPaidInFull || payment.isProcessing}
                className="h-40 bg-emerald-500 hover:bg-emerald-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
              >
                <Gift size={48} />
                <div className="text-2xl font-bold">{t('checkout.comp.title')}</div>
                <div className="text-sm opacity-90">{t('checkout.comp.desc')}</div>
              </button>
              <button
                onClick={() => setShowDiscountModal(true)}
                disabled={payment.isProcessing}
                className="h-40 bg-orange-500 hover:bg-orange-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
              >
                <Percent size={48} />
                <div className="text-2xl font-bold">{t('checkout.order_discount.title')}</div>
                <div className="text-sm opacity-90">{t('checkout.order_discount.desc')}</div>
              </button>
              <button
                onClick={() => setShowSurchargeModal(true)}
                disabled={payment.isProcessing}
                className="h-40 bg-purple-500 hover:bg-purple-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
              >
                <TrendingUp size={48} />
                <div className="text-2xl font-bold">{t('checkout.order_surcharge.title')}</div>
                <div className="text-sm opacity-90">{t('checkout.order_surcharge.desc')}</div>
              </button>

              {/* Trailing: Member + Stamp Cards */}
              {order.member_id ? (
                <button
                  onClick={() => onNavigate('MEMBER_DETAIL')}
                  disabled={payment.isProcessing}
                  className="h-40 bg-primary-500 hover:bg-primary-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
                >
                  <Crown size={48} />
                  <div className="text-2xl font-bold">{order.member_name}</div>
                  <div className="text-sm opacity-90">{order.marketing_group_name}</div>
                </button>
              ) : (
                <button
                  onClick={() => setShowMemberModal(true)}
                  disabled={payment.isProcessing}
                  className="h-40 bg-primary-500 hover:bg-primary-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
                >
                  <UserCheck size={48} />
                  <div className="text-2xl font-bold">{t('checkout.member.link')}</div>
                  <div className="text-sm opacity-90">{t('checkout.member.link_desc')}</div>
                </button>
              )}

              {/* Stamp cards: only show redeemable or already redeemed */}
              {order.member_id && stamp.stampProgress.map((sp) => {
                const alreadyRedeemed = order.stamp_redemptions?.some(
                  (r) => r.stamp_activity_id === sp.stamp_activity_id
                );
                const orderBonus = order.items
                  .filter((item) => !item.is_comped && sp.stamp_targets.some((t) =>
                    t.target_type === 'PRODUCT' ? t.target_id === item.id
                    : t.target_type === 'CATEGORY' ? t.target_id === item.category_id
                    : false
                  ))
                  .reduce((sum, item) => sum + item.quantity, 0);
                const effectiveStamps = sp.current_stamps + orderBonus;
                const canRedeem = effectiveStamps >= sp.stamps_required && !alreadyRedeemed;
                const progressPercent = Math.min(100, Math.round((effectiveStamps / sp.stamps_required) * 100));

                // Only show redeemable or already-redeemed cards
                if (!canRedeem && !alreadyRedeemed) return null;

                return (
                  <div
                    key={sp.stamp_activity_id}
                    onClick={() => {
                      if (canRedeem) stamp.setStampRedeemActivity(sp);
                    }}
                    className={`h-40 rounded-2xl shadow-xl transition-all relative flex flex-col items-center justify-center gap-2 ${
                      alreadyRedeemed
                        ? 'bg-violet-100 text-violet-600'
                        : 'bg-violet-500 text-white hover:shadow-2xl hover:scale-[1.02] cursor-pointer'
                    }`}
                  >
                    {/* Cancel button for redeemed */}
                    {alreadyRedeemed && (
                      <button
                        onClick={(e) => { e.stopPropagation(); stamp.handleCancelStampRedemption(sp.stamp_activity_id); }}
                        disabled={payment.isProcessing}
                        className="absolute top-2 right-2 p-1.5 rounded-full bg-violet-200 hover:bg-violet-300 text-violet-700 transition-colors disabled:opacity-50"
                        title={t('checkout.stamp.cancel')}
                      >
                        <X size={14} />
                      </button>
                    )}

                    <Stamp size={28} />
                    <div className="text-sm font-bold">{sp.stamp_activity_name}</div>

                    {/* Progress display */}
                    <div className="flex items-baseline gap-1">
                      <span className="text-xl font-black tabular-nums">{effectiveStamps}</span>
                      <span className="text-xs opacity-75">/ {sp.stamps_required}</span>
                      {orderBonus > 0 && (
                        <span className={`text-xs ml-1 ${alreadyRedeemed ? 'text-violet-400' : 'text-white/70'}`}>
                          (+{orderBonus})
                        </span>
                      )}
                    </div>

                    {/* Progress bar */}
                    <div className={`w-3/4 h-1.5 rounded-full overflow-hidden ${alreadyRedeemed ? 'bg-violet-200' : 'bg-white/20'}`}>
                      <div
                        className={`h-full rounded-full transition-all ${alreadyRedeemed ? 'bg-violet-400' : 'bg-white'}`}
                        style={{ width: `${progressPercent}%` }}
                      />
                    </div>

                    {/* Status label */}
                    {alreadyRedeemed ? (
                      <div className="text-xs font-medium bg-violet-200/50 px-3 py-0.5 rounded-full">
                        {t('checkout.stamp.already_redeemed')}
                      </div>
                    ) : (
                      <div className="text-xs font-medium bg-white/20 px-3 py-0.5 rounded-full">
                        {t('checkout.stamp.redeem')}
                      </div>
                    )}
                  </div>
                );
              })}
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
          payment.handleManualComplete();
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
        isOpen={payment.showCashModal}
        amountDue={remaining}
        isProcessing={payment.isProcessing}
        onConfirm={payment.handleConfirmFullCash}
        onCancel={() => payment.setShowCashModal(false)}
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

      {stamp.rewardPickerActivity && (
        <StampRewardPickerModal
          isOpen={!!stamp.rewardPickerActivity}
          activityName={stamp.rewardPickerActivity.stamp_activity_name}
          rewardTargets={stamp.rewardPickerActivity.reward_targets}
          rewardQuantity={stamp.rewardPickerActivity.reward_quantity}
          onSelect={(productId) => stamp.handleSelectionRedeem(stamp.rewardPickerActivity!.stamp_activity_id, productId)}
          onClose={() => stamp.setRewardPickerActivity(null)}
        />
      )}

      {showMergeTableModal && (
        <div className="fixed inset-0 z-50">
          <TableSelectionScreen
            heldOrders={useActiveOrdersStore.getState().getActiveOrders()}
            onSelectTable={handleMergeToTable}
            onClose={() => setShowMergeTableModal(false)}
            mode="HOLD"
            cart={[]}
          />
        </div>
      )}

      <KitchenReprintModal
        isOpen={showKitchenReprintModal}
        orderId={order.order_id}
        onClose={() => setShowKitchenReprintModal(false)}
      />
      <LabelReprintModal
        isOpen={showLabelReprintModal}
        orderId={order.order_id}
        onClose={() => setShowLabelReprintModal(false)}
      />

      {stamp.stampRedeemActivity && (() => {
        const spa = stamp.stampRedeemActivity;
        const isDesignated = spa.reward_strategy === 'DESIGNATED';
        const matchingItems = isDesignated
          ? getDesignatedMatchingItems(order.items, spa)
          : getMatchingItems(order.items, spa);
        const oBonus = order.items
          .filter((item) => !item.is_comped && spa.stamp_targets.some((t) =>
            t.target_type === 'PRODUCT' ? t.target_id === item.id
            : t.target_type === 'CATEGORY' ? t.target_id === item.category_id
            : false
          ))
          .reduce((sum, item) => sum + item.quantity, 0);
        const excess = (spa.current_stamps + oBonus) > spa.stamps_required;
        const eff = spa.current_stamps + oBonus;

        return (
          <StampRedeemModal
            isOpen={!!stamp.stampRedeemActivity}
            activity={spa}
            matchingItems={matchingItems}
            hasExcess={excess}
            effectiveStamps={eff}
            orderBonus={oBonus}
            isProcessing={payment.isProcessing}
            onMatchRedeem={() => stamp.handleMatchRedeem(spa)}
            onSelectRedeem={() => stamp.setRewardPickerActivity(spa)}
            onDirectRedeem={() => stamp.handleDirectRedeem(spa.stamp_activity_id)}
            onClose={() => stamp.setStampRedeemActivity(null)}
          />
        );
      })()}
    </>
  );
};
