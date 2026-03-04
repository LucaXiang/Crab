import React, { useState, useEffect, useCallback, useMemo } from 'react';
import { invokeApi } from '@/infrastructure/api';
import { formatCurrency } from '@/utils/currency';
import { localizedErrorMessage } from '@/utils/error/commandError';
import { useI18n } from '@/hooks/useI18n';
import { X, AlertTriangle } from 'lucide-react';
import { toast } from '@/presentation/components/Toast';
import type {
  ArchivedOrderDetail,
  RefundableInfo,
  CreditNoteDetail,
  CreateCreditNoteRequest,
} from '@/core/domain/types';

interface RefundModalProps {
  order: ArchivedOrderDetail;
  onClose: () => void;
  onCreated: () => void;
}

type RefundReason = 'CUSTOMER_REQUEST' | 'WRONG_ORDER' | 'QUALITY_ISSUE' | 'OVERCHARGE' | 'OTHER';

const REFUND_REASONS: RefundReason[] = [
  'CUSTOMER_REQUEST',
  'WRONG_ORDER',
  'QUALITY_ISSUE',
  'OVERCHARGE',
  'OTHER',
];

interface RefundItem {
  instance_id: string;
  name: string;
  unit_price: number;
  max_quantity: number;
  quantity: number;
  selected: boolean;
}

