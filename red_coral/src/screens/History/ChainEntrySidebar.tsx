import React from 'react';
import type { ChainEntryItem } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { Search, Clock, ChevronRight, ArrowLeft, Undo2, Receipt, Ban, FileUp, AlertTriangle } from 'lucide-react';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import type { ChainEntryType } from '@/core/domain/types';

function formatChainNumber(displayNumber: string, entryType: ChainEntryType): string {
  switch (entryType) {
    case 'CREDIT_NOTE': return `DEV ${displayNumber}`;
    case 'ANULACION': return `ANU ${displayNumber}`;
    case 'UPGRADE': return `UPG ${displayNumber}`;
    case 'BREAK': return `BRK ${displayNumber}`;
    default: return `ORD ${displayNumber}`;
  }
}

interface ChainEntrySidebarProps {
  entries: ChainEntryItem[];
  selectedChainId: number | null;
  onSelect: (entry: ChainEntryItem) => void;
  search: string;
  setSearch: (term: string) => void;
  page: number;
  totalPages: number;
  setPage: (p: number) => void;
  loading: boolean;
  onBack: () => void;
}

export const ChainEntrySidebar: React.FC<ChainEntrySidebarProps> = ({
  entries, selectedChainId, onSelect, search, setSearch,
  page, totalPages, setPage, loading, onBack,
}) => {
  const { t } = useI18n();

  return (
    <div className="w-96 bg-white border-r border-gray-200 flex flex-col shrink-0">
      <div className="p-4 border-b border-gray-100 shrink-0">
        <div className="flex items-center gap-3 mb-4">
          <button onClick={onBack} className="p-2 -ml-2 hover:bg-gray-100 rounded-full text-gray-600 transition-colors">
            <ArrowLeft size={24} />
          </button>
          <h2 className="text-xl font-bold text-gray-800 flex items-center gap-2 flex-1">
            <Clock className="text-primary-500" size={24} />
            <span>{t('history.sidebar.title')}</span>
          </h2>
        </div>
        <div className="relative">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" size={16} />
          <input
            type="text"
            placeholder={t('history.sidebar.search')}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="w-full bg-gray-100 pl-9 pr-4 py-2.5 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-100 transition-all"
          />
        </div>
      </div>

      <div className="flex-1 overflow-y-auto relative">
        {loading && (
          <div className="absolute inset-0 bg-white/50 flex items-center justify-center z-10">
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500" />
          </div>
        )}
        {entries.length === 0 && !loading ? (
          <div className="flex flex-col items-center justify-center p-8 text-center text-gray-400 gap-4">
            <span className="text-sm">{t('history.no_orders')}</span>
          </div>
        ) : (
          <div className="divide-y divide-gray-50">
            {entries.map(entry => (
              <ChainEntryCard
                key={entry.chain_id}
                entry={entry}
                isSelected={selectedChainId === entry.chain_id}
                onSelect={onSelect}
              />
            ))}
          </div>
        )}
      </div>

      <div className="p-4 border-t border-gray-100 bg-gray-50 flex justify-center items-center text-sm">
        <button
          onClick={() => setPage(page + 1)}
          disabled={page >= totalPages || loading}
          className="px-4 py-2 rounded-lg border border-gray-200 bg-white text-gray-700 hover:bg-gray-100 disabled:opacity-50 disabled:cursor-default flex items-center gap-2"
        >
          <span>{page < totalPages ? t('history.load_more') : t('history.no_more')}</span>
          {page < totalPages && <ChevronRight size={16} />}
        </button>
      </div>
    </div>
  );
};

// ── Entry Card ─────────────────────────────────────────────────────────────

interface ChainEntryCardProps {
  entry: ChainEntryItem;
  isSelected: boolean;
  onSelect: (entry: ChainEntryItem) => void;
}

