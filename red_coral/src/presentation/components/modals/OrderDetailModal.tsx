import React from 'react';
import { X, Loader2 } from 'lucide-react';
import { useHistoryOrderDetail } from '@/hooks/useHistoryOrderDetail';
import { useI18n } from '@/hooks/useI18n';
import { HistoryDetail } from '@/screens/History/HistoryDetail';
import { toast } from '@/presentation/components/Toast';
import { getErrorMessage } from '@/utils/error';

interface OrderDetailModalProps {
  isOpen: boolean;
  onClose: () => void;
  orderId: number | null;
}

export const OrderDetailModal: React.FC<OrderDetailModalProps> = ({
  isOpen,
  onClose,
  orderId,
}) => {
  const { t } = useI18n();
  const { order, loading, error } = useHistoryOrderDetail(orderId);

  if (!isOpen) return null;

  const handleReprint = async () => {
    if (!order) return;
    // TODO: 收据重打由服务端处理，待接入后端 API
    toast.warning(t('common.message.not_implemented'));
  };

  if (loading) {
     return (
      <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm">
        <div className="bg-white p-6 rounded-xl shadow-xl">
            <Loader2 className="animate-spin text-blue-500" size={32} />
        </div>
      </div>
     )
  }

  // If error or no order (and not loading), close or show error? 
  // For now, if no order is found but ID was provided, it might be an error state.
  // But HistoryDetail handles "no order" gracefully by showing "Select Order".
  // However, in a modal, if we have no order, we probably shouldn't show the modal content empty.
  
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4 animate-in fade-in duration-200">
      {/* Backdrop click to close */}
      <div className="absolute inset-0" onClick={onClose} />
      
      <div className="relative bg-gray-50 rounded-2xl shadow-xl w-full max-w-5xl max-h-[90vh] flex flex-col overflow-hidden animate-in zoom-in-95 duration-200">
        
        {/* Close Button - Sticky */}
        <div className="absolute top-4 right-4 z-10">
            <button 
                onClick={onClose}
                className="p-2 bg-white/80 backdrop-blur-sm border border-gray-200 text-gray-500 hover:text-gray-700 hover:bg-white rounded-full shadow-sm transition-colors"
            >
                <X size={20} />
            </button>
        </div>

        {/* Content - Scrollable */}
        <div className="flex-1 overflow-y-auto p-6 md:p-8">
             {order ? (
                 <HistoryDetail 
                    order={order} 
                    onReprint={handleReprint} 
                 />
             ) : (
                 <div className="flex items-center justify-center h-full text-gray-400">
                     {error ? error : t('history.info.select_order')}
                 </div>
             )}
        </div>
      </div>
    </div>
  );
};
