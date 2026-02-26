import React, { useEffect, useState, useCallback } from 'react';
import { X, Printer, RefreshCw } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { createTauriClient } from '@/infrastructure/api/tauri-client';
import type { KitchenOrder } from '@/core/domain/types/api';

interface KitchenReprintModalProps {
  isOpen: boolean;
  orderId: string;
  onClose: () => void;
}

export const KitchenReprintModal: React.FC<KitchenReprintModalProps> = ({ isOpen, orderId, onClose }) => {
  const { t } = useI18n();
  const [kitchenOrders, setKitchenOrders] = useState<KitchenOrder[]>([]);
  const [loading, setLoading] = useState(false);
  const [reprintingId, setReprintingId] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    try {
      const client = createTauriClient();
      const res = await client.getKitchenOrdersForOrder(orderId);
      setKitchenOrders(res.items);
    } catch (err) {
      logger.error('Failed to fetch kitchen orders', err);
    } finally {
      setLoading(false);
    }
  }, [orderId]);

  useEffect(() => {
    if (isOpen) fetchData();
  }, [isOpen, fetchData]);

  const handleReprint = async (id: string) => {
    setReprintingId(id);
    try {
      const client = createTauriClient();
      await client.reprintKitchenOrder(id);
      toast.success(t('checkout.kitchen_reprint.success'));
      await fetchData();
    } catch (err) {
      logger.error('Kitchen reprint failed', err);
      toast.error(t('checkout.kitchen_reprint.failed'));
    } finally {
      setReprintingId(null);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={onClose}>
      <div className="bg-white rounded-2xl shadow-2xl w-[600px] max-h-[80vh] flex flex-col" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-gray-200">
          <h2 className="text-xl font-bold text-gray-800">{t('checkout.kitchen_reprint.tab_kitchen')}</h2>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-gray-100 transition-colors">
            <X size={20} className="text-gray-500" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5">
          {loading ? (
            <div className="flex items-center justify-center py-12">
              <RefreshCw size={24} className="animate-spin text-gray-400" />
            </div>
          ) : kitchenOrders.length === 0 ? (
            <div className="text-center py-12 text-gray-400">
              {t('checkout.kitchen_reprint.empty')}
            </div>
          ) : (
            <div className="space-y-3">
              {kitchenOrders.map((ko) => {
                const itemsSummary = ko.items.map((i) => i.context.kitchen_name).join(', ');
                const totalQty = ko.items.reduce((sum, i) => sum + i.context.quantity, 0);
                const time = new Date(ko.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });

                return (
                  <div key={ko.id} className="flex items-center gap-4 p-4 bg-gray-50 rounded-xl">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="text-sm font-medium text-gray-500">{time}</span>
                        <span className="text-sm text-gray-400">·</span>
                        <span className="text-sm font-medium text-gray-700">x{totalQty}</span>
                        {ko.print_count > 0 && (
                          <span className="text-xs bg-amber-100 text-amber-700 px-2 py-0.5 rounded-full font-medium">
                            {t('checkout.kitchen_reprint.print_count', { count: String(ko.print_count) })}
                          </span>
                        )}
                      </div>
                      <div className="text-sm text-gray-600 line-clamp-1">{itemsSummary}</div>
                    </div>
                    <button
                      onClick={() => handleReprint(ko.id)}
                      disabled={reprintingId === ko.id}
                      className="flex items-center gap-2 px-4 py-2 bg-amber-500 hover:bg-amber-600 text-white rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed shrink-0"
                    >
                      <Printer size={16} className={reprintingId === ko.id ? 'animate-pulse' : ''} />
                      {t('checkout.kitchen_reprint.button')}
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
