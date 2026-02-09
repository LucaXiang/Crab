/**
 * CashPaymentModal - 现金支付弹窗组件
 *
 * 职责：
 * 1. 提供现金支付金额输入界面
 * 2. 显示应付金额和找零
 * 3. 提供快捷金额按钮
 * 4. 验证支付金额
 */

import React, { useState, useCallback } from 'react';
import { Coins, CheckCircle, X } from 'lucide-react';
import { Numpad } from '@/presentation/components/ui/Numpad';
import { useI18n } from '@/hooks/useI18n';
import { Currency } from '@/utils/currency';
import { formatCurrency } from '@/utils/currency';

interface CashPaymentModalProps {
  /** 应付金额 */
  amountDue: number;
  /** 是否显示弹窗 */
  isOpen: boolean;
  /** 是否处理中 */
  isProcessing?: boolean;
  /** 确认支付回调 */
  onConfirm: (tenderedAmount: number) => void;
  /** 取消回调 */
  onCancel: () => void;
  /** 备注信息 */
  note?: string;
}

export const CashPaymentModal: React.FC<CashPaymentModalProps> = ({
  amountDue,
  isOpen,
  isProcessing = false,
  onConfirm,
  onCancel,
  note,
}) => {
  const { t } = useI18n();
  const [tenderedInput, setTenderedInput] = useState<string>(amountDue.toFixed(2));
  const [isSelected, setIsSelected] = useState(true); // 覆盖模式：输入时替换全部

  // Reset input when modal opens or amount changes
  React.useEffect(() => {
    if (isOpen) {
      setTenderedInput(amountDue.toFixed(2));
      setIsSelected(true); // 重新打开时进入覆盖模式
    }
  }, [isOpen, amountDue]);

  const tendered = parseFloat(tenderedInput) || 0;
  const change = Currency.sub(tendered, amountDue);
  const canPay = Currency.gte(tendered, amountDue);

  const handleNumPress = useCallback((num: string) => {
    if (isSelected) {
      // 覆盖模式：替换全部
      setIsSelected(false);
      setTenderedInput(num === '.' ? '0.' : num);
      return;
    }
    setTenderedInput((prev) => {
      if (num === '.' && prev.includes('.')) return prev;
      if (prev.includes('.') && prev.split('.')[1].length >= 2) return prev;
      if (prev === '0' && num !== '.') return num;
      return prev + num;
    });
  }, [isSelected]);

  const handleClear = useCallback(() => {
    setTenderedInput('0');
    setIsSelected(true); // 清空后重新进入覆盖模式
  }, []);

  const handleDelete = useCallback(() => {
    setIsSelected(false);
    setTenderedInput((prev) => {
      const newVal = prev.slice(0, -1);
      return newVal || '0';
    });
  }, []);

  const handleConfirm = useCallback(() => {
    if (canPay && !isProcessing) {
      onConfirm(tendered);
    }
  }, [canPay, isProcessing, tendered, onConfirm]);

  const handleQuickAmount = useCallback((amount: number) => {
    setTenderedInput(amount.toFixed(2));
    setIsSelected(false);
  }, []);

  /**
   * 生成快捷金额建议
   */
  const getCashSuggestions = useCallback((amount: number) => {
    const suggestions = new Set<number>();

    // 添加精确金额
    suggestions.add(amount);

    const bills = [5, 10, 20, 50, 100, 200, 500];

    // 添加最近的 5 的倍数
    const next5 = Math.ceil(amount / 5) * 5;
    if (next5 > amount) suggestions.add(next5);

    // 添加最近的 10 的倍数
    const next10 = Math.ceil(amount / 10) * 10;
    if (next10 > amount) suggestions.add(next10);

    // 添加常用纸币面额
    for (const bill of bills) {
      if (bill > amount) {
        suggestions.add(bill);
        if (suggestions.size >= 4) break;
      }
    }

    return Array.from(suggestions).sort((a, b) => a - b).slice(0, 4);
  }, []);

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-60 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div className="bg-gray-100 rounded-2xl shadow-2xl w-full max-w-5xl max-h-[95vh] flex flex-col md:flex-row overflow-hidden">
        {/* Left Panel - Info & Suggestions */}
        <div className="md:w-1/2 p-4 md:p-8 flex flex-col border-b md:border-b-0 md:border-r border-gray-200 bg-white">
          <div className="shrink-0">
            {/* Header */}
            <div className="flex items-center justify-between mb-4 md:mb-6">
              <h3 className="text-xl md:text-2xl font-bold text-gray-800 flex items-center gap-2">
                <Coins className="text-green-500" size={24} />
                {t('checkout.method.cash')}
              </h3>
              <button
                onClick={onCancel}
                disabled={isProcessing}
                className="p-2 hover:bg-gray-100 rounded-full transition-colors disabled:opacity-50"
                aria-label={t('common.action.close')}
              >
                <X size={20} className="text-gray-500" />
              </button>
            </div>

            {/* Note */}
            {note && (
              <div className="mb-3 text-sm text-gray-600 bg-blue-50 border border-blue-100 rounded-lg p-3">
                {note}
              </div>
            )}

            {/* Amount Due */}
            <div className="space-y-3 md:space-y-4">
              <div className="p-3 md:p-4 bg-gray-50 rounded-xl border border-gray-100">
                <div className="text-xs md:text-sm text-gray-500 uppercase font-bold">
                  {t('checkout.amount.due')}
                </div>
                <div className="text-2xl md:text-4xl font-bold text-gray-900 mt-1">
                  {formatCurrency(amountDue)}
                </div>
              </div>

              {/* Change */}
              <div
                className={`p-3 md:p-4 rounded-xl border transition-colors ${
                  canPay ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-100'
                }`}
              >
                <div className="text-xs md:text-sm text-gray-500 uppercase font-bold">
                  {t('checkout.amount.change')}
                </div>
                <div
                  className={`text-2xl md:text-4xl font-bold mt-1 ${
                    canPay ? 'text-green-600' : 'text-red-400'
                  }`}
                >
                  {canPay ? formatCurrency(change.toNumber()) : t('checkout.error.insufficient')}
                </div>
              </div>
            </div>
          </div>

          {/* Quick Amount Buttons */}
          <div className="grid grid-cols-2 gap-2 md:gap-3 mt-3 md:mt-4">
            {getCashSuggestions(amountDue).map((amt) => (
              <button
                key={amt}
                onClick={() => handleQuickAmount(amt)}
                disabled={isProcessing}
                className="h-12 md:h-14 bg-white border border-green-200 text-green-700 font-bold text-base md:text-lg rounded-xl hover:bg-green-50 active:scale-95 transition-all disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {formatCurrency(amt)}
              </button>
            ))}
          </div>

          {/* Spacer to push cancel button to bottom, aligned with confirm */}
          <div className="mt-auto" />
          {/* Cancel Button */}
          <button
            onClick={onCancel}
            disabled={isProcessing}
            className="mt-4 w-full h-14 md:h-20 bg-gray-100 text-gray-600 text-lg md:text-2xl font-bold hover:bg-gray-200 rounded-xl transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {t('common.action.cancel')}
          </button>
        </div>

        {/* Right Panel - Numpad */}
        <div className="md:w-1/2 p-4 md:p-8 bg-gray-50 flex flex-col min-h-0">
          <div className="flex-shrink-0 mb-4 md:mb-6">
            <label className="text-xs md:text-sm text-gray-500 font-bold ml-1">
              {t('checkout.amount.tendered')}
            </label>
            <div className="h-16 md:h-20 bg-white rounded-xl flex items-center justify-between px-4 md:px-6 mt-2 border-2 border-green-200 shadow-sm">
              <div className="flex items-center">
                <span className="text-green-500 mr-2 text-xl md:text-2xl font-bold">€</span>
                <span
                  className={`text-2xl md:text-4xl font-mono font-bold px-1 rounded transition-colors ${
                    isSelected
                      ? 'bg-green-500 text-white'
                      : 'text-gray-800'
                  }`}
                >
                  {tenderedInput || '0'}
                </span>
                {!isSelected && (
                  <span className="animate-pulse ml-0.5 w-0.5 h-6 md:h-8 bg-green-400 rounded" />
                )}
              </div>
            </div>
          </div>

          {/* Numpad */}
          <div className="flex-1 min-h-0 flex flex-col">
            <div className="flex-1 min-h-0">
              <Numpad onNumber={handleNumPress} onDelete={handleDelete} onClear={handleClear} className="h-full" showEnter={false} />
            </div>

            {/* Confirm Button */}
            <button
              onClick={handleConfirm}
              disabled={!canPay || isProcessing}
              className="mt-3 md:mt-4 h-14 md:h-20 bg-green-600 text-white rounded-xl text-lg md:text-2xl font-bold shadow-lg shadow-green-200 hover:bg-green-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-all flex items-center justify-center gap-2 shrink-0"
            >
              {isProcessing ? (
                <>
                  <div className="w-6 h-6 border-3 border-white border-t-transparent rounded-full animate-spin" />
                  {t('common.message.processing')}
                </>
              ) : (
                <>
                  <CheckCircle size={24} /> {t('checkout.payment.confirm')}
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
