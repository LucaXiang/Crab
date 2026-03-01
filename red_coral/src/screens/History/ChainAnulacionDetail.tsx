import React from 'react';
import type { ChainAnulacionDetail } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { Ban, Calendar, Clock, ChevronRight, Hash, FileText } from 'lucide-react';

interface ChainAnulacionDetailProps {
  detail: ChainAnulacionDetail;
  onNavigateToOrder: (orderPk: number) => void;
}

export const ChainAnulacionDetailView: React.FC<ChainAnulacionDetailProps> = ({ detail, onNavigateToOrder }) => {
  const { t } = useI18n();

  return (
    <div className="max-w-5xl mx-auto space-y-4">
      {/* Header */}
      <div className="bg-white rounded-2xl p-5 shadow-sm border border-gray-200">
        <div className="flex justify-between items-start">
          <div>
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 bg-gray-800 rounded-full flex items-center justify-center">
                <Ban className="text-white" size={20} />
              </div>
              <h1 className="text-2xl font-bold text-gray-900 font-mono">{detail.anulacion_number}</h1>
              <span className="px-2 py-1 bg-gray-800 text-white text-xs font-bold rounded uppercase">
                {t('anulacion.status.anulada')}
              </span>
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
              <div className="flex items-center gap-1.5 font-mono text-xs text-gray-400">
                <Hash size={14} />
                <span title={detail.prev_hash}>{detail.prev_hash ? detail.prev_hash.slice(0, 8) + '…' : 'genesis'}</span>
                <span className="text-gray-300">→</span>
                <span title={detail.curr_hash}>{detail.curr_hash ? detail.curr_hash.slice(0, 8) + '…' : '\u2014'}</span>
              </div>
            </div>
          </div>
        </div>

        {/* 原始发票跳转 */}
        <button
          onClick={() => onNavigateToOrder(detail.original_order_pk)}
          className="mt-4 flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-50 border border-gray-200 text-sm text-gray-700 hover:bg-gray-100 transition-colors w-full"
        >
          <FileText size={16} className="text-gray-400" />
          <span className="text-gray-500">{t('anulacion.original_invoice')}:</span>
          <span className="font-mono font-bold">{detail.original_invoice_number}</span>
          <ChevronRight size={14} className="ml-auto text-gray-400" />
        </button>
      </div>

      {/* 详情信息 */}
      <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
        <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
          <Ban size={18} />
          <span>{t('anulacion.details')}</span>
        </div>
        <div className="divide-y divide-gray-100">
          <DetailRow label={t('anulacion.serie')} value={detail.serie} />
          <DetailRow label={t('anulacion.reason_label')} value={t(`anulacion.reason.${detail.reason}`)} />
          {detail.note && <DetailRow label={t('credit_note.modal.note')} value={detail.note} />}
          <DetailRow label={t('anulacion.aeat_status_label')} value={
            <span className={`px-2 py-0.5 rounded-full text-xs font-bold ${
              detail.aeat_status === 'ACCEPTED' ? 'bg-green-100 text-green-700'
              : detail.aeat_status === 'REJECTED' ? 'bg-red-100 text-red-700'
              : 'bg-yellow-100 text-yellow-700'
            }`}>
              {detail.aeat_status}
            </span>
          } />
        </div>
      </div>

      {/* Huella */}
      <div className="bg-white rounded-2xl shadow-sm border border-gray-200 p-4">
        <div className="text-xs font-bold text-gray-500 uppercase mb-2">{t('anulacion.huella')}</div>
        <div className="font-mono text-xs text-gray-600 break-all bg-gray-50 p-3 rounded-lg">
          {detail.huella}
        </div>
      </div>
    </div>
  );
};

const DetailRow: React.FC<{ label: string; value: React.ReactNode }> = ({ label, value }) => (
  <div className="px-4 py-3 flex justify-between items-center">
    <span className="text-sm text-gray-500">{label}</span>
    <span className="text-sm font-medium text-gray-800">{typeof value === 'string' ? value : value}</span>
  </div>
);
