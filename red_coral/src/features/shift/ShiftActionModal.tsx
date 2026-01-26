/**
 * Shift Action Modal (班次操作弹窗)
 *
 * 支持三种操作:
 * - open: 开班 (输入开班现金)
 * - close: 收班 (输入实际现金，计算差异)
 * - force_close: 强制关闭 (不盘点现金)
 */

import React, { useState, useEffect, useMemo } from 'react';
import { X, Play, CheckCircle, AlertTriangle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';
import { toast } from '@/presentation/components/Toast';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { Currency } from '@/utils/currency';
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

  // Form state
  const [startingCash, setStartingCash] = useState('0');
  const [actualCash, setActualCash] = useState('');
  const [note, setNote] = useState('');
  const [loading, setLoading] = useState(false);

  // Reset form when modal opens
  useEffect(() => {
    if (open) {
      setStartingCash('0');
      setActualCash(shift?.expected_cash ? Currency.floor2(shift.expected_cash).toString() : '');
      setNote('');
    }
  }, [open, shift]);

  // Calculate variance preview using Currency utility
  const variancePreview = useMemo(() => {
    if (action !== 'close' || !shift || !actualCash) return null;
    try {
      const actual = Currency.toDecimal(actualCash);
      const expected = Currency.toDecimal(shift.expected_cash);
      return Currency.sub(actual, expected);
    } catch {
      return null;
    }
  }, [action, shift, actualCash]);

  // Handle submit
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!user) return;

    setLoading(true);
    try {
      if (action === 'open') {
        const cashValue = Currency.floor2(startingCash || '0').toNumber();
        if (cashValue < 0) {
          toast.error(t('settings.shift.invalid_amount'));
          setLoading(false);
          return;
        }
        await api.openShift({
          operator_id: user.id,
          operator_name: user.display_name,
          starting_cash: cashValue,
          note: note || undefined,
        });
        toast.success(t('settings.shift.open_success'));
      } else if (action === 'close' && shift?.id) {
        if (!actualCash) {
          toast.error(t('settings.shift.invalid_amount'));
          setLoading(false);
          return;
        }
        const actual = Currency.floor2(actualCash).toNumber();
        if (actual < 0) {
          toast.error(t('settings.shift.invalid_amount'));
          setLoading(false);
          return;
        }
        await api.closeShift(shift.id, {
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
        await api.forceCloseShift(shift.id, {
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

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={onClose}
      />

      {/* Modal */}
      <div className="relative bg-white rounded-2xl shadow-2xl w-full max-w-md mx-4 overflow-hidden">
        {/* Header */}
        <div
          className={`px-6 py-4 flex items-center justify-between ${
            action === 'force_close'
              ? 'bg-orange-500'
              : action === 'close'
              ? 'bg-emerald-500'
              : 'bg-blue-500'
          }`}
        >
          <div className="flex items-center gap-3 text-white">
            {action === 'open' && <Play size={24} />}
            {action === 'close' && <CheckCircle size={24} />}
            {action === 'force_close' && <AlertTriangle size={24} />}
            <h2 className="text-lg font-bold">
              {action === 'open' && t('settings.shift.modal.open_title')}
              {action === 'close' && t('settings.shift.modal.close_title')}
              {action === 'force_close' && t('settings.shift.modal.force_close_title')}
            </h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-white/20 rounded-lg transition-colors text-white"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          {/* Open shift form */}
          {action === 'open' && (
            <>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.shift.modal.operator')}
                </label>
                <input
                  type="text"
                  value={user?.display_name || ''}
                  disabled
                  className="w-full px-4 py-2 border border-gray-200 rounded-lg bg-gray-50 text-gray-600"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.shift.modal.starting_cash')}
                </label>
                <div className="relative">
                  <span className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-500">¥</span>
                  <input
                    type="number"
                    step="0.01"
                    min="0"
                    value={startingCash}
                    onChange={(e) => setStartingCash(e.target.value)}
                    className="w-full pl-8 pr-4 py-2 border border-gray-200 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500"
                    autoFocus
                  />
                </div>
                <p className="mt-1 text-xs text-gray-500">
                  {t('settings.shift.modal.starting_cash_hint')}
                </p>
              </div>
            </>
          )}

          {/* Close shift form */}
          {action === 'close' && shift && (
            <>
              <div className="bg-gray-50 rounded-lg p-4 space-y-2">
                <div className="flex justify-between text-sm">
                  <span className="text-gray-600">{t('settings.shift.modal.expected_cash')}</span>
                  <span className="font-mono font-medium">¥{Currency.floor2(shift.expected_cash).toString()}</span>
                </div>
                <div className="flex justify-between text-sm">
                  <span className="text-gray-600">{t('settings.shift.modal.starting_cash')}</span>
                  <span className="font-mono">¥{Currency.floor2(shift.starting_cash).toString()}</span>
                </div>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.shift.modal.actual_cash')}
                </label>
                <div className="relative">
                  <span className="absolute left-4 top-1/2 -translate-y-1/2 text-gray-500">¥</span>
                  <input
                    type="number"
                    step="0.01"
                    min="0"
                    value={actualCash}
                    onChange={(e) => setActualCash(e.target.value)}
                    className="w-full pl-8 pr-4 py-2 border border-gray-200 rounded-lg focus:ring-2 focus:ring-emerald-500 focus:border-emerald-500"
                    autoFocus
                  />
                </div>
              </div>

              {/* Variance preview */}
              {variancePreview !== null && (
                <div
                  className={`p-3 rounded-lg ${
                    variancePreview.isZero()
                      ? 'bg-green-50 text-green-700'
                      : variancePreview.isPositive()
                      ? 'bg-blue-50 text-blue-700'
                      : 'bg-red-50 text-red-700'
                  }`}
                >
                  <div className="flex items-center justify-between">
                    <span className="text-sm font-medium">
                      {t('settings.shift.modal.variance')}
                    </span>
                    <span className="font-mono font-bold">
                      {variancePreview.isZero()
                        ? t('settings.shift.variance.balanced')
                        : `${variancePreview.isPositive() ? '+' : ''}¥${Currency.floor2(variancePreview).toString()}`}
                    </span>
                  </div>
                </div>
              )}
            </>
          )}

          {/* Force close warning */}
          {action === 'force_close' && (
            <div className="bg-orange-50 border border-orange-200 rounded-lg p-4">
              <div className="flex gap-3">
                <AlertTriangle className="text-orange-500 shrink-0" size={20} />
                <div>
                  <p className="text-sm font-medium text-orange-800">
                    {t('settings.shift.modal.force_close_warning_title')}
                  </p>
                  <p className="text-sm text-orange-700 mt-1">
                    {t('settings.shift.modal.force_close_warning')}
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* Note field */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('settings.shift.modal.note')}
            </label>
            <textarea
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder={t('settings.shift.modal.note_placeholder')}
              rows={2}
              className="w-full px-4 py-2 border border-gray-200 rounded-lg focus:ring-2 focus:ring-gray-300 focus:border-gray-400 resize-none"
            />
          </div>

          {/* Actions */}
          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              disabled={loading}
              className="flex-1 px-4 py-2 border border-gray-200 text-gray-700 rounded-lg hover:bg-gray-50 transition-colors disabled:opacity-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={loading}
              className={`flex-1 px-4 py-2 text-white rounded-lg transition-colors disabled:opacity-50 ${
                action === 'force_close'
                  ? 'bg-orange-500 hover:bg-orange-600'
                  : action === 'close'
                  ? 'bg-emerald-500 hover:bg-emerald-600'
                  : 'bg-blue-500 hover:bg-blue-600'
              }`}
            >
              {loading ? t('common.loading') : t('common.confirm')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
