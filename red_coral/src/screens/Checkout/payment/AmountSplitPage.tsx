import React, { useState, useCallback } from 'react';
import { HeldOrder } from '@/core/domain/types';
import { CreditCard, ArrowLeft, Minus, Plus, Banknote, Users, PieChart, Lock as LockIcon, Check, Clock } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { useRetailServiceType, toBackendServiceType } from '@/core/stores/order/useCheckoutStore';
import { formatCurrency, Currency } from '@/utils/currency';
import { openCashDrawer } from '@/core/services/order/paymentService';
import { completeOrder, splitByAmount, startAaSplit, payAaSplit } from '@/core/stores/order/commands';
import { CashPaymentModal } from './CashPaymentModal';
import { PaymentSuccessModal } from './PaymentSuccessModal';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { Numpad } from '@/presentation/components/ui/Numpad';

interface AmountSplitPageProps {
  order: HeldOrder;
  onBack: () => void;
  onComplete: () => void;
  onManageTable?: () => void;
}

export const AmountSplitPage: React.FC<AmountSplitPageProps> = ({ order, onBack, onComplete, onManageTable }) => {
  const { t } = useI18n();
  const serviceType = useRetailServiceType();

  const totalPaid = order.paid_amount;
  const remaining = order.remaining_amount;

  const isAALocked = !!(order.aa_total_shares && order.aa_total_shares > 0);
  const aaRemainingShares = isAALocked ? (order.aa_total_shares! - (order.aa_paid_shares ?? 0)) : 0;

  const [amountSplitValue, setAmountSplitValue] = useState<string>('');
  const [isProcessingAmountSplit, setIsProcessingAmountSplit] = useState(false);
  const [showCashModal, setShowCashModal] = useState(false);
  const [successModal, setSuccessModal] = useState<{
    isOpen: boolean;
    type: 'NORMAL' | 'CASH';
    change?: number;
    onClose: () => void;
    onPrint?: () => void;
    autoCloseDelay: number;
  } | null>(null);

  const [splitMode, setSplitMode] = useState<'CUSTOM' | 'AA'>('CUSTOM');
  const [activeInput, setActiveInput] = useState<'CUSTOM' | 'AA_TOTAL' | 'AA_PAY'>('CUSTOM');
  const replaceMode = React.useRef(true);
  const [aaTotalStr, setAATotalStr] = useState<string>(String(order.guest_count || 2));
  const [aaPayStr, setAAPayStr] = useState<string>('1');

  const handleComplete_cb = useCallback(() => {
    requestAnimationFrame(() => {
      onComplete();
    });
  }, [onComplete]);

  // Auto-enter AA mode
  React.useEffect(() => {
    if (isAALocked) {
      setSplitMode('AA');
      setActiveInput('AA_PAY');
      replaceMode.current = true;
      setAATotalStr(order.aa_total_shares!.toString());
      const pay = parseInt(aaPayStr) || 1;
      const maxPay = aaRemainingShares;
      if (pay > maxPay && maxPay > 0) {
        setAAPayStr(maxPay.toString());
      }
    } else {
      setSplitMode('AA');
      setActiveInput('AA_TOTAL');
      replaceMode.current = true;
    }
  }, [isAALocked, order.aa_total_shares, aaRemainingShares]);

  // Sync amountSplitValue with AA state
  React.useEffect(() => {
    if (splitMode === 'AA') {
      const total = isAALocked ? order.aa_total_shares! : (parseInt(aaTotalStr) || 1);
      const remainingSharesForCalc = isAALocked ? aaRemainingShares : total;
      let pay = parseInt(aaPayStr) || 0;

      if (pay > remainingSharesForCalc) {
        pay = remainingSharesForCalc;
        setAAPayStr(remainingSharesForCalc.toString());
      }

      const amount = remainingSharesForCalc > 0
        ? Currency.mul(Currency.div(remaining, remainingSharesForCalc), pay).toDecimalPlaces(2).toNumber()
        : 0;
      setAmountSplitValue(amount.toFixed(2));
    }
  }, [splitMode, aaTotalStr, aaPayStr, remaining, isAALocked, aaRemainingShares, order.aa_total_shares]);

  const handleNumpadInput = useCallback((value: string) => {
    if (value === 'C') {
      if (activeInput === 'CUSTOM') setAmountSplitValue('');
      else if (activeInput === 'AA_TOTAL') setAATotalStr('');
      else if (activeInput === 'AA_PAY') setAAPayStr('');
      replaceMode.current = false;
      return;
    }

    if (value === 'backspace') {
      replaceMode.current = false;
      if (activeInput === 'CUSTOM') setAmountSplitValue(prev => prev.slice(0, -1));
      else if (activeInput === 'AA_TOTAL') setAATotalStr(prev => prev.slice(0, -1));
      else if (activeInput === 'AA_PAY') setAAPayStr(prev => prev.slice(0, -1));
      return;
    }

    const shouldReplace = replaceMode.current;
    if (shouldReplace) replaceMode.current = false;

    if (activeInput === 'CUSTOM') {
      const base = shouldReplace ? '' : amountSplitValue;
      if (value === '.' && base.includes('.')) return;
      const next = base + value;
      if (next.includes('.') && next.split('.')[1].length > 2) return;
      setAmountSplitValue(next);
    } else {
      if (value === '.') return;

      if (activeInput === 'AA_TOTAL') {
        setAATotalStr(prev => {
          const next = (shouldReplace ? '' : prev) + value;
          if (parseInt(next) > 999) return prev;
          return next;
        });
      } else if (activeInput === 'AA_PAY') {
        setAAPayStr(prev => {
          const next = (shouldReplace ? '' : prev) + value;
          const total = parseInt(aaTotalStr) || 1;
          const maxPay = isAALocked ? aaRemainingShares : total;
          if (parseInt(next) > maxPay) return shouldReplace ? value : prev;
          if (parseInt(next) > 999) return prev;
          return next;
        });
      }
    }
  }, [activeInput, amountSplitValue, aaTotalStr, aaPayStr, isAALocked, aaRemainingShares]);

  const handleFocus = (field: 'CUSTOM' | 'AA_TOTAL' | 'AA_PAY') => {
    if (field === 'CUSTOM' && isAALocked) return;
    setActiveInput(field);
    replaceMode.current = true;
    if (field === 'CUSTOM') {
      setSplitMode('CUSTOM');
    } else {
      setSplitMode('AA');
    }
  };

  const incrementAA = (field: 'TOTAL' | 'PAY', delta: number) => {
    setSplitMode('AA');
    if (field === 'TOTAL') {
      setActiveInput('AA_TOTAL');
      const current = parseInt(aaTotalStr) || 0;
      const next = Math.max(1, current + delta);
      setAATotalStr(next.toString());
    } else {
      setActiveInput('AA_PAY');
      const current = parseInt(aaPayStr) || 0;
      const total = parseInt(aaTotalStr) || 1;
      const maxPay = isAALocked ? aaRemainingShares : total;
      const next = Math.max(1, Math.min(maxPay, current + delta));
      setAAPayStr(next.toString());
    }
  };

  const handleAmountSplitPayment = useCallback(
    async (method: 'CASH' | 'CARD', cashDetails?: { tendered: number }) => {
      if (!order || isProcessingAmountSplit) return false;

      const amount = parseFloat(amountSplitValue);
      if (isNaN(amount) || amount <= 0) {
        toast.error(t('checkout.amount_split.invalid_amount'));
        return false;
      }

      if (Currency.sub(amount, remaining).toNumber() > 0.01) {
        toast.error(t('checkout.amount_split.exceeds_remaining'));
        return false;
      }

      setIsProcessingAmountSplit(true);

      try {
        if (method === 'CASH') {
          await openCashDrawer();
        }

        const tendered = method === 'CASH' ? cashDetails?.tendered : undefined;

        if (splitMode === 'AA') {
          const payShares = parseInt(aaPayStr) || 1;
          if (isAALocked) {
            await payAaSplit(order.order_id, payShares, method, tendered);
          } else {
            const totalShares = parseInt(aaTotalStr) || 2;
            await startAaSplit(order.order_id, totalShares, payShares, method, tendered);
          }
        } else {
          await splitByAmount(order.order_id, amount, method, tendered);
        }

        const willComplete = Currency.sub(remaining, amount).toNumber() <= 0.01;

        if (willComplete) {
          await completeOrder(order.order_id, [], order.is_retail ? toBackendServiceType(serviceType) : null);
        }

        if (method === 'CASH' && cashDetails?.tendered !== undefined) {
          setSuccessModal({
            isOpen: true,
            type: 'CASH',
            change: Currency.sub(cashDetails.tendered, amount).toNumber(),
            onClose: willComplete ? handleComplete_cb : () => setSuccessModal(null),
            autoCloseDelay: willComplete && order.is_retail ? 0 : 10000,
          });
        } else if (willComplete) {
          setSuccessModal({
            isOpen: true,
            type: 'NORMAL',
            onClose: handleComplete_cb,
            autoCloseDelay: order.is_retail ? 0 : 10000,
          });
        }

        if (!willComplete) {
          setAmountSplitValue('');
        }
        return true;
      } catch (err) {
        logger.error('Amount split failed', err);
        toast.error(`${t('checkout.amount_split.failed')}: ${err}`);
        return false;
      } finally {
        setIsProcessingAmountSplit(false);
      }
    },
    [order, isProcessingAmountSplit, amountSplitValue, remaining, t, splitMode, aaPayStr, aaTotalStr, isAALocked, handleComplete_cb, serviceType]
  );

  const handleConfirmAmountSplitCash = useCallback(
    async (tenderedAmount: number) => {
      const success = await handleAmountSplitPayment('CASH', { tendered: tenderedAmount });
      if (success) {
        setShowCashModal(false);
      }
    },
    [handleAmountSplitPayment]
  );

  const parsedAmount = parseFloat(amountSplitValue) || 0;

  const totalShares = isAALocked ? order.aa_total_shares! : (parseInt(aaTotalStr) || 1);
  const paidSharesExact = isAALocked ? (order.aa_paid_shares ?? 0) : (() => {
    const sharePrice = Currency.div(order.total, totalShares).toNumber();
    return Math.abs(sharePrice) < 0.01 ? 0 : Currency.div(order.paid_amount, sharePrice).toNumber();
  })();
  const remainingSharesExact = isAALocked ? (totalShares - paidSharesExact) : (() => {
    const sharePrice = Currency.div(order.total, totalShares).toNumber();
    return Math.abs(sharePrice) < 0.01 ? 0 : Currency.div(remaining, sharePrice).toNumber();
  })();

  const formatShareCount = (val: number) => {
    const rounded = Math.round(val * 100) / 100;
    return rounded % 1 === 0 ? rounded.toFixed(0) : rounded.toFixed(1);
  };

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

        <div className="flex-1 flex flex-col relative overflow-hidden">
           {/* Background Decor */}
           <div className="absolute top-[-10%] right-[-10%] w-96 h-96 bg-cyan-200 rounded-full mix-blend-multiply filter blur-3xl opacity-30 animate-blob"></div>
           <div className="absolute bottom-[-10%] left-[20%] w-80 h-80 bg-teal-200 rounded-full mix-blend-multiply filter blur-3xl opacity-30 animate-blob animation-delay-4000"></div>

          {/* Header */}
          <div className="p-6 bg-white/80 backdrop-blur-md border-b border-gray-200/50 shadow-sm flex justify-between items-center z-10 shrink-0">
            <h3 className="font-bold text-gray-800 text-2xl flex items-center gap-3">
              <div className="p-2 bg-cyan-600 rounded-xl text-white shadow-lg shadow-cyan-500/30">
                <Banknote size={24} />
              </div>
              {isAALocked ? t('checkout.aa_split.title') : t('checkout.amount_split.title')}
            </h3>
            <button onClick={onBack}
              className="px-5 py-2.5 bg-white border border-gray-200 hover:bg-gray-50 hover:border-gray-300 text-gray-700 rounded-xl font-medium flex items-center gap-2 transition-all shadow-sm"
            >
              <ArrowLeft size={20} /> {t('common.action.back')}
            </button>
          </div>

          <div className="flex-1 flex overflow-hidden z-10 p-6 gap-6">
            {/* Left Column: Split Configuration */}
            <div className="flex-1 flex flex-col gap-6">

                {/* Info Card */}
                <div className="bg-white rounded-2xl p-6 shadow-sm border border-gray-100">
                    <div className="flex items-center justify-between">
                        <div>
                            <div className="text-gray-500 font-medium mb-1">{t('checkout.split.total')}</div>
                            <div className="text-3xl font-bold text-gray-900">{formatCurrency(order.total)}</div>
                        </div>
                        {(splitMode === 'AA' || isAALocked) && (
                        <div className="flex flex-col gap-2 text-sm">
                            <div className="flex items-center gap-2 text-gray-600 bg-green-50 px-3 py-2 rounded-lg border border-green-100">
                                 <div className="p-1 bg-green-200 text-green-700 rounded-full"><Check size={12} strokeWidth={3} /></div>
                                 <span>{t('checkout.aa_split.paid_shares')}: <b className="text-green-700">{formatShareCount(paidSharesExact)}</b> {t('checkout.aa_split.shares_unit')}</span>
                            </div>
                            <div className="flex items-center gap-2 text-gray-600 bg-orange-50 px-3 py-2 rounded-lg border border-orange-100">
                                 <div className="p-1 bg-orange-200 text-orange-700 rounded-full"><Clock size={12} strokeWidth={3} /></div>
                                 <span>{t('checkout.aa_split.unpaid_shares')}: <b className="text-orange-700">{formatShareCount(remainingSharesExact)}</b> {t('checkout.aa_split.shares_unit')}</span>
                            </div>
                        </div>
                        )}
                        <div className="text-right">
                            <div className="text-gray-500 font-medium mb-1">{t('checkout.payment.remaining')}</div>
                            <div className="text-3xl font-bold text-orange-600">{formatCurrency(remaining)}</div>
                        </div>
                    </div>
                </div>

                {/* Split Controls */}
                <div className="flex-1 bg-white rounded-2xl shadow-sm border border-gray-100 p-6 flex flex-col gap-6">

                    {/* Split Into (Total Shares) */}
                    <div
                        onClick={() => !isAALocked && handleFocus('AA_TOTAL')}
                        className={`
                            relative p-6 rounded-2xl border-2 transition-all flex items-center justify-between
                            ${isAALocked
                                ? 'border-purple-300 bg-purple-50/50 cursor-not-allowed opacity-90'
                                : activeInput === 'AA_TOTAL'
                                    ? 'border-purple-500 bg-purple-50 ring-4 ring-purple-100 cursor-pointer'
                                    : 'border-gray-200 hover:border-purple-200 hover:bg-gray-50 opacity-80 cursor-pointer'
                            }
                        `}
                    >
                        <div className="flex items-center gap-4">
                            <div className={`p-3 rounded-xl ${isAALocked ? 'bg-purple-200 text-purple-700' : activeInput === 'AA_TOTAL' ? 'bg-purple-200 text-purple-700' : 'bg-gray-100 text-gray-500'}`}>
                                {isAALocked ? <LockIcon size={24} /> : <Users size={24} />}
                            </div>
                            <div>
                                <div className="text-sm font-bold text-gray-500 uppercase tracking-wider">{t('checkout.aa_split.split_into')}</div>
                                <div className="text-sm text-gray-400">{isAALocked ? t('checkout.aa_split.locked') : t('checkout.aa_split.total_shares')}</div>
                            </div>
                        </div>
                        <div className="flex items-center gap-6">
                            {!isAALocked && activeInput === 'AA_TOTAL' && (
                                <button
                                    onClick={(e) => { e.stopPropagation(); incrementAA('TOTAL', -1); }}
                                    className="w-12 h-12 rounded-xl bg-white border border-gray-200 flex items-center justify-center hover:bg-gray-50 text-gray-600 shadow-sm"
                                >
                                    <Minus size={20} />
                                </button>
                            )}
                            <div className={`text-4xl font-bold tabular-nums w-20 text-center ${isAALocked ? 'text-purple-700' : activeInput === 'AA_TOTAL' ? 'text-gray-800' : 'text-gray-500'}`}>
                                {isAALocked ? order.aa_total_shares : (aaTotalStr || <span className="text-gray-300">1</span>)}
                            </div>
                            {!isAALocked && activeInput === 'AA_TOTAL' && (
                                <button
                                    onClick={(e) => { e.stopPropagation(); incrementAA('TOTAL', 1); }}
                                    className="w-12 h-12 rounded-xl bg-white border border-gray-200 flex items-center justify-center hover:bg-gray-50 text-gray-600 shadow-sm"
                                >
                                    <Plus size={20} />
                                </button>
                            )}
                        </div>
                    </div>

                    {/* Pay For (Shares) */}
                    <div
                        onClick={() => handleFocus('AA_PAY')}
                        className={`
                            relative p-6 rounded-2xl border-2 transition-all cursor-pointer flex items-center justify-between
                            ${activeInput === 'AA_PAY'
                                ? 'border-orange-500 bg-orange-50 ring-4 ring-orange-100'
                                : 'border-gray-200 hover:border-orange-200 hover:bg-gray-50 opacity-80'
                            }
                        `}
                    >
                        <div className="flex items-center gap-4">
                             <div className={`p-3 rounded-xl ${activeInput === 'AA_PAY' ? 'bg-orange-200 text-orange-700' : 'bg-gray-100 text-gray-500'}`}>
                                <PieChart size={24} />
                            </div>
                            <div>
                                <div className="text-sm font-bold text-gray-500 uppercase tracking-wider">{t('checkout.aa_split.pay_for')}</div>
                                <div className="text-sm text-gray-400">{t('checkout.aa_split.your_shares')}</div>
                            </div>
                        </div>
                        <div className="flex items-center gap-6">
                            {activeInput === 'AA_PAY' && (
                                <button
                                    onClick={(e) => { e.stopPropagation(); incrementAA('PAY', -1); }}
                                    className="w-12 h-12 rounded-xl bg-white border border-gray-200 flex items-center justify-center hover:bg-gray-50 text-gray-600 shadow-sm"
                                >
                                    <Minus size={20} />
                                </button>
                            )}
                            <div className={`text-4xl font-bold tabular-nums w-20 text-center ${activeInput === 'AA_PAY' ? 'text-gray-800' : 'text-gray-500'}`}>
                                {aaPayStr || <span className="text-gray-300">0</span>}
                            </div>
                            {activeInput === 'AA_PAY' && (
                                <button
                                    onClick={(e) => { e.stopPropagation(); incrementAA('PAY', 1); }}
                                    className="w-12 h-12 rounded-xl bg-white border border-gray-200 flex items-center justify-center hover:bg-gray-50 text-gray-600 shadow-sm"
                                >
                                    <Plus size={20} />
                                </button>
                            )}
                        </div>
                    </div>

                    {/* Result Summary */}
                    <div className="mt-auto p-6 bg-gray-50 rounded-xl border border-gray-200 flex flex-col gap-2">
                        <div className="flex justify-between items-center text-gray-500">
                            <span>{t('checkout.aa_split.calculation')}</span>
                            <span className="text-sm">
                                {splitMode === 'AA'
                                    ? `(${formatCurrency(remaining)} / ${isAALocked ? aaRemainingShares : (parseInt(aaTotalStr)||1)}) × ${parseInt(aaPayStr)||1}`
                                    : t('checkout.amount_split.custom_amount')
                                }
                            </span>
                        </div>
                        <div className="flex justify-between items-end">
                            <span className="text-lg font-bold text-gray-700">{t('checkout.aa_split.amount_to_pay')}</span>
                            <span className="text-4xl font-bold text-purple-600 tabular-nums">
                                {formatCurrency(parsedAmount)}
                            </span>
                        </div>
                    </div>

                </div>
            </div>

            {/* Right Column: Numpad & Actions */}
            <div className="w-[380px] flex flex-col gap-6">

                {/* Custom Amount Display */}
                <div
                    onClick={() => handleFocus('CUSTOM')}
                    className={`
                        p-6 bg-white rounded-2xl shadow-sm border-2 transition-all flex flex-col items-end justify-center min-h-[120px]
                        ${isAALocked
                            ? 'border-gray-200 bg-gray-50 cursor-not-allowed opacity-60'
                            : activeInput === 'CUSTOM'
                                ? 'border-blue-500 bg-blue-50 ring-4 ring-blue-100 cursor-pointer'
                                : 'border-gray-200 hover:border-blue-200 hover:bg-gray-50 cursor-pointer'
                        }
                    `}
                >
                    <div className="text-sm font-bold text-gray-400 uppercase mb-2">
                        {isAALocked ? t('checkout.aa_split.title') : t('checkout.amount_split.custom_amount')}
                    </div>
                    <div className="text-5xl font-bold text-gray-800 tabular-nums break-all text-right w-full">
                        {activeInput === 'CUSTOM' && !isAALocked
                            ? (amountSplitValue || <span className="text-gray-300">0.00</span>)
                            : <span className="text-gray-400">{amountSplitValue || '0.00'}</span>
                        }
                    </div>
                </div>

                {/* Numpad */}
                <div className="flex-1 bg-white rounded-2xl shadow-sm border border-gray-100 p-4">
                    <Numpad
                        onNumber={handleNumpadInput}
                        onDelete={() => handleNumpadInput('backspace')}
                        onClear={() => handleNumpadInput('C')}
                        onEnter={() => {
                            if (parsedAmount > 0) {
                                setShowCashModal(true);
                            }
                        }}
                        showEnter={false}
                        className="h-full"
                    />
                </div>

                {/* Payment Buttons */}
                <div className="grid grid-cols-2 gap-4">
                    <button
                        onClick={() => setShowCashModal(true)}
                        disabled={parsedAmount <= 0 || isProcessingAmountSplit}
                        className="py-4 bg-emerald-500 hover:bg-emerald-600 text-white rounded-xl font-bold text-lg shadow-lg shadow-emerald-500/30 hover:shadow-xl transition-all disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
                    >
                        <Banknote size={20} />
                        {t('checkout.split.pay_cash')}
                    </button>
                    <button
                        onClick={() => handleAmountSplitPayment('CARD')}
                        disabled={parsedAmount <= 0 || isProcessingAmountSplit}
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

      <CashPaymentModal
        isOpen={showCashModal}
        amountDue={parsedAmount}
        isProcessing={isProcessingAmountSplit}
        onConfirm={handleConfirmAmountSplitCash}
        onCancel={() => setShowCashModal(false)}
      />
    </>
  );
};
