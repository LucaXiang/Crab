import React, { useState, useEffect, useMemo } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { X, Trash2 } from 'lucide-react';
import { MAX_NOTE_LEN } from '@/shared/constants/validation';

const DELETE_REASONS = ['customer_changed_mind', 'wrong_item', 'kitchen_issue', 'other'] as const;
type DeleteReasonKey = typeof DELETE_REASONS[number];

interface DeleteReasonModalProps {
  isOpen: boolean;
  itemName: string;
  onClose: () => void;
  onConfirm: (reason: string) => void;
}

export const DeleteReasonModal: React.FC<DeleteReasonModalProps> = ({
  isOpen,
  itemName,
  onClose,
  onConfirm,
}) => {
  const { t } = useI18n();
  const [selectedReason, setSelectedReason] = useState<DeleteReasonKey | null>(null);
  const [note, setNote] = useState('');

  useEffect(() => {
    if (isOpen) {
      setSelectedReason(null);
      setNote('');
    }
  }, [isOpen]);

  const canConfirm = useMemo(() => {
    if (selectedReason === null) return false;
    if (selectedReason === 'other') return note.trim().length > 0;
    return true;
  }, [selectedReason, note]);

  const handleConfirm = () => {
    if (!canConfirm || !selectedReason) return;

    let finalReason: string;
    if (selectedReason === 'other') {
      finalReason = note.trim();
    } else {
      const parts = [t(`checkout.delete_item.reason.${selectedReason}`), note.trim()].filter(Boolean);
      finalReason = parts.join(' - ');
    }

    onConfirm(finalReason);
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
              <Trash2 size={24} />
            </div>
            <div>
              <h2 className="text-xl font-bold text-gray-800">{t('checkout.delete_item.title')}</h2>
              <p className="text-sm text-gray-500 mt-0.5">{itemName}</p>
            </div>
          </div>
          <button onClick={onClose} className="p-2 hover:bg-red-100 rounded-full transition-colors text-gray-500">
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto space-y-6">
          <div className="space-y-3">
            <label className="block text-sm font-medium text-gray-700">
              {t('checkout.delete_item.select_reason')}
            </label>
            <div className="grid grid-cols-2 gap-2">
              {DELETE_REASONS.map((key) => (
                <button
                  key={key}
                  onClick={() => setSelectedReason(key)}
                  className={`p-3 rounded-xl border-2 text-left transition-all ${
                    selectedReason === key
                      ? 'border-red-500 bg-red-50 text-red-700'
                      : 'border-gray-100 hover:border-red-200 hover:bg-gray-50 text-gray-600'
                  }`}
                >
                  <span className="font-medium text-sm">{t(`checkout.delete_item.reason.${key}`)}</span>
                </button>
              ))}
            </div>
          </div>

          {/* Note */}
          <div className="space-y-2">
            <label className="block text-sm font-medium text-gray-700">
              {t('checkout.void.note')}
              {selectedReason !== 'other' && (
                <span className="text-gray-400 font-normal ml-1">({t('common.optional')})</span>
              )}
            </label>
            <textarea
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder={t('checkout.delete_item.note_placeholder')}
              maxLength={MAX_NOTE_LEN}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:ring-2 focus:ring-red-500 focus:border-transparent resize-none"
              rows={2}
            />
          </div>
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
                ? 'bg-red-500 hover:bg-red-600 hover:shadow-red-500/30 hover:-translate-y-0.5'
                : 'bg-gray-300 cursor-not-allowed'
            }`}
          >
            {t('checkout.delete_item.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
};
