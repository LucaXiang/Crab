import { useState, useEffect, useCallback, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { logger } from '@/utils/logger';
import type { ChainEntryItem, ChainEntryListResponse } from '@/core/domain/types';

interface UseChainEntryListResult {
  entries: ChainEntryItem[];
  total: number;
  hasMore: boolean;
  loadMore: () => void;
  search: string;
  setSearch: (term: string) => void;
  loading: boolean;
  refresh: () => void;
}

const PAGE_SIZE = 15;

/**
 * Cursor-based chain entry list with real-time prepend.
 *
 * - First page: offset=0 (no cursor needed)
 * - Subsequent pages: before=<last chain_id> (stable, immune to new inserts)
 * - Listens to 'order-sync' for terminal events and auto-prepends new entries
 */
export const useChainEntryList = (
  enabled: boolean = true,
): UseChainEntryListResult => {
  const [entries, setEntries] = useState<ChainEntryItem[]>([]);
  const [total, setTotal] = useState(0);
  const [hasMore, setHasMore] = useState(true);
  const [search, setSearch] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');
  const [loading, setLoading] = useState(false);
  // Trigger counter for loading more pages
  const [loadMoreTrigger, setLoadMoreTrigger] = useState(0);
  // Track whether we need a fresh first-page fetch
  const [fetchGeneration, setFetchGeneration] = useState(0);
  const cursorRef = useRef<number | null>(null);

  // Debounce search: only reset when search actually changes
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(prev => {
        if (prev === search) return prev; // no change, skip reset
        // Search changed: reset pagination
        cursorRef.current = null;
        setEntries([]);
        setTotal(0);
        setHasMore(true);
        setFetchGeneration(g => g + 1);
        return search;
      });
    }, 400);
    return () => clearTimeout(timer);
  }, [search]);

  // Fetch entries (first page or cursor page)
  const fetchEntries = useCallback(async () => {
    if (!enabled) return;
    setLoading(true);
    try {
      const searchParam = debouncedSearch || undefined;
      const cursor = cursorRef.current;

      let response: ChainEntryListResponse;
      if (cursor === null) {
        // First page
        response = await invokeApi<ChainEntryListResponse>('fetch_chain_entries', {
          params: { limit: PAGE_SIZE, offset: 0, search: searchParam },
        });
        setEntries(response.entries);
      } else {
        // Cursor page
        response = await invokeApi<ChainEntryListResponse>('fetch_chain_entries', {
          params: { limit: PAGE_SIZE, before: cursor, search: searchParam },
        });
        setEntries(prev => {
          const existingIds = new Set(prev.map(e => e.chain_id));
          const newEntries = response.entries.filter(e => !existingIds.has(e.chain_id));
          return [...prev, ...newEntries];
        });
      }

      // Update cursor for next page
      if (response.entries.length > 0) {
        cursorRef.current = response.entries[response.entries.length - 1].chain_id;
      }
      setTotal(response.total);
      setHasMore(response.entries.length >= PAGE_SIZE);
    } catch (err) {
      logger.error('Failed to fetch chain entries', err, { component: 'useChainEntryList' });
      if (cursorRef.current === null) { setEntries([]); setTotal(0); }
    } finally {
      setLoading(false);
    }
  }, [enabled, debouncedSearch, fetchGeneration, loadMoreTrigger]);

  useEffect(() => { fetchEntries(); }, [fetchEntries]);

  const loadMore = useCallback(() => {
    if (!loading && hasMore) {
      setLoadMoreTrigger(n => n + 1);
    }
  }, [loading, hasMore]);

  const refresh = useCallback(() => {
    cursorRef.current = null;
    setFetchGeneration(g => g + 1);
  }, []);

  // Real-time: listen for terminal order events and prepend new chain entries
  useEffect(() => {
    if (!enabled) return;

    const TERMINAL_EVENTS = new Set([
      'ORDER_COMPLETED', 'ORDER_VOIDED', 'ORDERS_MERGED',
    ]);

    let cancelled = false;
    let unlisten: (() => void) | null = null;

    const setup = async () => {
      unlisten = await listen<{ event: { event_type: string } }>('order-sync', async (tauriEvent) => {
        if (cancelled) return;
        const eventType = tauriEvent.payload.event.event_type;
        if (!TERMINAL_EVENTS.has(eventType)) return;

        // Fetch the latest entries (just page 1) and prepend any new ones
        try {
          const response = await invokeApi<ChainEntryListResponse>('fetch_chain_entries', {
            params: { limit: PAGE_SIZE, offset: 0, search: debouncedSearch || undefined },
          });
          if (cancelled) return;
          setEntries(prev => {
            const existingIds = new Set(prev.map(e => e.chain_id));
            const newEntries = response.entries.filter(e => !existingIds.has(e.chain_id));
            if (newEntries.length === 0) return prev;
            return [...newEntries, ...prev];
          });
          setTotal(response.total);
        } catch {
          // Silently fail — user can manually refresh
        }
      });
    };

    setup();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [enabled, debouncedSearch]);

  return { entries, total, hasMore, loadMore, search, setSearch, loading, refresh };
};
