import React, { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api';
import { formatCurrency } from '@/utils/currency';
import { useI18n } from '@/hooks/useI18n';
import { Undo2, Clock, Coins, CreditCard, ChevronDown, ChevronUp } from 'lucide-react';
import type { CreditNote, ArchivedOrderDetail } from '@/core/domain/types';

interface CreditNoteSectionProps {
  order: ArchivedOrderDetail;
  onRefundCreated?: () => void;
  onNavigateToCreditNote?: (creditNotePk: number) => void;
}

export const CreditNoteSection: React.FC<CreditNoteSectionProps> = ({ order, onRefundCreated, onNavigateToCreditNote }) => {
  const { t } = useI18n();
  const [creditNotes, setCreditNotes] = useState<CreditNote[]>([]);
  const [loading, setLoading] = useState(false);
  const [expanded, setExpanded] = useState(false);

  const fetchCreditNotes = useCallback(async () => {
    setLoading(true);
    try {
      const notes = await invokeApi<CreditNote[]>('fetch_credit_notes_by_order', {
        orderPk: order.order_id,
      });
      setCreditNotes(notes);
    } catch {
      setCreditNotes([]);
    } finally {
      setLoading(false);
    }
  }, [order.order_id]);

  useEffect(() => {
    fetchCreditNotes();
  }, [fetchCreditNotes]);

  // Re-fetch when a refund is created externally (from header button)
  useEffect(() => {
    if (onRefundCreated) {
      // The parent will call refresh which triggers re-render
    }
  }, [onRefundCreated]);

  const totalRefunded = creditNotes.reduce((sum, cn) => sum + cn.total_credit, 0);

  if (creditNotes.length === 0) return null;

  return (
    <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
      <button
        onClick={() => setExpanded(!expanded)}
        className="w-full p-4 border-b border-gray-100 bg-gray-50 flex items-center justify-between hover:bg-gray-100 transition-colors"
      >
        <div className="flex items-center gap-2 font-bold text-gray-700">
          <Undo2 size={18} />
          <span>{t('credit_note.title')}</span>
          <span className="text-sm font-normal text-gray-500">({creditNotes.length})</span>
          {totalRefunded > 0 && (
            <span className="text-sm font-normal text-red-500 ml-2">
              -{formatCurrency(totalRefunded)}
            </span>
          )}
        </div>
        {expanded ? <ChevronUp size={18} className="text-gray-400" /> : <ChevronDown size={18} className="text-gray-400" />}
      </button>

      {expanded && (
        loading ? (
          <div className="p-4 text-center text-gray-400 text-sm">...</div>
        ) : (
          <div className="divide-y divide-gray-100">
            {creditNotes.map((cn) => (
              <CreditNoteRow key={cn.id} creditNote={cn} t={t} onClick={onNavigateToCreditNote ? () => onNavigateToCreditNote(cn.id) : undefined} />
            ))}
          </div>
        )
      )}
    </div>
  );
};

const CreditNoteRow: React.FC<{
  creditNote: CreditNote;
  t: (key: string) => string;
  onClick?: () => void;
}> = ({ creditNote, t, onClick }) => {
  const isCash = creditNote.refund_method === 'CASH';
  return (
    <div className={`px-4 py-3 flex justify-between items-center ${onClick ? 'cursor-pointer hover:bg-gray-50 transition-colors' : ''}`} onClick={onClick}>
      <div className="flex items-center gap-3">
        <div className={`p-2 rounded-full ${isCash ? 'bg-green-100 text-green-600' : 'bg-indigo-100 text-indigo-600'}`}>
          {isCash ? <Coins size={16} /> : <CreditCard size={16} />}
        </div>
        <div>
          <div className="font-medium text-gray-800 flex items-center gap-2">
            <span className="text-xs font-mono bg-gray-100 text-gray-600 px-1.5 py-0.5 rounded">
              {creditNote.credit_note_number}
            </span>
            <span className="text-sm text-gray-500">
              {isCash ? t('credit_note.method.cash') : t('credit_note.method.card')}
            </span>
          </div>
          <div className="text-xs text-gray-400 flex items-center gap-2 mt-0.5">
            <Clock size={12} />
            <span>{new Date(creditNote.created_at).toLocaleString([], { dateStyle: 'short', timeStyle: 'short' })}</span>
            <span>·</span>
            <span>{creditNote.reason}</span>
            {creditNote.operator_name && (
              <>
                <span>·</span>
                <span>{creditNote.operator_name}</span>
              </>
            )}
          </div>
        </div>
      </div>
      <div className="font-bold text-red-500">
        -{formatCurrency(creditNote.total_credit)}
      </div>
    </div>
  );
};
