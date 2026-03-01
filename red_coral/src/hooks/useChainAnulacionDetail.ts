import { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import type { ChainAnulacionDetail } from '@/core/domain/types';

interface UseChainAnulacionDetailResult {
  detail: ChainAnulacionDetail | null;
  loading: boolean;
  refresh: () => void;
}

export const useChainAnulacionDetail = (
  id: number | null,
): UseChainAnulacionDetailResult => {
  const [detail, setDetail] = useState<ChainAnulacionDetail | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchDetail = useCallback(async () => {
    if (!id) { setDetail(null); return; }
    setLoading(true);
    try {
      const data = await invokeApi<ChainAnulacionDetail>('fetch_chain_anulacion_detail', { id });
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
