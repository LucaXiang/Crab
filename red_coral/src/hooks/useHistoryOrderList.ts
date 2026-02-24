import { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { logger } from '@/utils/logger';
import type { ArchivedOrderSummary, ArchivedOrderListResponse } from '@/core/domain/types';

interface UseHistoryOrderListResult {
  orders: ArchivedOrderSummary[];
  total: number;
  page: number;
  pageSize: number;
  setPage: (page: number) => void;
  search: string;
  setSearch: (term: string) => void;
  loading: boolean;
  refresh: () => void;
}

/**
 * Hook for fetching history order list (summary only, no items/timeline)
 *
 * Backend returns ArchivedOrderSummary directly - no frontend conversion needed.
 */
export const useHistoryOrderList = (
  initialPageSize: number = 20,
  enabled: boolean = true
): UseHistoryOrderListResult => {
  const [orders, setOrders] = useState<ArchivedOrderSummary[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [loading, setLoading] = useState(false);

  // Debounce search term to avoid too many requests
  const [debouncedSearch, setDebouncedSearch] = useState('');

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(search);
      setPage(1); // Reset to first page on search change
    }, 500);
    return () => clearTimeout(timer);
  }, [search]);

  const fetchOrderList = useCallback(async () => {
    if (!enabled) return;

    setLoading(true);
    try {
      // Only show orders from the last 7 days
      const sevenDaysAgo = new Date();
      sevenDaysAgo.setDate(sevenDaysAgo.getDate() - 7);

      const response = await invokeApi<ArchivedOrderListResponse>('fetch_order_list', {
        params: {
          page,
          limit: initialPageSize,
          search: debouncedSearch || undefined,
          start_time: sevenDaysAgo.getTime(),
        },
      });

      // Backend returns OrderSummary directly - no conversion needed
      // Deduplicate when appending pages (guards against unstable DB ordering)
      setOrders((prev) => {
        if (page === 1) return response.orders;
        const existingIds = new Set(prev.map(o => o.order_id));
        const newOrders = response.orders.filter(o => !existingIds.has(o.order_id));
        return [...prev, ...newOrders];
      });
      setTotal(Number(response.total));
    } catch (err) {
      console.error('[useHistoryOrderList] Failed to fetch order list:', err);
      logger.error('Failed to fetch order list', err, { component: 'useHistoryOrderList', action: 'fetchOrderList', page, search: debouncedSearch });
      setOrders([]);
      setTotal(0);
    } finally {
      setLoading(false);
    }
  }, [page, initialPageSize, debouncedSearch, enabled]);

  useEffect(() => {
    fetchOrderList();
  }, [fetchOrderList]);

  return {
    orders,
    total,
    page,
    pageSize: initialPageSize,
    setPage,
    search,
    setSearch,
    loading,
    refresh: fetchOrderList,
  };
};
