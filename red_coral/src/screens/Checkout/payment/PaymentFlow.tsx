import React, { useState, useCallback, useMemo } from 'react';
import { HeldOrder, PaymentRecord } from '@/core/domain/types';
import { Coins, CreditCard, ArrowLeft, Printer, Trash2, Split, Minus, Plus, Banknote, Utensils, ShoppingBag, Receipt, Users, Calculator, PieChart, X, Lock as LockIcon, Check, Clock, Gift, Percent, TrendingUp, ClipboardList, ChevronRight } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { Permission } from '@/core/domain/types';
import { useRetailServiceType, setRetailServiceType, toBackendServiceType } from '@/core/stores/order/useCheckoutStore';
import { CompItemMode } from '../CompItemMode';
import { OrderDetailMode } from '../OrderDetailMode';
import { OrderDiscountModal } from '../OrderDiscountModal';
import { OrderSurchargeModal } from '../OrderSurchargeModal';
import { formatCurrency } from '@/utils/currency';
import { Currency } from '@/utils/currency';

// Services & Operations
import { openCashDrawer } from '@/core/services/order/paymentService';
import { completeOrder, splitByItems, splitByAmount, startAaSplit, payAaSplit, updateOrderInfo } from '@/core/stores/order/useOrderOperations';
import { useOrderCommands } from '@/core/stores/order/useOrderCommands';

// Components
import { CashPaymentModal } from './CashPaymentModal';
import { PaymentSuccessModal } from './PaymentSuccessModal';
import { OrderSidebar } from '@/presentation/components/OrderSidebar';
import { ConfirmDialog } from '@/shared/components';
import { SplitItemRow } from '../components';
import { Numpad } from '@/presentation/components/ui/Numpad';
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

type PaymentMode = 'SELECT' | 'ITEM_SPLIT' | 'AMOUNT_SPLIT' | 'PAYMENT_RECORDS' | 'COMP' | 'ORDER_DETAIL';

