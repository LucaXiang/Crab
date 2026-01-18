import React, { useState, useEffect } from 'react';
import { useHistoryOrderList } from '@/hooks/useHistoryOrderList';
import { useHistoryOrderDetail } from '@/hooks/useHistoryOrderDetail';
import { HistorySidebar } from './HistorySidebar';
import { HistoryDetail } from './HistoryDetail';

interface HistoryScreenProps {
  isVisible: boolean;
  onBack: () => void;
  onOpenStatistics?: () => void;
}

export const HistoryScreen: React.FC<HistoryScreenProps> = ({ isVisible, onBack, onOpenStatistics }) => {

  // Step 1: Load order list (summary only - lightweight)
  const { orders, total, page, pageSize, setPage, search, setSearch, loading: listLoading } = useHistoryOrderList(6, isVisible);

  // Step 2: Track selected order key
  const [selectedID, setSelectedID] = useState<number | null>(null);

  // Auto-select first order when list loads
  useEffect(() => {
    if (orders.length > 0 && !selectedID) {
      setSelectedID(orders[0].order_id);
    }
  }, [orders, selectedID]);

  // Step 3: Load full details for selected order only (lazy load)
  const { order: selectedOrder, loading: detailLoading } = useHistoryOrderDetail(selectedID);

  const totalPages = Math.ceil(total / pageSize) || 1;

  const handleReprint = async () => {
    if (!selectedOrder) return;
    try {
      const { reprintReceipt } = await import('@/services/printService');
      await reprintReceipt(selectedOrder.key);
    } catch {}
  };

  return (
    <div className="flex h-full w-full bg-gray-100 overflow-hidden font-sans">
      <HistorySidebar
        orders={orders}
        selectedKey={selectedID}
        onSelect={setSelectedID}
        search={search}
        setSearch={setSearch}
        page={page}
        totalPages={totalPages}
        setPage={setPage}
        loading={listLoading}
        onBack={onBack}
        onOpenStatistics={onOpenStatistics}
      />
      <div className="flex-1 overflow-y-auto bg-gray-50 p-6">
        {detailLoading ? (
          <div className="h-full flex items-center justify-center">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-red-500"></div>
          </div>
        ) : (
          <HistoryDetail order={selectedOrder || undefined} onReprint={handleReprint} />
        )}
      </div>
    </div>
  );
};