const ChainEntryCard: React.FC<ChainEntryCardProps> = React.memo(({ entry, isSelected, onSelect }) => {
  const { t } = useI18n();
  const isOrder = entry.entry_type === 'ORDER';
  const isCreditNote = entry.entry_type === 'CREDIT_NOTE';
  const isAnulacion = entry.entry_type === 'ANULACION';
  const isUpgrade = entry.entry_type === 'UPGRADE';
  const isBreak = entry.entry_type === 'BREAK';
  const isVoid = entry.status === 'VOID';
  const isMerged = entry.status === 'MERGED';

  const statusBadge = isBreak ? 'bg-amber-100 text-amber-700'
    : isAnulacion ? 'bg-gray-800 text-white'
    : isUpgrade ? 'bg-blue-100 text-blue-700'
    : isCreditNote ? 'bg-orange-100 text-orange-700'
    : isVoid ? 'bg-red-100 text-red-600'
    : isMerged ? 'bg-blue-100 text-blue-700'
    : 'bg-green-100 text-green-700';

  const statusLabel = isBreak ? t('chain_entry.break')
    : isAnulacion ? t('anulacion.title')
    : isUpgrade ? t('upgrade.title')
    : isCreditNote ? t('credit_note.title')
    : isVoid ? t('history.status.voided')
    : isMerged ? t('history.status.merged')
    : t('checkout.amount.paid_status');

  const iconBg = isBreak ? 'bg-amber-100 text-amber-600'
    : isAnulacion ? 'bg-gray-800 text-white'
    : isUpgrade ? 'bg-blue-100 text-blue-600'
    : isCreditNote ? 'bg-orange-100 text-orange-500'
    : 'bg-gray-100 text-gray-500';

  const icon = isBreak ? <AlertTriangle size={14} />
    : isUpgrade ? <FileUp size={14} />
    : isAnulacion ? <Ban size={14} />
    : isCreditNote ? <Undo2 size={14} />
    : <Receipt size={14} />;

  return (
    <button
      onClick={() => !isBreak && onSelect(entry)}
      className={`w-full p-4 text-left transition-colors flex justify-between items-start group
        ${isBreak ? 'cursor-default opacity-60' : isSelected ? 'bg-primary-50' : 'hover:bg-gray-50'}`}
    >
      <div className="flex items-start gap-3 flex-1 min-w-0">
        <div className={`w-8 h-8 rounded-full flex items-center justify-center shrink-0 ${iconBg}`}>
          {icon}
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2 mb-1">
            <span className={`font-bold text-sm ${isSelected ? 'text-primary-600' : isBreak ? 'text-amber-700' : isCreditNote ? 'text-orange-700' : isAnulacion ? 'text-gray-800' : isUpgrade ? 'text-blue-700' : 'text-gray-800'}`}>
              {formatChainNumber(entry.display_number, entry.entry_type)}
            </span>
          </div>
          <div className="flex gap-1.5 flex-wrap text-[0.625rem] items-center mb-1">
            <span className={`px-1.5 py-0.5 rounded-full font-bold ${statusBadge}`}>
              {statusLabel}
            </span>
            {(isCreditNote || isAnulacion) && entry.original_receipt && (
              <span className="text-gray-400 font-mono text-[0.6rem]">
                ← {entry.original_receipt}
              </span>
            )}
          </div>
          <div className="text-xs text-gray-400 font-mono">
            {new Date(entry.created_at).toLocaleString([], { hour12: false })}
          </div>
        </div>
      </div>

      <div className="text-right shrink-0 pl-2">
        <div className={`font-bold text-sm ${isBreak ? 'text-amber-500' : isCreditNote ? 'text-red-500' : isUpgrade ? 'text-blue-600' : isAnulacion ? 'text-gray-400' : isVoid || isMerged ? 'text-gray-400 line-through' : 'text-gray-800'}`}>
          {isBreak ? '\u2014' : entry.amount != null ? (isCreditNote ? `-${formatCurrency(entry.amount)}` : formatCurrency(entry.amount)) : '\u2014'}
        </div>
        {!isBreak && (
          <ChevronRight size={16} className={`ml-auto mt-1 transition-opacity
            ${isSelected ? 'text-primary-400 opacity-100' : 'text-gray-300 opacity-0 group-hover:opacity-100'}`} />
        )}
      </div>
    </button>
  );
});

ChainEntryCard.displayName = 'ChainEntryCard';
