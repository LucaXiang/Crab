import React, { useEffect } from 'react';
import { Check, Coins, Printer } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency/formatCurrency';

interface PaymentSuccessModalProps {
  isOpen: boolean;
  type: 'NORMAL' | 'CASH';
  change?: number;
  onClose: () => void;
  onPrint?: () => void;
  autoCloseDelay?: number; // ms
}

const PaymentSuccessModalComponent: React.FC<PaymentSuccessModalProps> = ({
  isOpen,
  type,
  change,
  onClose,
  onPrint,
  autoCloseDelay = 5000
}) => {
  const { t } = useI18n();
  const [timeLeft, setTimeLeft] = React.useState(Math.ceil(autoCloseDelay / 1000));

  useEffect(() => {
    if (!isOpen || autoCloseDelay <= 0) return () => {};

    setTimeLeft(Math.ceil(autoCloseDelay / 1000));
    
    const timer = setInterval(() => {
      setTimeLeft((prev) => {
        if (prev <= 1) {
          clearInterval(timer);
          onClose();
          return 0;
        }
        return prev - 1;
      });
    }, 1000);

    return () => clearInterval(timer);
  }, [isOpen, autoCloseDelay, onClose]);

  if (!isOpen) return null;

  return (
    <div 
      className="fixed inset-0 z-100 flex items-center justify-center bg-black/60 backdrop-blur-sm animate-in fade-in duration-200"
      onClick={onClose}
    >
      <div className="bg-white rounded-3xl p-8 max-w-sm w-full mx-4 shadow-2xl transform transition-all animate-in zoom-in-95 duration-200 flex flex-col items-center text-center" onClick={(e) => e.stopPropagation()}>
        <div className={`w-20 h-20 rounded-full flex items-center justify-center mb-6 ${
          type === 'CASH' ? 'bg-green-100 text-green-500' : 'bg-red-100 text-[#FF5E5E]'
        }`}>
          {type === 'CASH' ? <Coins size={40} /> : <Check size={40} strokeWidth={3} />}
        </div>
        
        <h2 className="text-2xl font-bold text-gray-900 mb-2">
          {type === 'CASH' ? t('checkout.payment.success') : t('checkout.order_completed')}
        </h2>
        
        {type === 'CASH' && change !== undefined && (
          <div className="mt-4 p-4 bg-gray-50 rounded-2xl w-full">
            <div className="text-sm text-gray-500 font-medium uppercase tracking-wide mb-1">
              {t('checkout.amount.change')}
            </div>
            <div className="text-4xl font-bold text-gray-900">
              {formatCurrency(change)}
            </div>
          </div>
        )}

        {onPrint ? (
          <div className="flex gap-4 mt-8 w-full">
            <button 
              onClick={(e) => {
                e.stopPropagation();
                onPrint();
                onClose();
              }}
              className="flex-1 py-3 bg-blue-600 text-white rounded-xl font-bold hover:bg-blue-700 transition-colors flex items-center justify-center gap-2"
            >
              <Printer size={20} />
              {t('common.action.print')}
            </button>
            <button 
              onClick={(e) => {
                e.stopPropagation();
                onClose();
              }}
              className="flex-1 py-3 bg-gray-100 text-gray-700 rounded-xl font-bold hover:bg-gray-200 transition-colors"
            >
              {t('common.action.close')}
            </button>
          </div>
        ) : (
          <div className="mt-8 text-sm text-gray-400 cursor-pointer hover:text-gray-600 transition-colors" onClick={onClose}>
             {t('common.hint.tap_to_close')}
             {autoCloseDelay > 0 && ` (${timeLeft}s)`}
          </div>
        )}
      </div>
    </div>
  );
};

export const PaymentSuccessModal = React.memo(PaymentSuccessModalComponent);

PaymentSuccessModal.displayName = 'PaymentSuccessModal';