export const PaymentFlow: React.FC<PaymentFlowProps> = ({ order, onComplete, onCancel, onVoid, onManageTable }) => {
  const { t } = useI18n();
  const serviceType = useRetailServiceType();
  const { cancelPayment } = useOrderCommands();

  // Calculate payment state from order (server state)
  const totalPaid = order.paid_amount;
  const remaining = Math.max(0, Currency.sub(order.total, totalPaid).toNumber());
  const isPaidInFull = remaining === 0;

  // Get active (non-cancelled) payments
  const activePayments = useMemo(() => {
    return [...(order.payments || [])]
      .filter(p => !p.cancelled)
      .sort((a, b) => b.timestamp - a.timestamp); // Sort by time desc
  }, [order.payments]);

  // Local State
  const [mode, setMode] = useState<PaymentMode>('SELECT');
  const [cancellingPaymentId, setCancellingPaymentId] = useState<string | null>(null);
  const [cancelConfirm, setCancelConfirm] = useState<{ paymentId: string; isSplit: boolean } | null>(null);
  const [cancelAuthorizer, setCancelAuthorizer] = useState<{ id: number; name: string } | null>(null);
  const [isProcessing, setIsProcessing] = useState(false);
  const [showCashModal, setShowCashModal] = useState(false);
  const [paymentContext, setPaymentContext] = useState<'FULL' | 'SPLIT' | 'AMOUNT_SPLIT'>('FULL');
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

  // Amount Split State (金额分单)
  const [amountSplitValue, setAmountSplitValue] = useState<string>('');
  const [isProcessingAmountSplit, setIsProcessingAmountSplit] = useState(false);
  
  // AA / Custom Split Logic
  // Server-authoritative: once AA is locked, aa_total_shares comes from server
  const isAALocked = !!(order.aa_total_shares && order.aa_total_shares > 0);
  const aaRemainingShares = isAALocked ? (order.aa_total_shares! - (order.aa_paid_shares ?? 0)) : 0;
  const [splitMode, setSplitMode] = useState<'CUSTOM' | 'AA'>('CUSTOM');
  const [activeInput, setActiveInput] = useState<'CUSTOM' | 'AA_TOTAL' | 'AA_PAY'>('CUSTOM');
  const replaceMode = React.useRef(true); // First keypress after focus clears the field
  const [aaTotalStr, setAATotalStr] = useState<string>(String(order.guest_count || 2));
  const [aaPayStr, setAAPayStr] = useState<string>('1');

  // Auto-enter AA mode: when server has AA locked, or when entering AMOUNT_SPLIT
  React.useEffect(() => {
    if (isAALocked) {
      setSplitMode('AA');
      setActiveInput('AA_PAY');
      replaceMode.current = true;
      setAATotalStr(order.aa_total_shares!.toString());
      // Clamp aaPayStr to remaining shares
      const pay = parseInt(aaPayStr) || 1;
      const maxPay = aaRemainingShares;
      if (pay > maxPay && maxPay > 0) {
        setAAPayStr(maxPay.toString());
      }
    } else if (mode === 'AMOUNT_SPLIT') {
      // First time entering AA: focus on total shares
      setSplitMode('AA');
      setActiveInput('AA_TOTAL');
      replaceMode.current = true;
    }
  }, [isAALocked, order.aa_total_shares, aaRemainingShares, mode]);

  // Sync amountSplitValue with AA state
  React.useEffect(() => {
    if (splitMode === 'AA') {
      const total = isAALocked ? order.aa_total_shares! : (parseInt(aaTotalStr) || 1);
      const remainingSharesForCalc = isAALocked ? aaRemainingShares : total;
      let pay = parseInt(aaPayStr) || 0;

      // Ensure pay doesn't exceed remaining shares
      if (pay > remainingSharesForCalc) {
        pay = remainingSharesForCalc;
        setAAPayStr(remainingSharesForCalc.toString());
      }

      // Calculate: (remaining / remaining_shares) * pay_shares
      const amount = remainingSharesForCalc > 0
        ? Currency.mul(Currency.div(remaining, remainingSharesForCalc), pay).toDecimalPlaces(2).toNumber()
        : 0;
      setAmountSplitValue(amount.toFixed(2));
    }
  }, [splitMode, aaTotalStr, aaPayStr, remaining, isAALocked, aaRemainingShares, order.aa_total_shares]);

  // Numpad Handler
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

    // Replace mode: first keypress after focus clears the field
    const shouldReplace = replaceMode.current;
    if (shouldReplace) replaceMode.current = false;

    // Handle numeric input
    if (activeInput === 'CUSTOM') {
      // Allow decimals for amount
      const base = shouldReplace ? '' : amountSplitValue;
      if (value === '.' && base.includes('.')) return;
      const next = base + value;
      if (next.includes('.') && next.split('.')[1].length > 2) return;
      setAmountSplitValue(next);
    } else {
      // Integer only for counts
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

  // Handle Input Focus
  const handleFocus = (field: 'CUSTOM' | 'AA_TOTAL' | 'AA_PAY') => {
    // When AA is locked, prevent switching to CUSTOM mode
    if (field === 'CUSTOM' && isAALocked) return;
    setActiveInput(field);
    replaceMode.current = true; // First keypress will replace existing value
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

  // Retail order cancel confirmation
  const [showRetailCancelConfirm, setShowRetailCancelConfirm] = useState(false);

  // Order adjustment modals
  const [showDiscountModal, setShowDiscountModal] = useState(false);
  const [showSurchargeModal, setShowSurchargeModal] = useState(false);

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
      const response = await cancelPayment(order.order_id, paymentId, undefined, cancelAuthorizer ?? undefined);
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
      setCancelAuthorizer(null);
    }
  }, [cancelConfirm, order.order_id, cancelPayment, cancelAuthorizer, t]);

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
        // receipt_number is server-generated at OpenTable
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
      // receipt_number is server-generated at OpenTable
      await completeOrder(order.order_id, [payment], order.is_retail ? toBackendServiceType(serviceType) : null);
      const is_retail = order.is_retail;

      setSuccessModal({
        isOpen: true,
        type: 'NORMAL',
        onClose: handleComplete,
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
        toast.error(t('checkout.error.no_receipt_number'));
        return;
      }

      // receipt_number is immutable (set at OpenTable), no need to pass
      await updateOrderInfo(order.order_id, {
        is_pre_payment: true,
      });

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

        // Server calculates amount from items (server-authoritative)
        // Fire & forget - UI updates via WebSocket
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

        // Check if this split covers the remaining amount → auto-complete
        const willComplete = Currency.sub(remaining, total).toNumber() <= 0.01;

        if (willComplete) {
          await completeOrder(order.order_id, [], order.is_retail ? toBackendServiceType(serviceType) : null);
        }

        // Show success modal for cash payments
        if (method === 'CASH' && cashDetails?.tendered !== undefined) {
          setSuccessModal({
            isOpen: true,
            type: 'CASH',
            change: cashDetails.tendered - total,
            onClose: willComplete ? handleComplete : () => setSuccessModal(null),
            autoCloseDelay: willComplete && order.is_retail ? 0 : 10000,
          });
        } else if (willComplete) {
          // Card split that completes the order
          setSuccessModal({
            isOpen: true,
            type: 'NORMAL',
            onClose: handleComplete,
            autoCloseDelay: order.is_retail ? 0 : 10000,
          });
        }

        if (!willComplete) {
          setMode('ITEM_SPLIT');
          setSplitItems({});
        }
        return true;
      } catch (err) {
        console.error('Split failed:', err);
        toast.error(`${t('checkout.split.failed')}: ${err}`);
        return false;
      } finally {
        setIsProcessingSplit(false);
      }
    },
    [order, isProcessingSplit, splitItems, remaining, t, handleComplete]
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



  /**
   * 金额分单支付处理
   */
  const handleAmountSplitPayment = useCallback(
    async (method: 'CASH' | 'CARD', cashDetails?: { tendered: number }) => {
      if (!order || isProcessingAmountSplit) return false;

      const amount = parseFloat(amountSplitValue);
      if (isNaN(amount) || amount <= 0) {
        toast.error(t('checkout.amount_split.invalid_amount'));
        return false;
      }

      if (amount > remaining + 0.01) {
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
          // AA split: server calculates amount from shares
          const payShares = parseInt(aaPayStr) || 1;
          if (isAALocked) {
            // Subsequent AA payment
            await payAaSplit(order.order_id, payShares, method, tendered);
          } else {
            // First AA payment — lock headcount + pay
            const totalShares = parseInt(aaTotalStr) || 2;
            await startAaSplit(order.order_id, totalShares, payShares, method, tendered);
          }
        } else {
          // Amount-based split
          await splitByAmount(order.order_id, amount, method, tendered);
        }

        // Check if this split covers the remaining amount → auto-complete
        const willComplete = Currency.sub(remaining, amount).toNumber() <= 0.01;

        if (willComplete) {
          await completeOrder(order.order_id, [], order.is_retail ? toBackendServiceType(serviceType) : null);
        }

        // Show success modal for cash payments
        if (method === 'CASH' && cashDetails?.tendered !== undefined) {
          setSuccessModal({
            isOpen: true,
            type: 'CASH',
            change: cashDetails.tendered - amount,
            onClose: willComplete ? handleComplete : () => setSuccessModal(null),
            autoCloseDelay: willComplete && order.is_retail ? 0 : 10000,
          });
        } else if (willComplete) {
          // Card split that completes the order
          setSuccessModal({
            isOpen: true,
            type: 'NORMAL',
            onClose: handleComplete,
            autoCloseDelay: order.is_retail ? 0 : 10000,
          });
        }

        if (!willComplete) {
          setAmountSplitValue('');
        }
        return true;
      } catch (err) {
        console.error('Amount split failed:', err);
        toast.error(`${t('checkout.amount_split.failed')}: ${err}`);
        return false;
      } finally {
        setIsProcessingAmountSplit(false);
      }
    },
    [order, isProcessingAmountSplit, amountSplitValue, remaining, t, splitMode, aaPayStr, aaTotalStr, isAALocked, handleComplete]
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

  const handleCashModalConfirm = useCallback(
    (tenderedAmount: number) => {
      if (paymentContext === 'FULL') {
        handleConfirmFullCash(tenderedAmount);
      } else if (paymentContext === 'AMOUNT_SPLIT') {
        handleConfirmAmountSplitCash(tenderedAmount);
      } else {
        handleConfirmSplitCash(tenderedAmount);
      }
    },
    [paymentContext, handleConfirmFullCash, handleConfirmSplitCash, handleConfirmAmountSplitCash]
  );

  const splitTotal = useMemo(() => {
    if (!order) return 0;
    let total = 0;
    (Object.entries(splitItems) as [string, number][]).forEach(([instanceId, qty]) => {
      const item = order.items.find(i => i.instance_id === instanceId);
      if (item) {
        // Use server-authoritative unit_price
        const unitPrice = item.unit_price;
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
        category: product?.category_id != null ? String(product.category_id) : undefined,
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
      // Skip comped items (they are free, cannot be split)
      if (item.is_comped) return;
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
      const category = categories.find(c => c.id === Number(categoryRef));
      result.push({
        categoryId: categoryRef,
        categoryName: category?.name || t('common.label.unknown_item'),
        items,
      });
    });

    // Sort items within each group by external_id (consistent with OrderItemsSummary)
    const extIdMap = new Map(products.map(p => [p.id, p.external_id]));
    for (const group of result) {
      group.items.sort((a, b) => {
        const extA = extIdMap.get(a.id) ?? Number.MAX_SAFE_INTEGER;
        const extB = extIdMap.get(b.id) ?? Number.MAX_SAFE_INTEGER;
        if (extA !== extB) return extA - extB;
        return a.name.localeCompare(b.name);
      });
    }

    // Sort groups by category sort_order (consistent with OrderItemsSummary)
    const categoryMap = new Map(categories.map(c => [c.id, c]));
    result.sort((a, b) => {
      const sortA = a.categoryId ? (categoryMap.get(Number(a.categoryId))?.sort_order ?? 0) : Number.MAX_SAFE_INTEGER;
      const sortB = b.categoryId ? (categoryMap.get(Number(b.categoryId))?.sort_order ?? 0) : Number.MAX_SAFE_INTEGER;
      return sortA - sortB;
    });

    // Add uncategorized at the end
    if (uncategorized.length > 0) {
      result.push({
        categoryId: null,
        categoryName: t('common.label.unknown_item'),
        items: uncategorized,
      });
    }

    return result;
  }, [order.items, order.paid_item_quantities, productInfoMap, categories, products, t]);

  // Filter logic for Split Mode UI
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

  const renderSplitMode = () => {
    return (
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
            <button onClick={() => { setMode('SELECT'); setSplitItems({}); }} className="px-5 py-2.5 bg-white border border-gray-200 hover:bg-gray-50 hover:border-gray-300 text-gray-700 rounded-xl font-medium flex items-center gap-2 transition-all shadow-sm">
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
                                                      {isFullySelected && <div className="absolute inset-0 bg-black/40 flex items-center justify-center"><div className="text-white text-xs font-bold">ALL</div></div>}
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
                                  // Sort by category sort_order, then external_id, then name
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
                              onClick={() => {
                                  setPaymentContext('SPLIT');
                                  setShowCashModal(true);
                              }}
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
    );
  };

  const renderAmountSplitMode = () => {
    const parsedAmount = parseFloat(amountSplitValue) || 0;

    // Calculate Share Stats - use server AA state when locked
    const totalShares = isAALocked ? order.aa_total_shares! : (parseInt(aaTotalStr) || 1);
    const paidSharesExact = isAALocked ? (order.aa_paid_shares ?? 0) : (() => {
      const sharePrice = order.total / totalShares;
      const paidAmount = order.total - remaining;
      return Math.abs(sharePrice) < 0.01 ? 0 : paidAmount / sharePrice;
    })();
    const remainingSharesExact = isAALocked ? (totalShares - paidSharesExact) : (() => {
      const sharePrice = order.total / totalShares;
      return Math.abs(sharePrice) < 0.01 ? 0 : remaining / sharePrice;
    })();

    // Helper to format shares (e.g. 1, 1.5, 0.3)
    const formatShareCount = (val: number) => {
        const rounded = Math.round(val * 100) / 100;
        return rounded % 1 === 0 ? rounded.toFixed(0) : rounded.toFixed(1);
    };

    return (
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
            <button onClick={() => { 
                setMode('SELECT'); 
                setAmountSplitValue(''); 
                setSplitMode('CUSTOM');
                setActiveInput('CUSTOM');
              }} 
              className="px-5 py-2.5 bg-white border border-gray-200 hover:bg-gray-50 hover:border-gray-300 text-gray-700 rounded-xl font-medium flex items-center gap-2 transition-all shadow-sm"
            >
              <ArrowLeft size={20} /> {t('common.action.back')}
            </button>
          </div>

          <div className="flex-1 flex overflow-hidden z-10 p-6 gap-6">
            {/* Left Column: Split Configuration */}
            <div className="flex-1 flex flex-col gap-6">
                
                {/* Info Card */}
                <div className="bg-white rounded-2xl p-6 shadow-sm border border-gray-100 flex flex-col gap-4">
                    <div className="flex items-center justify-between border-b border-gray-100 pb-4">
                        <div>
                            <div className="text-gray-500 font-medium mb-1">{t('checkout.split.total')}</div>
                            <div className="text-3xl font-bold text-gray-900">{formatCurrency(order.total)}</div>
                        </div>
                        <div className="text-right">
                            <div className="text-gray-500 font-medium mb-1">{t('checkout.payment.remaining')}</div>
                            <div className="text-3xl font-bold text-orange-600">{formatCurrency(remaining)}</div>
                        </div>
                    </div>
                    
                    {/* Share Stats - only show when in AA mode or AA is locked */}
                    {(splitMode === 'AA' || isAALocked) && (
                    <div className="flex items-center justify-between text-sm">
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
                </div>

                {/* Split Controls */}
                <div className="flex-1 bg-white rounded-2xl shadow-sm border border-gray-100 p-6 flex flex-col gap-6">
                    
                    {/* Split Into (Total Shares) - locked after first AA payment */}
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
                
                {/* Custom Amount Display (Clickable) - disabled when AA locked */}
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
                                setPaymentContext('AMOUNT_SPLIT');
                                setShowCashModal(true);
                            }
                        }}
                        showEnter={false} // We have dedicated buttons below
                        className="h-full"
                    />
                </div>

                {/* Payment Buttons */}
                <div className="grid grid-cols-2 gap-4">
                    <button
                        onClick={() => {
                            setPaymentContext('AMOUNT_SPLIT');
                            setShowCashModal(true);
                        }}
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
              <button onClick={() => setMode('ITEM_SPLIT')} disabled={isPaidInFull || isProcessing || order.has_amount_split || isAALocked} className="h-40 bg-indigo-500 hover:bg-indigo-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <Split size={48} />
                <div className="text-2xl font-bold">{t('checkout.split.title')}</div>
                <div className="text-sm opacity-90">{isAALocked ? t('checkout.aa_split.locked') : order.has_amount_split ? t('checkout.amount_split.item_split_disabled') : t('checkout.split.desc')}</div>
              </button>
            </div>

            {/* Amount Split & Payment Records Buttons */}
            <div className="grid grid-cols-3 gap-6">
              <button onClick={() => setMode('AMOUNT_SPLIT')} disabled={isPaidInFull || isProcessing} className="h-40 bg-cyan-600 hover:bg-cyan-700 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4">
                <Banknote size={48} />
                <div className="text-2xl font-bold">{isAALocked ? t('checkout.aa_split.title') : t('checkout.amount_split.title')}</div>
                <div className="text-sm opacity-90">{isAALocked ? t('checkout.aa_split.desc') : t('checkout.amount_split.desc')}</div>
              </button>

              <button
                onClick={() => setMode('PAYMENT_RECORDS')}
                disabled={activePayments.length === 0}
                className="h-40 bg-teal-600 hover:bg-teal-700 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all disabled:opacity-50 disabled:cursor-not-allowed disabled:hover:scale-100 flex flex-col items-center justify-center gap-4"
              >
                <Receipt size={48} />
                <div className="text-2xl font-bold">{t('checkout.payment.records')}</div>
                <div className="text-sm opacity-90">{activePayments.length} {t('checkout.payment.record_count')} · {formatCurrency(totalPaid)}</div>
              </button>
              <button
                onClick={() => setMode('ORDER_DETAIL')}
                className="h-40 bg-amber-500 hover:bg-amber-600 text-white rounded-2xl shadow-xl hover:shadow-2xl hover:scale-[1.02] transition-all flex flex-col items-center justify-center gap-4"
              >
                <ClipboardList size={48} />
                <div className="text-2xl font-bold">{t('checkout.order_detail.title')}</div>
                <div className="text-sm opacity-90">{t('checkout.order_detail.desc')}</div>
              </button>
            </div>

            {/* Order Adjustments: Comp, Discount, Surcharge */}
            <div className="grid grid-cols-3 gap-6">
              <button
                onClick={() => setMode('COMP')}
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
              <Receipt size={24} className="text-teal-600" />
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
                const isSplit = Array.isArray(payment.split_items);
                const isAmountSplit = isSplit && payment.split_items!.length === 0;
                const isItemSplit = isSplit && payment.split_items!.length > 0;
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
                            {isAmountSplit && (
                              <span className="text-xs bg-amber-100 text-amber-600 px-2 py-0.5 rounded-full font-medium">
                                {t('checkout.amount_split.title')}
                              </span>
                            )}
                            {isItemSplit && (
                              <span className="text-xs bg-purple-100 text-purple-600 px-2 py-0.5 rounded-full font-medium">
                                {t('checkout.split.label')}
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
                            handleCancelPayment(payment.payment_id, isSplit);
                          }}
                        >
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
                        </EscalatableGate>
                      </div>
                    </div>

                    {/* Split Items Detail (collapsible, default collapsed) */}
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
    );
  };

  const renderContent = () => {
    switch (mode) {
      case 'SELECT':
        return renderSelectMode();
      case 'ITEM_SPLIT':
        return renderSplitMode();
      case 'AMOUNT_SPLIT':
        return renderAmountSplitMode();
      case 'PAYMENT_RECORDS':
        return renderPaymentRecordsMode();
      case 'COMP':
        return (
          <CompItemMode
            order={order}
            totalPaid={totalPaid}
            remaining={remaining}
            onBack={() => setMode('SELECT')}
            onManageTable={onManageTable}
          />
        );
      case 'ORDER_DETAIL':
        return (
          <OrderDetailMode
            order={order}
            totalPaid={totalPaid}
            remaining={remaining}
            onBack={() => setMode('SELECT')}
            onManageTable={onManageTable}
          />
        );
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
        amountDue={
          paymentContext === 'SPLIT' ? splitTotal :
          paymentContext === 'AMOUNT_SPLIT' ? (parseFloat(amountSplitValue) || 0) :
          remaining
        }
        isProcessing={isProcessing || isProcessingSplit || isProcessingAmountSplit}
        onConfirm={handleCashModalConfirm}
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
    </>
  );
};
