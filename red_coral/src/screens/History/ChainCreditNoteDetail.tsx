import React from 'react';
import type { ChainCreditNoteDetail } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { Undo2, Calendar, Clock, CreditCard, Coins, ChevronRight, Hash } from 'lucide-react';

interface ChainCreditNoteDetailProps {
  detail: ChainCreditNoteDetail;
  onNavigateToOrder: (orderPk: number) => void;
}

export const ChainCreditNoteDetailView: React.FC<ChainCreditNoteDetailProps> = ({ detail, onNavigateToOrder }) => {
  const { t } = useI18n();
  const isCash = detail.refund_method === 'CASH';

  return (
    <div className="max-w-5xl mx-auto space-y-4">
      {/* Header */}
      <div className="bg-white rounded-2xl p-5 shadow-sm border border-gray-200">
        <div className="flex justify-between items-start">
          <div>
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 bg-red-100 rounded-full flex items-center justify-center">
                <Undo2 className="text-red-600" size={20} />
              </div>
              <h1 className="text-2xl font-bold text-gray-900 font-mono">{detail.credit_note_number}</h1>
            </div>
            <div className="flex gap-4 text-sm text-gray-500 mt-2 flex-wrap">
              <div className="flex items-center gap-1.5">
                <Calendar size={16} />
                <span>{new Date(detail.created_at).toLocaleDateString()}</span>
              </div>
              <div className="flex items-center gap-1.5">
                <Clock size={16} />
                <span>{new Date(detail.created_at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit', hour12: false })}</span>
              </div>
              {detail.operator_name && <span>{t('history.info.operator')}: {detail.operator_name}</span>}
              {detail.authorizer_name && <span>{detail.authorizer_name}</span>}
              <div className="flex items-center gap-1.5 font-mono text-xs text-gray-400">
                <Hash size={14} />
                <span title={detail.prev_hash}>{detail.prev_hash ? detail.prev_hash.slice(0, 8) + '…' : 'genesis'}</span>
                <span className="text-gray-300">→</span>
                <span title={detail.curr_hash}>{detail.curr_hash ? detail.curr_hash.slice(0, 8) + '…' : '\u2014'}</span>
              </div>
            </div>
          </div>
          <div className="text-right">
            <div className="text-sm text-gray-500 uppercase font-bold tracking-wider mb-1">
              {t('credit_note.modal.total_refund')}
            </div>
            <div className="text-3xl font-bold text-red-500">-{formatCurrency(detail.total_credit)}</div>
          </div>
        </div>

        {/* 原始订单跳转 */}
        <button
          onClick={() => onNavigateToOrder(detail.original_order_pk)}
          className="mt-4 flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-50 border border-gray-200 text-sm text-gray-700 hover:bg-gray-100 transition-colors w-full"
        >
          <span className="text-gray-500">{t('chain_entry.original_order')}:</span>
          <span className="font-mono font-bold">{detail.original_receipt}</span>
          <ChevronRight size={14} className="ml-auto text-gray-400" />
        </button>
      </div>

      {/* 退款明细 */}
      <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
        <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
          <div className={`p-2 rounded-full ${isCash ? 'bg-green-100 text-green-600' : 'bg-indigo-100 text-indigo-600'}`}>
            {isCash ? <Coins size={16} /> : <CreditCard size={16} />}
          </div>
          <span>{isCash ? t('credit_note.method.cash') : t('credit_note.method.card')}</span>
        </div>
        <div className="divide-y divide-gray-100">
          {detail.items.map(item => (
            <div key={item.id} className="px-4 py-3 flex justify-between items-center">
              <div>
                <div className="font-medium text-gray-800">{item.item_name}</div>
                <div className="text-xs text-gray-400 mt-0.5">
                  x{item.quantity} @ {formatCurrency(item.unit_price)}
                  {item.tax_rate > 0 && <span className="ml-2 text-gray-300">IVA {item.tax_rate}%</span>}
                </div>
              </div>
              <div className="font-bold text-red-500">-{formatCurrency(item.line_credit)}</div>
            </div>
          ))}
        </div>
        <div className="p-4 bg-gray-50 border-t border-gray-200">
          <div className="flex justify-between items-end">
            <span className="font-bold text-gray-800">{t('credit_note.modal.total_refund')}</span>
            <span className="text-xl font-bold text-red-500">-{formatCurrency(detail.total_credit)}</span>
          </div>
        </div>
      </div>

      {/* 原因 */}
      {detail.reason && (
        <div className="bg-white rounded-2xl shadow-sm border border-gray-200 p-4">
          <div className="text-xs font-bold text-gray-500 uppercase mb-1">{t('credit_note.modal.reason')}</div>
          <div className="text-gray-800">{detail.reason}</div>
          {detail.note && <div className="text-sm text-gray-500 mt-2">{detail.note}</div>}
        </div>
      )}
    </div>
  );
};
