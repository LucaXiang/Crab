import React, { useState } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { X, AlertTriangle } from 'lucide-react';

interface VoidReasonModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: (reason: string) => void;
}

export const VoidReasonModal: React.FC<VoidReasonModalProps> = ({
  isOpen,
  onClose,
  onConfirm,
}) => {
  const { t } = useI18n();
  const [selectedReason, setSelectedReason] = useState<string | null>(null);

  if (!isOpen) return null;

  const reasons = [
    { key: 'runaway', label: t('checkout.void_reason.runaway') },
    { key: 'dineAndDash', label: t('checkout.void_reason.dine_and_dash') },
    { key: 'systemTest', label: t('checkout.void_reason.system_test') },
    { key: 'ownerTreat', label: t('checkout.void_reason.owner_treat') },
  ];

  return (
    <div className="fixed inset-0 z-80 bg-black/50 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-md w-full overflow-hidden flex flex-col max-h-[90vh]">
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
        <div className="p-6 overflow-y-auto">
          <p className="text-gray-600 mb-6">{t('checkout.void.confirm')}</p>
          
          <div className="space-y-3">
            <label className="block text-sm font-medium text-gray-700 mb-2">
              {t('checkout.void.reason')}
            </label>
            <div className="grid grid-cols-1 gap-3">
              {reasons.map((reason) => (
                <button
                  key={reason.key}
                  onClick={() => setSelectedReason(reason.key)}
                  className={`p-4 rounded-xl border-2 text-left transition-all ${
                    selectedReason === reason.key
                      ? 'border-red-500 bg-red-50 text-red-700'
                      : 'border-gray-100 hover:border-red-200 hover:bg-gray-50 text-gray-600'
                  }`}
                >
                  <span className="font-medium">{reason.label}</span>
                </button>
              ))}
            </div>
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
            onClick={() => {
              if (selectedReason) {
                // Construct the full translation key
                onConfirm(`checkout.voidReason.${selectedReason}`);
                onClose();
              }
            }}
            disabled={!selectedReason}
            className={`flex-1 py-3 px-4 rounded-xl font-bold text-white transition-all shadow-lg ${
              selectedReason
                ? 'bg-red-500 hover:bg-red-600 hover:shadow-red-500/30 hover:-translate-y-0.5'
                : 'bg-gray-300 cursor-not-allowed'
            }`}
          >
            {t('checkout.void.title')}
          </button>
        </div>
      </div>
    </div>
  );
};
