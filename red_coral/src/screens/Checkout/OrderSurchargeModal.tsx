import React, { useState, useEffect, useMemo } from 'react';
import { HeldOrder, Permission } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { X, TrendingUp, Percent, Tag } from 'lucide-react';
import { formatCurrency } from '@/utils/currency';
import { EscalatableGate } from '@/presentation/components/auth/EscalatableGate';
import { applyOrderSurcharge } from '@/core/stores/order/useOrderOperations';
import { toast } from '@/presentation/components/Toast';

type SurchargeType = 'percent' | 'fixed';

interface OrderSurchargeModalProps {
  isOpen: boolean;
  order: HeldOrder;
  onClose: () => void;
}

export const OrderSurchargeModal: React.FC<OrderSurchargeModalProps> = ({
  isOpen,
  order,
  onClose,
}) => {
  const { t } = useI18n();

  const hasExistingSurcharge = !!(
    (order.order_manual_surcharge_fixed && order.order_manual_surcharge_fixed > 0) ||
    (order.order_manual_surcharge_percent && order.order_manual_surcharge_percent > 0)
  );

  const [surchargeType, setSurchargeType] = useState<SurchargeType>(
    order.order_manual_surcharge_percent ? 'percent' : 'fixed'
  );
  const [value, setValue] = useState('');
  const [reason, setReason] = useState('');
  const [isProcessing, setIsProcessing] = useState(false);

  // Reset state only when modal opens (not on order WebSocket updates)
  useEffect(() => {
    if (isOpen) {
      if (order.order_manual_surcharge_percent) {
        setSurchargeType('percent');
        setValue(String(order.order_manual_surcharge_percent));
      } else if (order.order_manual_surcharge_fixed) {
        setSurchargeType('fixed');
        setValue(String(order.order_manual_surcharge_fixed));
      } else {
        setSurchargeType('fixed');
        setValue('');
      }
      setReason('');
      setIsProcessing(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isOpen]);

  const parsedValue = parseFloat(value) || 0;

  const canConfirm = useMemo(() => {
    if (isProcessing) return false;
    if (parsedValue <= 0) return false;
    if (surchargeType === 'percent' && parsedValue > 100) return false;
    return true;
  }, [isProcessing, parsedValue, surchargeType]);

  const handleApply = async (authorizer?: { id: string; name: string }) => {
    if (!canConfirm) return;
    setIsProcessing(true);
    try {
      await applyOrderSurcharge(order.order_id, {
        surchargePercent: surchargeType === 'percent' ? parsedValue : undefined,
        surchargeAmount: surchargeType === 'fixed' ? parsedValue : undefined,
        reason: reason.trim() || undefined,
        authorizer,
      });
      toast.success(t('checkout.order_surcharge.title'));
      onClose();
    } catch (err) {
      console.error('Apply order surcharge failed:', err);
      toast.error(String(err));
    } finally {
      setIsProcessing(false);
    }
  };

  const handleClear = async (authorizer?: { id: string; name: string }) => {
    setIsProcessing(true);
    try {
      await applyOrderSurcharge(order.order_id, { authorizer });
      toast.success(t('checkout.order_surcharge.clear'));
      onClose();
    } catch (err) {
      console.error('Clear order surcharge failed:', err);
      toast.error(String(err));
    } finally {
      setIsProcessing(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-80 bg-black/50 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-md w-full overflow-hidden flex flex-col max-h-[90vh]">
        {/* Header */}
        <div className="p-6 border-b border-gray-100 flex justify-between items-center bg-purple-50">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-purple-100 rounded-full text-purple-600">
              <TrendingUp size={24} />
            </div>
            <h2 className="text-xl font-bold text-gray-800">{t('checkout.order_surcharge.title')}</h2>
          </div>
          <button onClick={onClose} className="p-2 hover:bg-purple-100 rounded-full transition-colors text-gray-500">
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto space-y-6">
          {/* Order Total Display */}
          <div className="bg-gray-50 rounded-xl p-4">
            <div className="flex justify-between text-sm">
              <span className="text-gray-600">{t('checkout.amount.total')}</span>
              <span className="font-medium text-gray-900">{formatCurrency(order.original_total)}</span>
            </div>
          </div>

          {/* Surcharge Type Selection */}
          <div className="space-y-3">
            <div className="grid grid-cols-2 gap-3">
              <button
                onClick={() => { setSurchargeType('percent'); setValue(''); }}
                className={`p-4 rounded-xl border-2 text-left transition-all flex items-center gap-3 ${
                  surchargeType === 'percent'
                    ? 'border-purple-500 bg-purple-50'
                    : 'border-gray-100 hover:border-purple-200 hover:bg-gray-50'
                }`}
              >
                <div className={`p-2 rounded-lg ${surchargeType === 'percent' ? 'bg-purple-100 text-purple-600' : 'bg-gray-100 text-gray-500'}`}>
                  <Percent size={20} />
                </div>
                <span className={`font-medium ${surchargeType === 'percent' ? 'text-purple-700' : 'text-gray-700'}`}>
                  {t('checkout.order_surcharge.type_percent')}
                </span>
              </button>
              <button
                onClick={() => { setSurchargeType('fixed'); setValue(''); }}
                className={`p-4 rounded-xl border-2 text-left transition-all flex items-center gap-3 ${
                  surchargeType === 'fixed'
                    ? 'border-purple-500 bg-purple-50'
                    : 'border-gray-100 hover:border-purple-200 hover:bg-gray-50'
                }`}
              >
                <div className={`p-2 rounded-lg ${surchargeType === 'fixed' ? 'bg-purple-100 text-purple-600' : 'bg-gray-100 text-gray-500'}`}>
                  <Tag size={20} />
                </div>
                <span className={`font-medium ${surchargeType === 'fixed' ? 'text-purple-700' : 'text-gray-700'}`}>
                  {t('checkout.order_surcharge.type_fixed')}
                </span>
              </button>
            </div>
          </div>

          {/* Value Input */}
          <div className="space-y-2">
            <input
              type="number"
              value={value}
              onChange={(e) => setValue(e.target.value)}
              placeholder={surchargeType === 'percent'
                ? t('checkout.order_surcharge.percent_placeholder')
                : t('checkout.order_surcharge.amount_placeholder')
              }
              className="w-full px-4 py-4 border border-gray-200 rounded-xl focus:ring-2 focus:ring-purple-500 focus:border-transparent text-2xl font-bold text-center tabular-nums"
              min="0"
              max={surchargeType === 'percent' ? '100' : undefined}
              step={surchargeType === 'percent' ? '1' : '0.01'}
              autoFocus
            />
            {surchargeType === 'percent' && parsedValue > 100 && (
              <p className="text-sm text-red-500 text-center">0-100%</p>
            )}
          </div>

          {/* Reason */}
          <div className="space-y-2">
            <textarea
              value={reason}
              onChange={(e) => setReason(e.target.value)}
              placeholder={t('checkout.order_surcharge.reason_placeholder')}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-purple-500 focus:border-transparent resize-none text-sm"
              rows={2}
            />
          </div>
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-gray-100 flex flex-col gap-3 bg-gray-50">
          {hasExistingSurcharge && (
            <EscalatableGate
              permission={Permission.ORDERS_DISCOUNT}
              mode="intercept"
              description={t('checkout.order_surcharge.auth_required')}
              onAuthorized={(user) => handleClear({ id: user.id, name: user.display_name })}
            >
              <button
                disabled={isProcessing}
                className="w-full py-3 px-4 rounded-xl font-bold text-orange-600 bg-orange-50 hover:bg-orange-100 border border-orange-200 transition-colors disabled:opacity-50"
              >
                {t('checkout.order_surcharge.clear')}
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
              description={t('checkout.order_surcharge.auth_required')}
              onAuthorized={(user) => handleApply({ id: user.id, name: user.display_name })}
            >
              <button
                disabled={!canConfirm}
                className={`flex-1 py-3 px-4 rounded-xl font-bold text-white transition-all shadow-lg ${
                  canConfirm
                    ? 'bg-purple-500 hover:bg-purple-600 hover:shadow-purple-500/30 hover:-translate-y-0.5'
                    : 'bg-gray-300 cursor-not-allowed'
                }`}
              >
                {t('checkout.order_surcharge.confirm')}
              </button>
            </EscalatableGate>
          </div>
        </div>
      </div>
    </div>
  );
};
