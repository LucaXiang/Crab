import React, { useState, useEffect } from 'react';
import { toast } from '@/presentation/components/Toast';
import { t } from '@/infrastructure/i18n';
import { getErrorMessage } from '@/utils/error';
import { useHistoryOrderList } from '@/hooks/useHistoryOrderList';
import { useHistoryOrderDetail } from '@/hooks/useHistoryOrderDetail';
import { HistorySidebar } from './HistorySidebar';
import { HistoryDetail } from './HistoryDetail';

interface HistoryScreenProps {
  isVisible: boolean;
  onBack: () => void;
  onOpenStatistics?: () => void;
}

// Sidebar layout constants (px)
const SIDEBAR_HEADER = 120; // title row + search input + padding
const SIDEBAR_FOOTER = 56; // "load more" button area
const ORDER_ITEM_HEIGHT = 85; // average height per order card

export const HistoryScreen: React.FC<HistoryScreenProps> = ({ isVisible, onBack, onOpenStatistics }) => {

  // Compute pageSize from available sidebar height (once on mount)
  const [pageSize] = useState(() => {
    const listHeight = window.innerHeight - SIDEBAR_HEADER - SIDEBAR_FOOTER;
    return Math.max(Math.ceil(listHeight / ORDER_ITEM_HEIGHT) + 2, 6);
  });

  // Step 1: Load order list (summary only - lightweight)
  const { orders, total, page, setPage, search, setSearch, loading: listLoading } = useHistoryOrderList(pageSize, isVisible);

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
    // TODO: 收据重打由服务端处理，待接入后端 API
    toast.warning(t('common.message.not_implemented'));
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
      <div className="flex-1 overflow-y-auto bg-gray-50 p-4" style={{ scrollbarGutter: 'stable' }}>
        {detailLoading ? (
          <div className="h-full flex items-center justify-center">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-500"></div>
          </div>
        ) : (
          <HistoryDetail order={selectedOrder || undefined} onReprint={handleReprint} />
        )}
      </div>
    </div>
  );
};

