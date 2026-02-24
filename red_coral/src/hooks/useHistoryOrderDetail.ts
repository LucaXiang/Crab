import { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api';
import type { ArchivedOrderDetail } from '@/core/domain/types';
import { logger } from '@/utils/logger';

interface UseHistoryOrderDetailResult {
  order: ArchivedOrderDetail | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Hook for fetching archived order details (items, payments, timeline)
 *
 * Backend returns ArchivedOrderDetail directly.
 * No frontend conversion needed.
 */
export const useHistoryOrderDetail = (order_id: number | null): UseHistoryOrderDetailResult => {
  const [order, setOrder] = useState<ArchivedOrderDetail | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchOrderDetail = useCallback(async () => {
    if (!order_id) {
      setOrder(null);
      return;
    }

    setLoading(true);
    setError(null);

    try {
      // Backend returns ArchivedOrderDetail directly via graph traversal
      const detail = await invokeApi<ArchivedOrderDetail>('fetch_order_detail', {
        orderId: order_id,
      });

      logger.debug('Loaded archived order detail', {
        component: 'useHistoryOrderDetail',
        action: 'fetchOrderDetail',
        order_id,
        itemCount: detail.items.length,
        paymentCount: detail.payments.length,
        eventCount: detail.timeline.length,
      });

      setOrder(detail);
    } catch (err) {
      logger.error('Failed to fetch order details', err, { component: 'useHistoryOrderDetail', action: 'fetchOrderDetail', order_id });
      setError(err instanceof Error ? err.message : 'Failed to load order');
      setOrder(null);
    } finally {
      setLoading(false);
    }
  }, [order_id]);

  useEffect(() => {
    fetchOrderDetail();
  }, [fetchOrderDetail]);

  return {
    order,
    loading,
    error,
    refresh: fetchOrderDetail,
  };
};
