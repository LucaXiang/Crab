/**
 * Shift Action Modal (班次操作弹窗)
 *
 * 支持三种操作:
 * - open: 开班 (输入开班现金)
 * - close: 收班 (输入实际现金，计算差异)
 * - force_close: 强制关闭 (不盘点现金)
 *
 * UI 风格与 CashPaymentModal 保持一致
 */

import React, { useState, useEffect, useMemo, useCallback, useRef } from 'react';
import { X, Play, CheckCircle, AlertTriangle, Banknote } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';
import { toast } from '@/presentation/components/Toast';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useShiftStore } from '@/core/stores/shift';
import { Currency, formatCurrency } from '@/utils/currency';
import { Numpad } from '@/presentation/components/ui/Numpad';
import type { Shift } from '@/core/domain/types/api';

const api = createTauriClient();

interface ShiftActionModalProps {
  open: boolean;
  action: 'open' | 'close' | 'force_close';
  shift: Shift | null;
  onClose: () => void;
  onSuccess: () => void;
}

export const ShiftActionModal: React.FC<ShiftActionModalProps> = ({
  open,
  action,
  shift,
  onClose,
  onSuccess,
}) => {
  const { t } = useI18n();
  const user = useAuthStore(state => state.user);
  const { openShift: storeOpenShift, closeShift: storeCloseShift, forceCloseShift: storeForceCloseShift } = useShiftStore();

  // Form state
  const [cashInput, setCashInput] = useState('0');
  const [note, setNote] = useState('');
  const [loading, setLoading] = useState(false);
  const isTypingRef = useRef(false);

  // Reset form when modal opens
  useEffect(() => {
    if (open) {
      if (action === 'close' && shift?.expected_cash) {
        setCashInput(Currency.floor2(shift.expected_cash).toString());
      } else {
        setCashInput('0');
      }
      setNote('');
      isTypingRef.current = false;
    }
  }, [open, action, shift]);

  const cashValue = parseFloat(cashInput) || 0;

  // Calculate variance preview for close action
  const variancePreview = useMemo(() => {
    if (action !== 'close' || !shift) return null;
    return Currency.sub(cashValue, shift.expected_cash);
  }, [action, shift, cashValue]);

  // Numpad handlers
  const handleNumPress = useCallback((num: string) => {
    setCashInput((prev) => {
      if (!isTypingRef.current) {
        isTypingRef.current = true;
        return num === '.' ? '0.' : num;
      }
      if (num === '.' && prev.includes('.')) return prev;
      if (prev.includes('.') && prev.split('.')[1].length >= 2) return prev;
      if (prev === '0' && num !== '.') return num;
      return prev + num;
    });
  }, []);

  const handleClear = useCallback(() => {
    setCashInput('');
    isTypingRef.current = true;
  }, []);

  // Quick amount buttons
  const quickAmounts = [0, 100, 200, 500];

  // Handle submit
  const handleSubmit = async () => {
    if (!user || loading) return;

    setLoading(true);
    try {
      if (action === 'open') {
        const cashAmount = Currency.floor2(cashInput || '0').toNumber();
        if (cashAmount < 0) {
          toast.error(t('settings.shift.invalid_amount'));
          setLoading(false);
          return;
        }
        await storeOpenShift({
          operator_id: user.id,
          operator_name: user.display_name,
          starting_cash: cashAmount,
          note: note || undefined,
        });
        toast.success(t('settings.shift.open_success'));
      } else if (action === 'close' && shift?.id) {
        const actual = Currency.floor2(cashInput).toNumber();
        if (actual < 0) {
          toast.error(t('settings.shift.invalid_amount'));
          setLoading(false);
          return;
        }
        await storeCloseShift(shift.id, {
          actual_cash: actual,
          note: note || undefined,
        });
        const variance = Currency.sub(actual, shift.expected_cash);
        if (variance.isZero()) {
          toast.success(t('settings.shift.close_success_balanced'));
        } else {
          toast.success(
            t('settings.shift.close_success_variance', {
              variance: (variance.isPositive() ? '+' : '') + Currency.floor2(variance).toString(),
            })
          );
        }
      } else if (action === 'force_close' && shift?.id) {
        await storeForceCloseShift(shift.id, {
          note: note || t('settings.shift.force_close_default_note'),
        });
        toast.success(t('settings.shift.force_close_success'));
      }
      onSuccess();
    } catch (err) {
      console.error('Shift action failed:', err);
      toast.error(t('settings.shift.action_failed'));
    } finally {
      setLoading(false);
    }
  };

  if (!open) return null;

  // Force close - simple confirmation dialog
  if (action === 'force_close') {
    return (
      <div className="fixed inset-0 z-60 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200">
        <div className="bg-white rounded-2xl shadow-2xl w-full max-w-md overflow-hidden">
          {/* Header */}
          <div className="p-6 border-b border-gray-100">
            <div className="flex items-center justify-between">
              <h3 className="text-xl font-bold text-gray-800 flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-orange-100 flex items-center justify-center">
                  <AlertTriangle className="text-orange-500" size={20} />
                </div>
                {t('settings.shift.modal.force_close_title')}
              </h3>
              <button
                onClick={onClose}
                disabled={loading}
                className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
              >
                <X size={20} className="text-gray-500" />
              </button>
            </div>
          </div>

          {/* Content */}
          <div className="p-6 space-y-4">
            <div className="bg-orange-50 border border-orange-200 rounded-xl p-4">
              <p className="text-sm text-orange-800">
                {t('settings.shift.modal.force_close_warning')}
              </p>
            </div>

            {/* Note */}
            <div>
              <label className="block text-sm font-medium text-gray-600 mb-2">
                {t('settings.shift.modal.note')}
              </label>
              <textarea
                value={note}
                onChange={(e) => setNote(e.target.value)}
                placeholder={t('settings.shift.modal.note_placeholder')}
                rows={2}
                className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-orange-200 focus:border-orange-400 resize-none"
              />
            </div>
          </div>

          {/* Actions */}
          <div className="p-6 pt-0 flex gap-3">
            <button
              onClick={onClose}
              disabled={loading}
              className="flex-1 py-3 text-gray-600 font-medium hover:bg-gray-100 rounded-xl transition-colors"
            >
              {t('common.cancel')}
            </button>
            <button
              onClick={handleSubmit}
              disabled={loading}
              className="flex-1 py-3 bg-orange-500 text-white font-bold rounded-xl hover:bg-orange-600 transition-colors disabled:opacity-50 flex items-center justify-center gap-2"
            >
              {loading ? (
                <div className="w-5 h-5 border-2 border-white border-t-transparent rounded-full animate-spin" />
              ) : (
                <>
                  <AlertTriangle size={18} />
                  {t('settings.shift.force_close_shift')}
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Open / Close - full numpad layout
  const isOpenAction = action === 'open';
  const accentColor = isOpenAction ? 'green' : 'emerald';

  return (
    <div className="fixed inset-0 z-60 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div className="bg-gray-100 rounded-2xl shadow-2xl w-full max-w-4xl max-h-[95vh] flex flex-col md:flex-row overflow-hidden">
        {/* Left Panel - Info */}
        <div className="md:w-1/2 p-6 md:p-8 flex flex-col border-b md:border-b-0 md:border-r border-gray-200 bg-white">
          {/* Header */}
          <div className="flex items-center justify-between mb-6">
            <h3 className="text-xl md:text-2xl font-bold text-gray-800 flex items-center gap-3">
              <div className={`w-10 h-10 rounded-full bg-${accentColor}-100 flex items-center justify-center`}>
                {isOpenAction ? (
                  <Play className={`text-${accentColor}-600`} size={20} />
                ) : (
                  <CheckCircle className={`text-${accentColor}-600`} size={20} />
                )}
              </div>
              {isOpenAction ? t('settings.shift.modal.open_title') : t('settings.shift.modal.close_title')}
            </h3>
            <button
              onClick={onClose}
              disabled={loading}
              className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
            >
              <X size={20} className="text-gray-500" />
            </button>
          </div>

          {/* Operator Info */}
          <div className="p-4 bg-gray-50 rounded-xl mb-4">
            <div className="text-xs text-gray-500 uppercase font-bold mb-1">
              {t('settings.shift.modal.operator')}
            </div>
            <div className="text-lg font-medium text-gray-800">
              {user?.display_name || '-'}
            </div>
          </div>

          {/* Close action: Show expected cash */}
          {action === 'close' && shift && (
            <div className="space-y-3 mb-4">
              <div className="p-4 bg-gray-50 rounded-xl">
                <div className="text-xs text-gray-500 uppercase font-bold">
                  {t('settings.shift.modal.expected_cash')}
                </div>
                <div className="text-2xl md:text-3xl font-bold text-gray-900 mt-1 font-mono">
                  {formatCurrency(shift.expected_cash)}
                </div>
              </div>

              {/* Variance Preview */}
              {variancePreview !== null && (
                <div
                  className={`p-4 rounded-xl border transition-colors ${
                    variancePreview.isZero()
                      ? 'bg-green-50 border-green-200'
                      : variancePreview.isPositive()
                      ? 'bg-blue-50 border-blue-200'
                      : 'bg-red-50 border-red-200'
                  }`}
                >
                  <div className="text-xs text-gray-500 uppercase font-bold">
                    {t('settings.shift.modal.variance')}
                  </div>
                  <div
                    className={`text-2xl md:text-3xl font-bold mt-1 font-mono ${
                      variancePreview.isZero()
                        ? 'text-green-600'
                        : variancePreview.isPositive()
                        ? 'text-blue-600'
                        : 'text-red-600'
                    }`}
                  >
                    {variancePreview.isZero()
                      ? t('settings.shift.variance.balanced')
                      : `${variancePreview.isPositive() ? '+' : ''}${formatCurrency(variancePreview.toNumber())}`}
                  </div>
                </div>
              )}
            </div>
          )}

          {/* Quick amounts for open action */}
          {isOpenAction && (
            <div className="grid grid-cols-2 gap-3 mb-4">
              {quickAmounts.map((amt) => (
                <button
                  key={amt}
                  onClick={() => {
                    setCashInput(amt.toString());
                    isTypingRef.current = true;
                  }}
                  disabled={loading}
                  className="h-12 bg-white border border-green-200 text-green-700 font-bold rounded-xl hover:bg-green-50 active:scale-95 transition-all disabled:opacity-50"
                >
                  {formatCurrency(amt)}
                </button>
              ))}
            </div>
          )}

          {/* Note field */}
          <div className="mt-auto">
            <label className="block text-xs text-gray-500 uppercase font-bold mb-2">
              {t('settings.shift.modal.note')}
            </label>
            <textarea
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder={t('settings.shift.modal.note_placeholder')}
              rows={2}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-green-200 focus:border-green-400 resize-none text-sm"
            />
          </div>

          {/* Cancel button */}
          <button
            onClick={onClose}
            disabled={loading}
            className="mt-4 w-full py-3 text-gray-500 font-bold hover:bg-gray-100 rounded-xl transition-colors"
          >
            {t('common.cancel')}
          </button>
        </div>

        {/* Right Panel - Numpad */}
        <div className="md:w-1/2 p-6 md:p-8 bg-gray-50 flex flex-col min-h-0">
          {/* Cash Input Display */}
          <div className="flex-shrink-0 mb-6">
            <label className="text-xs text-gray-500 font-bold uppercase ml-1">
              {isOpenAction ? t('settings.shift.modal.starting_cash') : t('settings.shift.modal.actual_cash')}
            </label>
            <div className="h-16 md:h-20 bg-white rounded-xl flex items-center px-6 mt-2 border-2 border-green-200 shadow-sm">
              <Banknote className="text-green-500 mr-3" size={24} />
              <span className="text-2xl md:text-4xl font-mono font-bold text-gray-800 truncate">
                {formatCurrency(cashValue)}
              </span>
              <span className="animate-pulse ml-1 w-0.5 h-6 md:h-8 bg-green-400" />
            </div>
            {isOpenAction && (
              <p className="mt-2 text-xs text-gray-500 ml-1">
                {t('settings.shift.modal.starting_cash_hint')}
              </p>
            )}
          </div>

          {/* Numpad */}
          <div className="flex-1 min-h-0 flex flex-col">
            <div className="flex-1 min-h-0">
              <Numpad onNumber={handleNumPress} onClear={handleClear} className="h-full" showEnter={false} />
            </div>

            {/* Confirm Button */}
            <button
              onClick={handleSubmit}
              disabled={loading}
              className="mt-4 h-14 md:h-20 bg-green-600 text-white rounded-xl text-lg md:text-2xl font-bold shadow-lg shadow-green-200 hover:bg-green-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition-all flex items-center justify-center gap-2 shrink-0"
            >
              {loading ? (
                <>
                  <div className="w-6 h-6 border-3 border-white border-t-transparent rounded-full animate-spin" />
                  {t('common.loading')}
                </>
              ) : (
                <>
                  <CheckCircle size={24} />
                  {isOpenAction ? t('settings.shift.open_shift') : t('settings.shift.close_shift')}
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
