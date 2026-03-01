import React, { useState, useEffect } from 'react';
import { invokeApi } from '@/infrastructure/api';
import { useI18n } from '@/hooks/useI18n';
import { X, AlertTriangle } from 'lucide-react';
import { toast } from '@/presentation/components/Toast';
import type { ArchivedOrderDetail } from '@/core/domain/types';

interface AnulacionModalProps {
  order: ArchivedOrderDetail;
  onClose: () => void;
  onCreated: () => void;
}

type AnulacionReason = 'TestOrder' | 'WrongCustomer' | 'Duplicate' | 'Other';

const REASONS: { value: AnulacionReason; i18nKey: string }[] = [
  { value: 'TestOrder', i18nKey: 'anulacion.reason.TEST_ORDER' },
  { value: 'WrongCustomer', i18nKey: 'anulacion.reason.WRONG_CUSTOMER' },
  { value: 'Duplicate', i18nKey: 'anulacion.reason.DUPLICATE' },
  { value: 'Other', i18nKey: 'anulacion.reason.OTHER' },
];

interface EligibilityResult {
  eligible: boolean;
  reason?: string;
}

export const AnulacionModal: React.FC<AnulacionModalProps> = ({ order, onClose, onCreated }) => {
  const { t } = useI18n();
  const [loading, setLoading] = useState(true);
  const [eligible, setEligible] = useState(false);
  const [ineligibleReason, setIneligibleReason] = useState<string | null>(null);
  const [reason, setReason] = useState<AnulacionReason>('TestOrder');
  const [note, setNote] = useState('');
  const [submitting, setSubmitting] = useState(false);

  // Check eligibility on mount
  useEffect(() => {
    (async () => {
      try {
        const result = await invokeApi<EligibilityResult>('check_anulacion_eligibility', {
          orderPk: order.order_id,
        });
        setEligible(result.eligible);
        if (!result.eligible) {
          setIneligibleReason(result.reason ?? null);
        }
      } catch (err) {
        setIneligibleReason(err instanceof Error ? err.message : 'Failed to check eligibility');
      } finally {
        setLoading(false);
      }
    })();
  }, [order.order_id]);

  const handleSubmit = async () => {
    if (!eligible || submitting) return;
    setSubmitting(true);

    try {
      await invokeApi('create_anulacion', {
        request: {
          original_order_pk: order.order_id,
          reason,
          note: note.trim() || null,
        },
      });
      toast.success(t('anulacion.created'));
      onCreated();
    } catch (err) {
      toast.error(err instanceof Error ? err.message : 'Failed to create anulación');
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4" onClick={onClose}>
      <div
        className="bg-white rounded-2xl w-full max-w-md max-h-[85vh] overflow-hidden flex flex-col shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="p-5 border-b border-gray-200 flex items-center justify-between shrink-0">
          <div>
            <h2 className="text-lg font-bold text-gray-900">{t('anulacion.modal.title')}</h2>
            <p className="text-sm text-gray-500 mt-0.5">{order.receipt_number}</p>
          </div>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-lg transition-colors">
            <X size={20} className="text-gray-500" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5 space-y-5">
          {loading ? (
            <div className="text-center text-gray-400 py-8">...</div>
          ) : !eligible ? (
            <div className="flex items-start gap-3 p-4 bg-amber-50 border border-amber-200 rounded-xl">
              <AlertTriangle size={20} className="text-amber-600 shrink-0 mt-0.5" />
              <div>
                <p className="text-sm font-medium text-amber-800">{t('anulacion.modal.ineligible')}</p>
                {ineligibleReason && (
                  <p className="text-sm text-amber-600 mt-1">{ineligibleReason}</p>
                )}
              </div>
            </div>
          ) : (
            <>
              {/* Warning */}
              <div className="flex items-start gap-3 p-4 bg-red-50 border border-red-200 rounded-xl">
                <AlertTriangle size={20} className="text-red-600 shrink-0 mt-0.5" />
                <p className="text-sm text-red-700">{t('anulacion.modal.warning')}</p>
              </div>

              {/* Reason selector */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  {t('anulacion.reason_label')} *
                </label>
                <div className="space-y-2">
                  {REASONS.map((r) => (
                    <label
                      key={r.value}
                      className={`flex items-center gap-3 px-4 py-3 rounded-xl border-2 cursor-pointer transition-colors ${
                        reason === r.value
                          ? 'border-red-500 bg-red-50'
                          : 'border-gray-200 hover:border-gray-300'
                      }`}
                    >
                      <input
                        type="radio"
                        name="anulacion-reason"
                        value={r.value}
                        checked={reason === r.value}
                        onChange={() => setReason(r.value)}
                        className="w-4 h-4 text-red-500 focus:ring-red-500"
                      />
                      <span className="text-sm font-medium text-gray-800">{t(r.i18nKey)}</span>
                    </label>
                  ))}
                </div>
              </div>

              {/* Note (optional) */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  {t('anulacion.modal.note')}
                </label>
                <textarea
                  value={note}
                  onChange={(e) => setNote(e.target.value)}
                  placeholder={t('anulacion.modal.note_placeholder')}
                  rows={2}
                  className="w-full px-4 py-2.5 rounded-xl border border-gray-300 text-sm focus:outline-none focus:ring-2 focus:ring-red-500/30 focus:border-red-500 resize-none"
                />
              </div>
            </>
          )}
        </div>

        {/* Footer */}
        <div className="p-5 border-t border-gray-200 shrink-0">
          <div className="flex gap-3">
            <button
              onClick={onClose}
              className="flex-1 py-2.5 rounded-xl border border-gray-300 text-sm font-medium text-gray-700 hover:bg-gray-50 transition-colors"
            >
              {t('common.action.cancel')}
            </button>
            {eligible && (
              <button
                onClick={handleSubmit}
                disabled={submitting}
                className="flex-1 py-2.5 rounded-xl bg-red-600 text-white text-sm font-medium hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {submitting ? '...' : t('anulacion.modal.confirm')}
              </button>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
