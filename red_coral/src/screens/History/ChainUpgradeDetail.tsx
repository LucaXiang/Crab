import React from 'react';
import type { ChainUpgradeDetail } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { FileUp, Calendar, Clock, ChevronRight, Hash, FileText, User, Mail, Phone, MapPin } from 'lucide-react';

interface ChainUpgradeDetailProps {
  detail: ChainUpgradeDetail;
  onNavigateToOrder: (orderPk: number) => void;
}

export const ChainUpgradeDetailView: React.FC<ChainUpgradeDetailProps> = ({ detail, onNavigateToOrder }) => {
  const { t } = useI18n();

  return (
    <div className="max-w-5xl mx-auto space-y-4">
      {/* Header */}
      <div className="bg-white rounded-2xl p-5 shadow-sm border border-gray-200">
        <div className="flex justify-between items-start">
          <div>
            <div className="flex items-center gap-3 mb-2">
              <div className="w-10 h-10 bg-blue-100 rounded-full flex items-center justify-center">
                <FileUp className="text-blue-600" size={20} />
              </div>
              <h1 className="text-2xl font-bold text-gray-900 font-mono">{detail.receipt_number}</h1>
              <span className="px-2 py-1 bg-blue-100 text-blue-700 text-xs font-bold rounded uppercase">
                {t('upgrade.title')}
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
              {t('checkout.amount.total')}
            </div>
            <div className="text-3xl font-bold text-blue-600">{formatCurrency(detail.total_amount)}</div>
          </div>
        </div>

        {/* 跳转到原始订单 */}
        <button
          onClick={() => onNavigateToOrder(detail.order_pk)}
          className="mt-4 flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-50 border border-gray-200 text-sm text-gray-700 hover:bg-gray-100 transition-colors w-full"
        >
          <FileText size={16} className="text-gray-400" />
          <span className="text-gray-500">{t('upgrade.original_invoice')}:</span>
          <span className="font-mono font-bold">{detail.receipt_number}</span>
          <ChevronRight size={14} className="ml-auto text-gray-400" />
        </button>
      </div>

      {/* 客户信息 */}
      {detail.customer_nif && (
        <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
          <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
            <User size={18} />
            <span>{t('upgrade.customer_info')}</span>
          </div>
          <div className="divide-y divide-gray-100">
            <DetailRow label="NIF" value={detail.customer_nif} />
            {detail.customer_nombre && <DetailRow label={t('upgrade.field.nombre')} value={detail.customer_nombre} />}
            {detail.customer_address && (
              <DetailRow label={t('upgrade.field.address')} value={
                <span className="flex items-center gap-1.5"><MapPin size={14} className="text-gray-400 shrink-0" />{detail.customer_address}</span>
              } />
            )}
            {detail.customer_email && (
              <DetailRow label={t('upgrade.field.email')} value={
                <span className="flex items-center gap-1.5"><Mail size={14} className="text-gray-400 shrink-0" />{detail.customer_email}</span>
              } />
            )}
            {detail.customer_phone && (
              <DetailRow label={t('upgrade.field.phone')} value={
                <span className="flex items-center gap-1.5"><Phone size={14} className="text-gray-400 shrink-0" />{detail.customer_phone}</span>
              } />
            )}
          </div>
        </div>
      )}

      {/* 金额明细 */}
      <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
        <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
          <FileUp size={18} />
          <span>{t('upgrade.invoice_details')}</span>
        </div>
        <div className="divide-y divide-gray-100">
          <DetailRow label={t('upgrade.field.tax')} value={formatCurrency(detail.tax)} />
          <DetailRow label={t('checkout.amount.total')} value={
            <span className="font-bold text-blue-600">{formatCurrency(detail.total_amount)}</span>
          } />
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
