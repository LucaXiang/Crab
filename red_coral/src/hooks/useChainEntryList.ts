import { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { logger } from '@/utils/logger';
import type { ChainEntryItem, ChainEntryListResponse } from '@/core/domain/types';

interface UseChainEntryListResult {
  entries: ChainEntryItem[];
  total: number;
  page: number;
  setPage: (page: number) => void;
  search: string;
  setSearch: (term: string) => void;
  loading: boolean;
  refresh: () => void;
}

const PAGE_SIZE = 50;

export const useChainEntryList = (
  enabled: boolean = true,
): UseChainEntryListResult => {
  const [entries, setEntries] = useState<ChainEntryItem[]>([]);
  const [total, setTotal] = useState(0);
  const [page, setPage] = useState(1);
  const [search, setSearch] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(search);
      setPage(1);
    }, 400);
    return () => clearTimeout(timer);
  }, [search]);

  const fetchEntries = useCallback(async () => {
    if (!enabled) return;
    setLoading(true);
    try {
      const offset = (page - 1) * PAGE_SIZE;
      const response = await invokeApi<ChainEntryListResponse>('fetch_chain_entries', {
        params: {
          limit: PAGE_SIZE,
          offset,
          search: debouncedSearch || undefined,
        },
      });
      setEntries(prev => {
        if (page === 1) return response.entries;
        const existingIds = new Set(prev.map(e => e.chain_id));
        return [...prev, ...response.entries.filter(e => !existingIds.has(e.chain_id))];
      });
      setTotal(response.total);
    } catch (err) {
      logger.error('Failed to fetch chain entries', err, { component: 'useChainEntryList' });
      if (page === 1) { setEntries([]); setTotal(0); }
    } finally {
      setLoading(false);
    }
  }, [enabled, page, debouncedSearch]);

  useEffect(() => { fetchEntries(); }, [fetchEntries]);

  return { entries, total, page, setPage, search, setSearch, loading, refresh: fetchEntries };
};
