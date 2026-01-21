import { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { logger } from '@/utils/logger';

/**
 * Lightweight order summary for list view
 * Only includes essential fields for display in sidebar
 */
export interface OrderSummary {
  order_id: number;
  receipt_number?: string;
  table_name: string;
  total: number;
  status: 'COMPLETED' | 'VOID' | 'MOVED' | 'MERGED';
  start_time: number;
  end_time?: number;
  guest_count: number;
}

interface FetchOrderListResponse {
  orders: OrderSummary[];
  total: number;
  page: number;
}

interface UseHistoryOrderListResult {
  orders: OrderSummary[];
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
 * Performance optimization: Only loads essential fields for list display.
 * Full order details (items, timeline) are loaded separately when selected.
 */
export const useHistoryOrderList = (
  initialPageSize: number = 20,
  enabled: boolean = true
): UseHistoryOrderListResult => {
  const [orders, setOrders] = useState<OrderSummary[]>([]);
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
    if (!('__TAURI__' in window)) {
      logger.warn('History orders only available in Tauri environment', { component: 'useHistoryOrderList', action: 'fetchOrderList' });
      return;
    }

    setLoading(true);
    try {
      // Call Rust backend to get order list (summary only)
      // Only show orders from the last 7 days
      const sevenDaysAgo = new Date();
      sevenDaysAgo.setDate(sevenDaysAgo.getDate() - 7);

      const response = await invokeApi<FetchOrderListResponse>('fetch_order_list', {
        params: {
          page,
          limit: initialPageSize,
          search: debouncedSearch || undefined,
          start_time: sevenDaysAgo.getTime(),
        },
      });

      const toMs = (ts: any): number => {
        const n = Number(ts ?? 0);
        if (!Number.isFinite(n) || n <= 0) return 0;
        return n > 10000000000 ? n : n * 1000;
      };
      const mapped = (response.orders || []).map((o: any) => ({
        order_id: Number(o.order_id),
        receipt_number: o.receipt_number,
        table_name: o.table_name,
        total: Number(o.total),
        status: o.status,
        start_time: toMs(o.start_time),
        end_time: toMs(o.end_time),
        guest_count: Number(o.guest_count),
      }));
      setOrders((prev) => (page === 1 ? mapped : [...prev, ...mapped]));
      setTotal(Number(response.total));
    } catch (err) {
      logger.error('Failed to fetch order list', err, { component: 'useHistoryOrderList', action: 'fetchOrderList', page, search: debouncedSearch });
      // Fallback: empty list
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
