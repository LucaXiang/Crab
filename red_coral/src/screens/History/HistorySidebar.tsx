import React from 'react';
import type { ArchivedOrderSummary } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { Search, Clock, ChevronRight, ArrowLeft } from 'lucide-react';
import { formatCurrency } from '@/utils/currency/formatCurrency';

interface HistorySidebarProps {
  orders: ArchivedOrderSummary[];
  selectedKey: string | null;
  onSelect: (id: string) => void;
  search: string;
  setSearch: (term: string) => void;
  page: number;
  totalPages: number;
  setPage: (p: number) => void;
  loading: boolean;
  onBack: () => void;
  onOpenStatistics?: () => void;
}

export const HistorySidebar: React.FC<HistorySidebarProps> = ({
  orders,
  selectedKey,
  onSelect,
  search,
  setSearch,
  page,
  totalPages,
  setPage,
  loading,
  onBack,
}) => {
  const { t } = useI18n();

  const filteredOrders = orders.filter(order => {
    // Filter out orders without a receipt number (e.g. unfinalized retail/prepaid)
    // User request: "Modify retail page logic to hide voided/prepaid receipts" & "If it is null, do not display it"
    // Update: Allow VOID / MERGED orders to be visible for audit purposes
    if (order.status === 'VOID' || order.status === 'MERGED') return true;
    if (!order.receipt_number) return false;
    return true;
  });

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
            <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
          </div>
        )}
        {filteredOrders.length === 0 ? (
          <div className="flex flex-col items-center justify-center p-8 text-center text-gray-400 gap-4">
            <span className="text-sm">{t('history.no_orders')}</span>
          </div>
        ) : (
          <div className="divide-y divide-gray-50">
            {filteredOrders.map((order) => {
              const isSelected = selectedKey === order.order_id;
              const isVoid = order.status === 'VOID';
              const isMerged = order.status === 'MERGED';
              return (
                <button
                  key={order.order_id}
                  onClick={() => onSelect(order.order_id)}
                  className={`w-full p-4 text-left transition-colors flex justify-between items-start group ${isSelected ? 'bg-primary-50' : 'hover:bg-gray-50'}`}
                >
                  <div>
                    <div className="flex items-center gap-2 mb-1">
                      <span className={`font-bold ${isSelected ? 'text-primary-600' : 'text-gray-800'}`}>
                        {order.receipt_number || order.table_name}
                      </span>
                    </div>
                    <div className="flex gap-2 text-[0.625rem] items-center mb-1">
                      {order.is_retail ? (
                        <span className="text-blue-600 bg-blue-100 px-1 rounded">{t('common.label.retail')}</span>
                      ) : (
                        order.table_name && (
                          <span className="text-blue-600 bg-blue-100 px-1 rounded">{order.table_name}</span>
                        )
                      )}
                      <span className={`px-1.5 py-0.5 rounded-full font-bold ${isVoid ? 'bg-red-100 text-red-600' : isMerged ? 'bg-blue-100 text-blue-700' : 'bg-green-100 text-green-700'}`}>
                        {isVoid
                          ? t('history.status.voided').toUpperCase()
                          : isMerged
                          ? t('history.status.merged').toUpperCase()
                          : t('checkout.amount.paid_status').toUpperCase()}
                      </span>
                      {isVoid && order.void_type && (
                        <span className={`px-1.5 py-0.5 rounded-full font-bold ${order.void_type === 'LOSS_SETTLED' ? 'bg-orange-100 text-orange-600' : 'bg-gray-200 text-gray-600'}`}>
                          {t(`history.void_type.${order.void_type}`)}
                        </span>
                      )}
                    </div>
                    <div className="text-xs text-gray-400 font-mono">
                      {new Date(order.end_time || order.start_time).toLocaleString([], { hour12: false })}
                    </div>
                  </div>
                    <div className="text-right">
                      {order.void_type === 'CANCELLED' ? (
                        <div className="font-bold text-gray-400 line-through">{formatCurrency(order.total)}</div>
                      ) : order.void_type === 'LOSS_SETTLED' ? (
                        <>
                          <div className="font-bold text-gray-800">{formatCurrency(order.total - (order.loss_amount || 0))}</div>
                          <div className="text-xs text-orange-500">{t('common.label.loss_amount')}: {formatCurrency(order.loss_amount || 0)}</div>
                        </>
                      ) : (
                        <div className={`font-bold ${isVoid || isMerged ? 'text-gray-400 line-through' : 'text-gray-800'}`}>{formatCurrency(order.total)}</div>
                      )}
                    <ChevronRight size={16} className={`ml-auto mt-1 transition-opacity ${isSelected ? 'text-primary-400 opacity-100' : 'text-gray-300 opacity-0 group-hover:opacity-100'}`} />
                  </div>
                </button>
              );
            })}
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
