import React from 'react';
import type { ChainAnulacionDetail } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { Ban, Calendar, Clock, ChevronRight, Hash, FileText, Gift, ShoppingBag } from 'lucide-react';
import { formatCurrency } from '@/utils/currency/formatCurrency';

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
              <div className="w-10 h-10 bg-red-100 rounded-full flex items-center justify-center">
                <Ban className="text-red-600" size={20} />
              </div>
              <h1 className="text-2xl font-bold text-gray-900 font-mono">{detail.receipt_number}</h1>
              <span className="px-2 py-1 bg-red-100 text-red-700 text-xs font-bold rounded uppercase">
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
          <div className="text-right">
            <div className="text-2xl font-bold text-red-400 line-through">{formatCurrency(detail.total_amount)}</div>
          </div>
        </div>

        {/* 跳转到原始订单 */}
        <button
          onClick={() => onNavigateToOrder(detail.order_pk)}
          className="mt-4 flex items-center gap-2 px-3 py-2 rounded-lg bg-gray-50 border border-gray-200 text-sm text-gray-700 hover:bg-gray-100 transition-colors w-full"
        >
          <FileText size={16} className="text-gray-400" />
          <span className="text-gray-500">{t('anulacion.original_invoice')}:</span>
          <span className="font-mono font-bold">{detail.receipt_number}</span>
          <ChevronRight size={14} className="ml-auto text-gray-400" />
        </button>
      </div>

      {/* 原始订单菜品 */}
      {detail.items.length > 0 && (
        <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
          <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
            <ShoppingBag size={16} />
            <span>{t('history.info.order_items')}</span>
          </div>
          <div className="divide-y divide-gray-100">
            {detail.items.map(item => (
              <div key={item.instance_id} className="px-4 py-3 flex justify-between items-center">
                <div className="flex items-center gap-3 flex-1">
                  <div className={`w-8 h-8 rounded flex items-center justify-center font-bold text-sm shrink-0 ${item.is_comped ? 'bg-emerald-100 text-emerald-600' : 'bg-gray-100 text-gray-500'}`}>
                    x{item.quantity}
                  </div>
                  <div className="flex-1 min-w-0">
                    <div className="font-medium text-gray-800 flex items-center gap-2 flex-wrap">
                      <span className="text-[0.625rem] text-blue-600 bg-blue-100 font-bold font-mono px-1.5 py-0.5 rounded border border-blue-200 shrink-0">
                        #{item.instance_id.slice(-5)}
                      </span>
                      <span className="line-through text-gray-500">{item.name}</span>
                      {item.spec_name && item.spec_name !== 'default' && (
                        <span className="text-xs text-gray-400">({item.spec_name})</span>
                      )}
                      {item.is_comped && (
                        <span className="text-[0.625rem] font-bold bg-emerald-100 text-emerald-700 px-1.5 py-0.5 rounded flex items-center gap-0.5">
                          <Gift size={10} />
                          {t('checkout.comp.badge')}
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-gray-400 flex items-center gap-2">
                      <span>{formatCurrency(item.unit_price)}</span>
                      <span>/ {t('checkout.amount.unit_price')}</span>
                      {item.tax_rate > 0 && <span className="text-gray-300">IVA {item.tax_rate}%</span>}
                    </div>
                  </div>
                </div>
                <div className="font-bold text-gray-400 line-through pl-4">{formatCurrency(item.line_total)}</div>
              </div>
            ))}
          </div>
          <div className="p-4 bg-gray-50 border-t border-gray-200">
            <div className="flex justify-between items-end">
              <span className="font-bold text-gray-800">{t('history.info.total_amount')}</span>
              <span className="text-xl font-bold text-red-400 line-through">{formatCurrency(detail.total_amount)}</span>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
