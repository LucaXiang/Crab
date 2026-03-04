import React, { useState, useEffect, useCallback, useMemo, useRef } from 'react';
import { toast } from '@/presentation/components/Toast';
import { t } from '@/infrastructure/i18n';
import { getErrorMessage } from '@/utils/error';
import { useChainEntryList } from '@/hooks/useChainEntryList';
import { useHistoryOrderDetail } from '@/hooks/useHistoryOrderDetail';
import { useChainCreditNoteDetail } from '@/hooks/useChainCreditNoteDetail';
import { useChainAnulacionDetail } from '@/hooks/useChainAnulacionDetail';
import { useChainUpgradeDetail } from '@/hooks/useChainUpgradeDetail';
import { usePrinterStore } from '@/core/stores/printer/usePrinterStore';
import { reprintArchivedReceipt } from '@/core/services/order/paymentService';
import { ChainEntrySidebar } from './ChainEntrySidebar';
import { HistoryDetail } from './HistoryDetail';
import { ChainCreditNoteDetailView } from './ChainCreditNoteDetail';
import { ChainAnulacionDetailView } from './ChainAnulacionDetail';
import { ChainUpgradeDetailView } from './ChainUpgradeDetail';
import type { ChainEntryItem } from '@/core/domain/types';

interface HistoryScreenProps {
  isVisible: boolean;
  onBack: () => void;
  onOpenStatistics?: () => void;
}

type SelectedEntry =
  | { type: 'ORDER'; chainId: number; entryPk: number }
  | { type: 'CREDIT_NOTE'; chainId: number; entryPk: number }
  | { type: 'ANULACION'; chainId: number; entryPk: number }
  | { type: 'UPGRADE'; chainId: number; entryPk: number }
  | { type: 'BREAK'; chainId: number; entryPk: number };

