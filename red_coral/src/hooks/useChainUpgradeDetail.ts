import { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import type { ChainUpgradeDetail } from '@/core/domain/types';

interface UseChainUpgradeDetailResult {
  detail: ChainUpgradeDetail | null;
  loading: boolean;
  refresh: () => void;
}

export const useChainUpgradeDetail = (
  id: number | null,
): UseChainUpgradeDetailResult => {
  const [detail, setDetail] = useState<ChainUpgradeDetail | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchDetail = useCallback(async () => {
    if (!id) { setDetail(null); return; }
    setLoading(true);
    try {
      const data = await invokeApi<ChainUpgradeDetail>('fetch_chain_upgrade_detail', { id });
      setDetail(data);
    } catch {
      setDetail(null);
    } finally {
      setLoading(false);
    }
  }, [id]);

  useEffect(() => { fetchDetail(); }, [fetchDetail]);

  return { detail, loading, refresh: fetchDetail };
};
