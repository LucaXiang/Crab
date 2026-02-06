import React, { useState, useEffect, useMemo, useCallback } from 'react';
import { HeldOrder, Permission } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { X, Percent, Tag } from 'lucide-react';
import { formatCurrency } from '@/utils/currency';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { applyOrderDiscount } from '@/core/stores/order/useOrderOperations';
import { toast } from '@/presentation/components/Toast';
import { Numpad } from '@/presentation/components/ui/Numpad';

type DiscountType = 'percent' | 'fixed';

const QUICK_PERCENT_VALUES = [5, 10, 15, 20];

interface OrderDiscountModalProps {
  isOpen: boolean;
  order: HeldOrder;
  onClose: () => void;
}

export const OrderDiscountModal: React.FC<OrderDiscountModalProps> = ({
  isOpen,
  order,
  onClose,
}) => {
  const { t } = useI18n();

  const hasExistingDiscount = !!(order.order_manual_discount_percent || order.order_manual_discount_fixed);

  const [discountType, setDiscountType] = useState<DiscountType>(
    order.order_manual_discount_fixed ? 'fixed' : 'percent'
  );
  const [value, setValue] = useState('');
  const [isSelected, setIsSelected] = useState(true);
  const [reason, setReason] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);

  // Reset state only when modal opens (not on order WebSocket updates)
  useEffect(() => {
    if (isOpen) {
      if (order.order_manual_discount_fixed) {
        setDiscountType('fixed');
        setValue(String(order.order_manual_discount_fixed));
      } else if (order.order_manual_discount_percent) {
        setDiscountType('percent');
        setValue(String(order.order_manual_discount_percent));
      } else {
        setDiscountType('percent');
        setValue('');
      }
      setIsSelected(true);
      setReason('');
      setIsProcessing(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);

  const parsedValue = parseFloat(value) || 0;

  const canConfirm = useMemo(() => {
    if (isProcessing) return false;
    if (parsedValue <= 0) return false;
    if (discountType === 'percent' && parsedValue > 100) return false;
    return true;
  }, [isProcessing, parsedValue, discountType]);

  const handleNumPress = useCallback((num: string) => {
    if (isSelected) {
      setIsSelected(false);
      setValue(num === '.' ? '0.' : num);
      return;
    }
    setValue((prev) => {
      if (num === '.' && prev.includes('.')) return prev;
      if (prev.includes('.') && prev.split('.')[1].length >= 2) return prev;
      if (prev === '0' && num !== '.') return num;
      return prev + num;
    });
  }, [isSelected]);

  const handleDelete = useCallback(() => {
    setIsSelected(false);
    setValue((prev) => {
      const newVal = prev.slice(0, -1);
      return newVal || '0';
    });
  }, []);

  const handleNumpadClear = useCallback(() => {
    setValue('0');
    setIsSelected(true);
  }, []);

  const handleQuickValue = useCallback((val: number) => {
    setValue(String(val));
    setIsSelected(false);
  }, []);

  const handleTypeChange = useCallback((type: DiscountType) => {
    setDiscountType(type);
    setValue('');
    setIsSelected(true);
  }, []);

  const handleApply = async (authorizer?: { id: string; name: string }) => {
    if (!canConfirm) return;
    setIsProcessing(true);
    try {
      await applyOrderDiscount(order.order_id, {
        discountPercent: discountType === 'percent' ? parsedValue : undefined,
        discountFixed: discountType === 'fixed' ? parsedValue : undefined,
        reason: reason.trim() || undefined,
        authorizer,
      });
      toast.success(t('checkout.order_discount.title'));
      onClose();
    } catch (err) {
      console.error('Apply order discount failed:', err);
      toast.error(String(err));
    } finally {
      setIsProcessing(false);
    }
  };

  const handleClear = async (authorizer?: { id: string; name: string }) => {
    setIsProcessing(true);
    try {
      await applyOrderDiscount(order.order_id, { authorizer });
      toast.success(t('checkout.order_discount.clear'));
      onClose();
    } catch (err) {
      console.error('Clear order discount failed:', err);
      toast.error(String(err));
    } finally {
      setIsProcessing(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-80 bg-black/50 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-lg w-full overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="shrink-0 px-6 py-4 border-b border-gray-100 flex justify-between items-center bg-orange-50">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-orange-100 rounded-full text-orange-600">
              <Percent size={24} />
            </div>
            <h2 className="text-xl font-bold text-gray-800">{t('checkout.order_discount.title')}</h2>
          </div>
          <button onClick={onClose} className="p-2 hover:bg-orange-100 rounded-full transition-colors text-gray-500">
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-5 overflow-y-auto space-y-4">
          {/* Order Total Display */}
          <div className="bg-gray-50 rounded-xl p-3">
            <div className="flex justify-between text-sm">
              <span className="text-gray-600">{t('checkout.amount.total')}</span>
              <span className="font-medium text-gray-900">{formatCurrency(order.original_total)}</span>
            </div>
          </div>

          {/* Discount Type Selection */}
          <div className="grid grid-cols-2 gap-3">
            <button
              onClick={() => handleTypeChange('percent')}
              className={`p-3 rounded-xl border-2 text-left transition-all flex items-center gap-3 ${
                discountType === 'percent'
                  ? 'border-orange-500 bg-orange-50'
                  : 'border-gray-100 hover:border-orange-200 hover:bg-gray-50'
              }`}
            >
              <div className={`p-2 rounded-lg ${discountType === 'percent' ? 'bg-orange-100 text-orange-600' : 'bg-gray-100 text-gray-500'}`}>
                <Percent size={20} />
              </div>
              <span className={`font-medium ${discountType === 'percent' ? 'text-orange-700' : 'text-gray-700'}`}>
                {t('checkout.order_discount.type_percent')}
              </span>
            </button>
            <button
              onClick={() => handleTypeChange('fixed')}
              className={`p-3 rounded-xl border-2 text-left transition-all flex items-center gap-3 ${
                discountType === 'fixed'
                  ? 'border-orange-500 bg-orange-50'
                  : 'border-gray-100 hover:border-orange-200 hover:bg-gray-50'
              }`}
            >
              <div className={`p-2 rounded-lg ${discountType === 'fixed' ? 'bg-orange-100 text-orange-600' : 'bg-gray-100 text-gray-500'}`}>
                <Tag size={20} />
              </div>
              <span className={`font-medium ${discountType === 'fixed' ? 'text-orange-700' : 'text-gray-700'}`}>
                {t('checkout.order_discount.type_fixed')}
              </span>
            </button>
          </div>

          {/* Value Display */}
          <div className="space-y-2">
            <div className="h-14 bg-white rounded-xl flex items-center justify-center px-4 border-2 border-orange-200 shadow-sm">
              <div className="flex items-center">
                {discountType === 'fixed' && (
                  <span className="text-orange-500 mr-2 text-xl font-bold">€</span>
                )}
                <span
                  className={`text-3xl font-mono font-bold px-2 rounded transition-colors ${
                    isSelected ? 'bg-orange-500 text-white' : 'text-gray-800'
                  }`}
                >
                  {value || '0'}
                </span>
                {discountType === 'percent' && (
                  <span className="text-orange-500 ml-1 text-xl font-bold">%</span>
                )}
                {!isSelected && (
                  <span className="animate-pulse ml-0.5 w-0.5 h-7 bg-orange-400 rounded" />
                )}
              </div>
            </div>
            {discountType === 'percent' && parsedValue > 100 && (
              <p className="text-sm text-red-500 text-center">0-100%</p>
            )}
          </div>

          {/* Quick Value Buttons */}
          {discountType === 'percent' && (
            <div className="grid grid-cols-4 gap-2">
              {QUICK_PERCENT_VALUES.map((val) => (
                <button
                  key={val}
                  onClick={() => handleQuickValue(val)}
                  disabled={isProcessing}
                  className="h-10 bg-orange-50 border border-orange-200 text-orange-600 font-semibold rounded-lg hover:bg-orange-100 active:scale-95 transition-all disabled:opacity-50"
                >
                  {val}%
                </button>
              ))}
            </div>
          )}

          {/* Numpad */}
          <Numpad
            onNumber={handleNumPress}
            onDelete={handleDelete}
            onClear={handleNumpadClear}
            showEnter={false}
            showDecimal={discountType === 'fixed'}
          />

          {/* Reason */}
          <textarea
            value={reason}
            onChange={(e) => setReason(e.target.value)}
            placeholder={t('checkout.order_discount.reason_placeholder')}
            className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-orange-500 focus:border-transparent resize-none text-sm"
            rows={2}
          />
        </div>

        {/* Footer */}
        <div className="shrink-0 p-5 border-t border-gray-100 flex flex-col gap-3 bg-gray-50">
          {hasExistingDiscount && (
            <EscalatableGate
              permission={Permission.ORDERS_DISCOUNT}
              mode="intercept"
              description={t('checkout.order_discount.auth_required')}
              onAuthorized={(user) => handleClear({ id: user.id, name: user.display_name })}
            >
              <button
                disabled={isProcessing}
                className="w-full py-3 px-4 rounded-xl font-bold text-orange-600 bg-orange-50 hover:bg-orange-100 border border-orange-200 transition-colors disabled:opacity-50"
              >
                {t('checkout.order_discount.clear')}
              </button>
            </EscalatableGate>
          )}
          <div className="flex gap-3">
            <button
              onClick={onClose}
              className="flex-1 py-3 px-4 rounded-xl font-bold text-gray-600 hover:bg-gray-200 transition-colors"
            >
              {t('common.action.cancel')}
            </button>
            <EscalatableGate
              permission={Permission.ORDERS_DISCOUNT}
              mode="intercept"
              description={t('checkout.order_discount.auth_required')}
              onAuthorized={(user) => handleApply({ id: user.id, name: user.display_name })}
            >
              <button
                disabled={!canConfirm}
                className={`flex-1 py-3 px-4 rounded-xl font-bold text-white transition-all shadow-lg ${
                  canConfirm
                    ? 'bg-orange-500 hover:bg-orange-600 hover:shadow-orange-500/30 hover:-translate-y-0.5'
                    : 'bg-gray-300 cursor-not-allowed'
                }`}
              >
                {t('checkout.order_discount.confirm')}
              </button>
            </EscalatableGate>
          </div>
        </div>
      </div>
    </div>
  );
};