export const HistoryScreen: React.FC<HistoryScreenProps> = ({ isVisible, onBack }) => {
  const { entries, total, hasMore, loadMore, search, setSearch, loading, refresh } =
    useChainEntryList(isVisible);

  const [selected, setSelected] = useState<SelectedEntry | null>(null);
  const userDismissed = useRef(false);

  // Auto-select first entry when list loads
  useEffect(() => {
    if (entries.length > 0 && !selected && !userDismissed.current) {
      const first = entries[0];
      setSelected({ type: first.entry_type, chainId: first.chain_id, entryPk: first.entry_pk });
    }
  }, [entries, selected]);

  const selectEntry = useCallback((entry: ChainEntryItem) => {
    userDismissed.current = false;
    setSelected({ type: entry.entry_type, chainId: entry.chain_id, entryPk: entry.entry_pk });
  }, []);

  // Jump from credit note detail to its original order
  const navigateToOrder = useCallback((orderPk: number) => {
    const orderEntry = entries.find(e => e.entry_type === 'ORDER' && e.entry_pk === orderPk);
    if (orderEntry) {
      setSelected({ type: 'ORDER', chainId: orderEntry.chain_id, entryPk: orderEntry.entry_pk });
    } else {
      // Order not in current loaded pages — still navigate, just no sidebar highlight
      setSelected({ type: 'ORDER', chainId: -1, entryPk: orderPk });
      toast.warning(t('chain_entry.order_not_in_view'));
    }
  }, [entries]);

  // Jump from order detail credit note row to credit note chain entry
  const navigateToCreditNote = useCallback((creditNotePk: number) => {
    const cnEntry = entries.find(e => e.entry_type === 'CREDIT_NOTE' && e.entry_pk === creditNotePk);
    if (cnEntry) {
      setSelected({ type: 'CREDIT_NOTE', chainId: cnEntry.chain_id, entryPk: cnEntry.entry_pk });
    } else {
      // Credit note not in current loaded pages — still navigate, just no sidebar highlight
      setSelected({ type: 'CREDIT_NOTE', chainId: -1, entryPk: creditNotePk });
      toast.warning(t('chain_entry.order_not_in_view'));
    }
  }, [entries]);

  // Find selected entry for hash display
  const selectedEntry = useMemo(
    () => entries.find(e => selected && e.chain_id === selected.chainId),
    [entries, selected],
  );

  // Order detail (only when ORDER selected)
  const { order: selectedOrder, loading: orderLoading } = useHistoryOrderDetail(
    selected?.type === 'ORDER' ? selected.entryPk : null,
  );

  // Credit note detail (only when CREDIT_NOTE selected)
  const { detail: cnDetail, loading: cnLoading } = useChainCreditNoteDetail(
    selected?.type === 'CREDIT_NOTE' ? selected.entryPk : null,
  );

  // Anulacion detail (only when ANULACION selected)
  const { detail: anulacionDetail, loading: anulacionLoading } = useChainAnulacionDetail(
    selected?.type === 'ANULACION' ? selected.entryPk : null,
  );

  // Upgrade detail (only when UPGRADE selected)
  const { detail: upgradeDetail, loading: upgradeLoading } = useChainUpgradeDetail(
    selected?.type === 'UPGRADE' ? selected.entryPk : null,
  );

  const detailLoading = selected?.type === 'ORDER' ? orderLoading
    : selected?.type === 'CREDIT_NOTE' ? cnLoading
    : selected?.type === 'UPGRADE' ? upgradeLoading
    : anulacionLoading;

  const receiptPrinter = usePrinterStore(state => state.receiptPrinter);

  const handleReprint = async () => {
    if (!selectedOrder) return;
    if (!receiptPrinter) { toast.warning(t('settings.printer.no_printer')); return; }
    try {
      await reprintArchivedReceipt(selectedOrder, receiptPrinter);
      toast.success(t('common.message.receipt_print_success'));
    } catch (error) {
      toast.error(getErrorMessage(error));
    }
  };

  // Refresh list after refund or anulacion created
  const handleRefundCreated = useCallback(() => {
    refresh();
  }, [refresh]);

  const handleAnulacionCreated = useCallback(() => {
    refresh();
  }, [refresh]);

  const handleUpgradeCreated = useCallback(() => {
    refresh();
  }, [refresh]);

  return (
    <div className="flex h-full w-full bg-gray-100 overflow-hidden font-sans">
      <ChainEntrySidebar
        entries={entries}
        selectedChainId={selected?.chainId ?? null}
        onSelect={selectEntry}
        search={search}
        setSearch={setSearch}
        hasMore={hasMore}
        loadMore={loadMore}
        loading={loading}
        onBack={onBack}
      />
      <div className="flex-1 overflow-y-auto bg-gray-50 p-4" style={{ scrollbarGutter: 'stable' }}>
        {detailLoading ? (
          <div className="h-full flex items-center justify-center">
            <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-primary-500" />
          </div>
        ) : selected?.type === 'UPGRADE' && upgradeDetail ? (
          <ChainUpgradeDetailView detail={upgradeDetail} onNavigateToOrder={navigateToOrder} />
        ) : selected?.type === 'ANULACION' && anulacionDetail ? (
          <ChainAnulacionDetailView detail={anulacionDetail} onNavigateToOrder={navigateToOrder} />
        ) : selected?.type === 'CREDIT_NOTE' && cnDetail ? (
          <ChainCreditNoteDetailView detail={cnDetail} onNavigateToOrder={navigateToOrder} />
        ) : selected?.type === 'BREAK' ? (
          <div className="h-full flex flex-col items-center justify-center text-amber-500 gap-3">
            <div className="w-16 h-16 rounded-full bg-amber-100 flex items-center justify-center">
              <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><path d="m21.73 18-8-14a2 2 0 0 0-3.48 0l-8 14A2 2 0 0 0 4 21h16a2 2 0 0 0 1.73-3"/><path d="M12 9v4"/><path d="M12 17h.01"/></svg>
            </div>
            <span className="text-lg font-bold">{t('chain_entry.break')}</span>
            <span className="text-sm text-gray-400">{t('chain_entry.break_description')}</span>
          </div>
        ) : selected?.type === 'ORDER' && selectedOrder ? (
          <HistoryDetail
            order={selectedOrder}
            onReprint={handleReprint}
            hashInfo={selectedEntry ? { prev_hash: selectedEntry.prev_hash ?? '', curr_hash: selectedEntry.curr_hash ?? '' } : undefined}
            onRefundCreated={handleRefundCreated}
            onNavigateToCreditNote={navigateToCreditNote}
            onAnulacionCreated={handleAnulacionCreated}
            onUpgradeCreated={handleUpgradeCreated}
          />
        ) : !selected ? (
          <div className="h-full flex items-center justify-center text-gray-300 text-sm">
            {t('history.no_orders')}
          </div>
        ) : null}
      </div>
    </div>
  );
};
