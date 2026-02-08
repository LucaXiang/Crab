import React, { useState, useEffect, useMemo } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { X, AlertTriangle, Ban, FileWarning } from 'lucide-react';
import { formatCurrency } from '@/utils/currency';

/** 作废类型 */
export type VoidType = 'CANCELLED' | 'LOSS_SETTLED';

/** 损失原因 */
export type LossReason = 'CUSTOMER_FLED' | 'CUSTOMER_INSOLVENT' | 'OTHER';

export interface VoidOrderOptions {
  voidType: VoidType;
  lossReason?: LossReason;
  lossAmount?: number;
  note?: string;
  authorizerId?: number | null;
  authorizerName?: string | null;
}

interface VoidReasonModalProps {
  isOpen: boolean;
  /** 已付金额 */
  paidAmount: number;
  /** 未付金额 */
  unpaidAmount: number;
  onClose: () => void;
  onConfirm: (options: VoidOrderOptions) => void;
}

export const VoidReasonModal: React.FC<VoidReasonModalProps> = ({
  isOpen,
  paidAmount,
  unpaidAmount,
  onClose,
  onConfirm,
}) => {
  const { t } = useI18n();

  // 智能默认：有已付款时默认选择损失结算
  const defaultVoidType: VoidType = paidAmount > 0 ? 'LOSS_SETTLED' : 'CANCELLED';

  const [voidType, setVoidType] = useState<VoidType>(defaultVoidType);
  const [lossReason, setLossReason] = useState<LossReason | null>(null);
  const [cancelReason, setCancelReason] = useState<CancelReasonKey | null>(null);
  const [note, setNote] = useState('');

  // 重置状态当 modal 打开时
  useEffect(() => {
    if (isOpen) {
      setVoidType(paidAmount > 0 ? 'LOSS_SETTLED' : 'CANCELLED');
      setLossReason(null);
      setCancelReason(null);
      setNote('');
    }
  }, [isOpen, paidAmount]);

  const CANCEL_REASONS = ['customer_cancelled', 'system_test', 'duplicate_order', 'other'] as const;
  type CancelReasonKey = typeof CANCEL_REASONS[number];

  const LOSS_REASONS: LossReason[] = ['CUSTOMER_FLED', 'CUSTOMER_INSOLVENT', 'OTHER'];

  // 确认按钮是否可用
  const canConfirm = useMemo(() => {
    if (voidType === 'CANCELLED') {
      if (cancelReason === null) return false;
      if (cancelReason === 'other') return note.trim().length > 0;
      return true;
    }
    if (voidType === 'LOSS_SETTLED') {
      if (lossReason === null) return false;
      if (lossReason === 'OTHER') return note.trim().length > 0;
      return true;
    }
    return false;
  }, [voidType, cancelReason, lossReason, note]);

  const handleConfirm = () => {
    if (!canConfirm) return;

    // Build note: for presets store the key, for OTHER use custom text
    let finalNote: string | undefined;
    if (voidType === 'CANCELLED' && cancelReason) {
      if (cancelReason === 'other') {
        finalNote = note.trim() || undefined;
      } else {
        const parts = [cancelReason, note.trim()].filter(Boolean);
        finalNote = parts.join(' - ');
      }
    } else {
      finalNote = note.trim() || undefined;
    }

    const options: VoidOrderOptions = {
      voidType,
      lossReason: voidType === 'LOSS_SETTLED' ? lossReason! : undefined,
      lossAmount: voidType === 'LOSS_SETTLED' ? unpaidAmount : undefined,
      note: finalNote,
    };

    onConfirm(options);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div className="bg-white rounded-2xl shadow-2xl max-w-md w-full overflow-hidden flex flex-col max-h-[90vh] animate-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="p-6 border-b border-gray-100 flex justify-between items-center bg-red-50">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-red-100 rounded-full text-red-600">
              <AlertTriangle size={24} />
            </div>
            <h2 className="text-xl font-bold text-gray-800">{t('checkout.void.title')}</h2>
          </div>
          <button onClick={onClose} className="p-2 hover:bg-red-100 rounded-full transition-colors text-gray-500">
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto space-y-6">
          {/* 金额显示 */}
          <div className="bg-gray-50 rounded-xl p-4">
            <div className="flex justify-between text-sm">
              <span className="text-gray-600">{t('checkout.void.paid_amount')}</span>
              <span className="font-medium text-gray-900">{formatCurrency(paidAmount)}</span>
            </div>
            <div className="flex justify-between text-sm mt-2">
              <span className="text-gray-600">{t('checkout.void.unpaid_amount')}</span>
              <span className="font-medium text-gray-900">{formatCurrency(unpaidAmount)}</span>
            </div>
          </div>

          {/* 作废类型选择 */}
          <div className="space-y-3">
            <label className="block text-sm font-medium text-gray-700">
              {t('checkout.void.select_type')}
            </label>

            {/* 取消订单 - 始终显示 */}
            <button
              onClick={() => setVoidType('CANCELLED')}
              className={`w-full p-4 rounded-xl border-2 text-left transition-all flex items-center gap-3 ${
                voidType === 'CANCELLED'
                  ? 'border-red-500 bg-red-50'
                  : 'border-gray-100 hover:border-red-200 hover:bg-gray-50'
              }`}
            >
              <div className={`p-2 rounded-lg ${voidType === 'CANCELLED' ? 'bg-red-100 text-red-600' : 'bg-gray-100 text-gray-500'}`}>
                <Ban size={20} />
              </div>
              <div>
                <span className={`font-medium ${voidType === 'CANCELLED' ? 'text-red-700' : 'text-gray-700'}`}>
                  {t('checkout.void.type.cancelled')}
                </span>
                <p className="text-sm text-gray-500 mt-0.5">
                  {t('checkout.void.type.cancelled_desc')}
                </p>
              </div>
            </button>

            {/* 损失结算 - 仅有支付记录时显示 */}
            {paidAmount > 0 && (
              <button
                onClick={() => setVoidType('LOSS_SETTLED')}
                className={`w-full p-4 rounded-xl border-2 text-left transition-all flex items-center gap-3 ${
                  voidType === 'LOSS_SETTLED'
                    ? 'border-orange-500 bg-orange-50'
                    : 'border-gray-100 hover:border-orange-200 hover:bg-gray-50'
                }`}
              >
                <div className={`p-2 rounded-lg ${voidType === 'LOSS_SETTLED' ? 'bg-orange-100 text-orange-600' : 'bg-gray-100 text-gray-500'}`}>
                  <FileWarning size={20} />
                </div>
                <div>
                  <span className={`font-medium ${voidType === 'LOSS_SETTLED' ? 'text-orange-700' : 'text-gray-700'}`}>
                    {t('checkout.void.type.loss_settled')}
                  </span>
                  <p className="text-sm text-gray-500 mt-0.5">
                    {t('checkout.void.type.loss_settled_desc')}
                  </p>
                </div>
              </button>
            )}
          </div>

          {/* 取消原因 - 仅在取消订单时显示 */}
          {voidType === 'CANCELLED' && (
            <div className="space-y-3">
              <label className="block text-sm font-medium text-gray-700">
                {t('checkout.void.cancel_reason.title')}
              </label>
              <div className="grid grid-cols-2 gap-2">
                {CANCEL_REASONS.map((key) => (
                  <button
                    key={key}
                    onClick={() => setCancelReason(key)}
                    className={`p-3 rounded-xl border-2 text-left transition-all ${
                      cancelReason === key
                        ? 'border-red-500 bg-red-50 text-red-700'
                        : 'border-gray-100 hover:border-red-200 hover:bg-gray-50 text-gray-600'
                    }`}
                  >
                    <span className="font-medium text-sm">{t(`checkout.void.cancel_reason.${key}`)}</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* 损失原因 - 仅在损失结算时显示 */}
          {voidType === 'LOSS_SETTLED' && (
            <div className="space-y-3">
              <label className="block text-sm font-medium text-gray-700">
                {t('checkout.void.loss_reason.title')}
              </label>
              <div className="grid grid-cols-1 gap-2">
                {LOSS_REASONS.map((key) => (
                  <button
                    key={key}
                    onClick={() => setLossReason(key)}
                    className={`p-3 rounded-xl border-2 text-left transition-all ${
                      lossReason === key
                        ? 'border-orange-500 bg-orange-50 text-orange-700'
                        : 'border-gray-100 hover:border-orange-200 hover:bg-gray-50 text-gray-600'
                    }`}
                  >
                    <span className="font-medium">{t(`checkout.void.loss_reason.${key.toLowerCase()}`)}</span>
                  </button>
                ))}
              </div>
            </div>
          )}

          {/* 备注输入 */}
          <div className="space-y-2">
            <label className="block text-sm font-medium text-gray-700">
              {t('checkout.void.note')}
              <span className="text-gray-400 font-normal ml-1">({t('common.optional')})</span>
            </label>
            <textarea
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder={t('checkout.void.note_placeholder')}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-red-500 focus:border-transparent resize-none"
              rows={2}
            />
          </div>

          {/* 损失金额提示 */}
          {voidType === 'LOSS_SETTLED' && unpaidAmount > 0 && (
            <div className="bg-orange-50 border border-orange-200 rounded-xl p-4">
              <p className="text-sm text-orange-700">
                {t('checkout.void.loss_amount_notice', { amount: formatCurrency(unpaidAmount) })}
              </p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="p-6 border-t border-gray-100 flex gap-3 bg-gray-50">
          <button
            onClick={onClose}
            className="flex-1 py-3 px-4 rounded-xl font-bold text-gray-600 hover:bg-gray-200 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          <button
            onClick={handleConfirm}
            disabled={!canConfirm}
            className={`flex-1 py-3 px-4 rounded-xl font-bold text-white transition-all shadow-lg ${
              canConfirm
                ? voidType === 'LOSS_SETTLED'
                  ? 'bg-orange-500 hover:bg-orange-600 hover:shadow-orange-500/30 hover:-translate-y-0.5'
                  : 'bg-red-500 hover:bg-red-600 hover:shadow-red-500/30 hover:-translate-y-0.5'
                : 'bg-gray-300 cursor-not-allowed'
            }`}
          >
            {t('checkout.void.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
};