export const RefundModal: React.FC<RefundModalProps> = ({ order, onClose, onCreated }) => {
  const { t } = useI18n();
  const [refundableInfo, setRefundableInfo] = useState<RefundableInfo | null>(null);
  const [items, setItems] = useState<RefundItem[]>([]);
  const [refundMethod, setRefundMethod] = useState<'CASH' | 'CARD'>('CASH');
  const [reason, setReason] = useState<RefundReason>('CUSTOMER_REQUEST');
  const [note, setNote] = useState('');
  const [submitting, setSubmitting] = useState(false);
  const [loading, setLoading] = useState(true);

  // Fetch refundable info on mount
  useEffect(() => {
    (async () => {
      try {
        const info = await invokeApi<RefundableInfo>('fetch_refundable_info', {
          orderPk: order.order_id,
        });
        setRefundableInfo(info);

        // Build refund item list, subtracting already-refunded quantities
        const refundedMap = new Map(
          info.refunded_items.map((ri) => [ri.instance_id, ri.refunded_quantity]),
        );
        const refundItems: RefundItem[] = order.items
          .filter((item) => !item.is_comped && item.quantity > 0)
          .map((item) => {
            const alreadyRefunded = refundedMap.get(item.instance_id) ?? 0;
            const remaining = item.quantity - alreadyRefunded;
            return {
              instance_id: item.instance_id,
              name: item.name + (item.spec_name && item.spec_name !== 'default' ? ` (${item.spec_name})` : ''),
              unit_price: item.unit_price,
              max_quantity: remaining,
              quantity: 0,
              selected: false,
            };
          })
          .filter((item) => item.max_quantity > 0); // hide fully refunded items
        setItems(refundItems);
      } catch (err) {
        toast.error(localizedErrorMessage(err));
      } finally {
        setLoading(false);
      }
    })();
  }, [order]);

  const toggleItem = useCallback((idx: number) => {
    setItems((prev) =>
      prev.map((item, i) =>
        i === idx
          ? { ...item, selected: !item.selected, quantity: !item.selected ? item.max_quantity : 0 }
          : item,
      ),
    );
  }, []);

  const setItemQuantity = useCallback((idx: number, qty: number) => {
    setItems((prev) =>
      prev.map((item, i) =>
        i === idx
          ? { ...item, quantity: Math.max(0, Math.min(qty, item.max_quantity)), selected: qty > 0 }
          : item,
      ),
    );
  }, []);

  const selectedItems = useMemo(() => items.filter((item) => item.selected && item.quantity > 0), [items]);

  const totalCredit = useMemo(
    () => selectedItems.reduce((sum, item) => sum + item.unit_price * item.quantity, 0),
    [selectedItems],
  );

  const canSubmit =
    selectedItems.length > 0 &&
    (reason !== 'OTHER' || note.trim().length > 0) &&
    !submitting &&
    refundableInfo != null &&
    totalCredit <= refundableInfo.remaining_refundable + 0.01;

  const handleSubmit = async () => {
    if (!canSubmit) return;
    setSubmitting(true);

    try {
      const request: CreateCreditNoteRequest = {
        original_order_pk: order.order_id,
        items: selectedItems.map((item) => ({
          instance_id: item.instance_id,
          quantity: item.quantity,
        })),
        refund_method: refundMethod,
        reason,
        note: note.trim() || undefined,
      };

      await invokeApi<CreditNoteDetail>('create_credit_note', { request });
      toast.success(t('credit_note.created'));
      onCreated();
    } catch (err) {
      toast.error(localizedErrorMessage(err));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4" onClick={onClose}>
      <div
        className="bg-white rounded-2xl w-full max-w-lg max-h-[85vh] overflow-hidden flex flex-col shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="p-5 border-b border-gray-200 flex items-center justify-between shrink-0">
          <div>
            <h2 className="text-lg font-bold text-gray-900">{t('credit_note.modal.title')}</h2>
            <p className="text-sm text-gray-500 mt-0.5">
              {order.receipt_number}
              {refundableInfo && (
                <span className="ml-2">
                  · {t('credit_note.remaining')}: {formatCurrency(refundableInfo.remaining_refundable)}
                </span>
              )}
            </p>
          </div>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-lg transition-colors">
            <X size={20} className="text-gray-500" />
          </button>
        </div>

        {/* Body */}
        <div className="flex-1 overflow-y-auto p-5 space-y-5">
          {loading ? (
            <div className="text-center text-gray-400 py-8">...</div>
          ) : (
            <>
              {/* Items selection */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  {t('credit_note.modal.select_items')}
                </label>
                <div className="border border-gray-200 rounded-xl overflow-hidden divide-y divide-gray-100">
                  {items.map((item, idx) => (
                    <div
                      key={item.instance_id}
                      className={`px-4 py-3 flex items-center gap-3 cursor-pointer transition-colors ${
                        item.selected ? 'bg-red-50' : 'hover:bg-gray-50'
                      }`}
                      onClick={() => toggleItem(idx)}
                    >
                      <input
                        type="checkbox"
                        checked={item.selected}
                        onChange={() => toggleItem(idx)}
                        className="w-4 h-4 rounded border-gray-300 text-red-500 focus:ring-red-500"
                      />
                      <div className="flex-1 min-w-0">
                        <div className="text-sm font-medium text-gray-800 truncate">{item.name}</div>
                        <div className="text-xs text-gray-400">{formatCurrency(item.unit_price)} × {item.max_quantity}</div>
                      </div>
                      {item.selected && (
                        <div className="flex items-center gap-2" onClick={(e) => e.stopPropagation()}>
                          <button
                            className="w-7 h-7 rounded-full border border-gray-300 flex items-center justify-center text-gray-600 hover:bg-gray-100"
                            onClick={() => setItemQuantity(idx, item.quantity - 1)}
                          >
                            -
                          </button>
                          <span className="w-8 text-center font-bold text-sm">{item.quantity}</span>
                          <button
                            className="w-7 h-7 rounded-full border border-gray-300 flex items-center justify-center text-gray-600 hover:bg-gray-100"
                            onClick={() => setItemQuantity(idx, item.quantity + 1)}
                          >
                            +
                          </button>
                        </div>
                      )}
                      <div className="text-sm font-bold text-gray-700 w-20 text-right shrink-0">
                        {item.selected ? formatCurrency(item.unit_price * item.quantity) : ''}
                      </div>
                    </div>
                  ))}
                </div>
              </div>

              {/* Refund method */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  {t('credit_note.modal.refund_method')}
                </label>
                <div className="flex gap-3">
                  {(['CASH', 'CARD'] as const).map((method) => (
                    <button
                      key={method}
                      onClick={() => setRefundMethod(method)}
                      className={`flex-1 py-2.5 rounded-xl border-2 text-sm font-medium transition-colors ${
                        refundMethod === method
                          ? 'border-red-500 bg-red-50 text-red-700'
                          : 'border-gray-200 bg-white text-gray-600 hover:border-gray-300'
                      }`}
                    >
                      {t(`credit_note.method.${method.toLowerCase()}`)}
                    </button>
                  ))}
                </div>
              </div>

              {/* Reason */}
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-2">
                  {t('credit_note.modal.reason')} *
                </label>
                <div className="space-y-2">
                  {REFUND_REASONS.map((r) => (
                    <label
                      key={r}
                      className={`flex items-center gap-3 px-4 py-3 rounded-xl border-2 cursor-pointer transition-colors ${
                        reason === r
                          ? 'border-red-500 bg-red-50'
                          : 'border-gray-200 hover:border-gray-300'
                      }`}
                    >
                      <input
                        type="radio"
                        name="refund-reason"
                        value={r}
                        checked={reason === r}
                        onChange={() => setReason(r)}
                        className="w-4 h-4 text-red-500 focus:ring-red-500"
                      />
                      <span className="text-sm font-medium text-gray-800">{t(`credit_note.reason.${r}`)}</span>
                    </label>
                  ))}
                </div>
              </div>

              {/* Note - only for OTHER reason */}
              {reason === 'OTHER' && (
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-2">
                    {t('credit_note.modal.note')} *
                  </label>
                  <textarea
                    value={note}
                    onChange={(e) => setNote(e.target.value)}
                    placeholder={t('credit_note.modal.note_placeholder')}
                    rows={2}
                    className="w-full px-4 py-2.5 rounded-xl border border-gray-300 text-sm focus:outline-none focus:ring-2 focus:ring-red-500/30 focus:border-red-500 resize-none"
                  />
                </div>
              )}

              {/* Over-refund warning */}
              {refundableInfo && totalCredit > refundableInfo.remaining_refundable + 0.01 && (
                <div className="flex items-center gap-2 p-3 bg-amber-50 border border-amber-200 rounded-xl text-sm text-amber-700">
                  <AlertTriangle size={16} className="shrink-0" />
                  <span>{t('credit_note.error.over_refund')}</span>
                </div>
              )}
            </>
          )}
        </div>

        {/* Footer */}
        <div className="p-5 border-t border-gray-200 shrink-0">
          <div className="flex items-center justify-between mb-3">
            <span className="text-sm text-gray-600">{t('credit_note.modal.total_refund')}</span>
            <span className="text-xl font-bold text-red-500">
              {totalCredit > 0 ? `-${formatCurrency(totalCredit)}` : formatCurrency(0)}
            </span>
          </div>
          <div className="flex gap-3">
            <button
              onClick={onClose}
              className="flex-1 py-2.5 rounded-xl border border-gray-300 text-sm font-medium text-gray-700 hover:bg-gray-50 transition-colors"
            >
              {t('common.action.cancel')}
            </button>
            <button
              onClick={handleSubmit}
              disabled={!canSubmit}
              className="flex-1 py-2.5 rounded-xl bg-red-500 text-white text-sm font-medium hover:bg-red-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {submitting ? '...' : t('credit_note.action.confirm_refund')}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
