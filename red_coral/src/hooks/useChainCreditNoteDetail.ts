import { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import type { ChainCreditNoteDetail } from '@/core/domain/types';

interface UseChainCreditNoteDetailResult {
  detail: ChainCreditNoteDetail | null;
  loading: boolean;
  refresh: () => void;
}

export const useChainCreditNoteDetail = (
  id: number | null,
): UseChainCreditNoteDetailResult => {
  const [detail, setDetail] = useState<ChainCreditNoteDetail | null>(null);
  const [loading, setLoading] = useState(false);

  const fetchDetail = useCallback(async () => {
    if (!id) { setDetail(null); return; }
    setLoading(true);
    try {
      const data = await invokeApi<ChainCreditNoteDetail>('fetch_chain_credit_note_detail', { id });
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
