import { useState, useCallback } from 'react';
import { HeldOrder, PaymentRecord } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { useRetailServiceType, toBackendServiceType } from '@/core/stores/order/useCheckoutStore';
import { formatCurrency, Currency } from '@/utils/currency';
import { openCashDrawer } from '@/core/services/order/paymentService';
import { completeOrder, updateOrderInfo } from '@/core/stores/order/commands';

interface SuccessModalState {
  isOpen: boolean;
  type: 'NORMAL' | 'CASH';
  change?: number;
  onClose: () => void;
  onPrint?: () => void;
  autoCloseDelay: number;
}

export function usePaymentActions(order: HeldOrder, onComplete: () => void) {
  const { t } = useI18n();
  const serviceType = useRetailServiceType();
  const remaining = order.remaining_amount;

  const [isProcessing, setIsProcessing] = useState(false);
  const [showCashModal, setShowCashModal] = useState(false);
  const [successModal, setSuccessModal] = useState<SuccessModalState | null>(null);

  const handleComplete = useCallback(() => {
    requestAnimationFrame(() => {
      onComplete();
    });
  }, [onComplete]);

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

  return {
    isProcessing,
    showCashModal,
    setShowCashModal,
    successModal,
    serviceType,
    handleManualComplete,
    handleFullCashPayment,
    handleConfirmFullCash,
    handleFullCardPayment,
    handlePrintPrePayment,
  };
}
