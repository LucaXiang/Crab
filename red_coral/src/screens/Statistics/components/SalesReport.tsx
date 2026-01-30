import React, { useState, useEffect } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { SalesReportItem, TimeRange } from '@/core/domain/types';
import { toast } from '@/presentation/components/Toast';
import { Download, FileText, Loader2, AlertCircle, Search, ChevronLeft, ChevronRight, ArrowUpDown, ArrowUp, ArrowDown, Filter, ChevronDown } from 'lucide-react';
import { OrderDetailModal } from '@/presentation/components/modals/OrderDetailModal';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import type { SalesReportResponse } from '@/core/domain/types';

interface SalesReportProps {
  timeRange: TimeRange;
  customStartDate?: string;
  customEndDate?: string;
}

export const SalesReport: React.FC<SalesReportProps> = ({
  timeRange,
  customStartDate,
  customEndDate,
}) => {
  const { t } = useI18n();
  const [data, setData] = useState<SalesReportItem[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Pagination & Filtering
  const [page, setPage] = useState(1);
  const [pageSize] = useState(10);
  const [totalPages, setTotalPages] = useState(1);
  const [totalCount, setTotalCount] = useState(0);
  const [sortBy, setSortBy] = useState<string>('date');
  const [sortOrder, setSortOrder] = useState<string>('desc');
  const [searchQuery, setSearchQuery] = useState('');
  const [statusFilter, setStatusFilter] = useState<string>('ALL');
  const [selectedOrderId, setSelectedOrderId] = useState<string | null>(null);

  // Debounce search
  const [debouncedSearch, setDebouncedSearch] = useState('');

  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(searchQuery);
      setPage(1); // Reset to page 1 on search
    }, 500);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  useEffect(() => {
    fetchReport();
  }, [timeRange, customStartDate, customEndDate, page, pageSize, sortBy, sortOrder, debouncedSearch, statusFilter]);

  const fetchReport = async () => {
    if (timeRange === 'custom' && (!customStartDate || !customEndDate)) {
      return;
    }

    setLoading(true);
    setError(null);

    try {
      if (!('__TAURI__' in window)) {
         console.warn('[SalesReport] Not running inside Tauri; skipping invoke');
         setLoading(false);
         return;
      }

      const params: Record<string, unknown> = { timeRange, page };
      if (customStartDate) params.startDate = customStartDate;
      if (customEndDate) params.endDate = customEndDate;
      const result = await invokeApi<SalesReportResponse>('get_sales_report', params);
      setData(result.items);
      setTotalPages(result.totalPages);
      setTotalCount(result.total);
    } catch (err) {
      console.error('Failed to fetch sales report:', err);
      setError(t('statistics.error.load'));
      toast.error(t('statistics.error.load'));
    } finally {
      setLoading(false);
    }
  };

  const handleExport = () => {
     // Placeholder for export functionality
     toast.success(t('common.message.exported'));
  };

  const handleSort = (field: string) => {
    if (sortBy === field) {
      setSortOrder(sortOrder === 'asc' ? 'desc' : 'asc');
    } else {
      setSortBy(field);
      setSortOrder('desc');
    }
  };

  const SortIcon = ({ field }: { field: string }) => {
    if (sortBy !== field) return <ArrowUpDown size={14} className="ml-1 text-gray-400" />;
    return sortOrder === 'asc' 
      ? <ArrowUp size={14} className="ml-1 text-blue-500" />
      : <ArrowDown size={14} className="ml-1 text-blue-500" />;
  };

  const renderContent = () => {
    if (loading && data.length === 0) {
      return (
        <div className="flex flex-col items-center justify-center h-96 text-gray-400">
          <Loader2 size={48} className="animate-spin mb-4 text-blue-500" />
          <p>{t('common.message.loading')}</p>
        </div>
      );
    }

    if (error) {
      return (
        <div className="flex flex-col items-center justify-center h-96 text-red-400">
          <AlertCircle size={48} className="mb-4" />
          <p>{error}</p>
          <button 
              onClick={fetchReport}
              className="mt-4 px-4 py-2 bg-white border border-red-200 rounded-md shadow-sm text-red-600 hover:bg-red-50"
          >
              {t('common.action.retry')}
          </button>
        </div>
      );
    }

    if (data.length === 0) {
      return (
        <div className="flex flex-col items-center justify-center h-96 text-gray-400">
          <FileText size={48} className="mb-4 opacity-20" />
          <p>{t('common.empty.no_data')}</p>
        </div>
      );
    }

    return (
      <div className="flex-1 overflow-auto">
        <table className="w-full text-left text-sm text-gray-600">
          <thead className="bg-gray-50 border-b border-gray-100 text-xs uppercase font-medium text-gray-500 sticky top-0 z-10">
            <tr>
              <th 
                className="px-6 py-4 bg-gray-50 cursor-pointer hover:bg-gray-100 transition-colors"
                onClick={() => handleSort('receipt_number')}
              >
                <div className="flex items-center">
                  {t('history.info.order_id')}
                  <SortIcon field="receipt_number" />
                </div>
              </th>
              <th 
                className="px-6 py-4 bg-gray-50 cursor-pointer hover:bg-gray-100 transition-colors"
                onClick={() => handleSort('date')}
              >
                <div className="flex items-center">
                  {t('common.label.date')}
                  <SortIcon field="date" />
                </div>
              </th>

              <th 
                className="px-6 py-4 text-right bg-gray-50 cursor-pointer hover:bg-gray-100 transition-colors"
                onClick={() => handleSort('total')}
              >
                <div className="flex items-center justify-end">
                  {t('common.label.total')}
                  <SortIcon field="total" />
                </div>
              </th>

              <th 
                className="px-6 py-4 bg-gray-50 cursor-pointer hover:bg-gray-100 transition-colors"
                onClick={() => handleSort('status')}
              >
                <div className="flex items-center">
                  {t('history.info.status')}
                  <SortIcon field="status" />
                </div>
              </th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-100">
            {data.map((item) => {
              const isVoided = item.status === 'VOIDED' || item.status === 'VOID';
              const isMerged = item.status === 'MERGED';
              return (
              <tr 
                key={item.order_id} 
                className={`transition-colors cursor-pointer ${
                  isVoided
                    ? 'bg-red-100 hover:bg-red-200 border-l-4 border-l-red-500'
                    : isMerged
                    ? 'bg-yellow-50 hover:bg-yellow-100 border-l-4 border-l-yellow-400'
                    : 'hover:bg-blue-50'
                }`}
                onClick={() => setSelectedOrderId(item.order_id)}
              >
                <td className="px-6 py-4 font-medium text-gray-900">
                    {item.receipt_number || `#${item.order_id}`}
                </td>
                <td className="px-6 py-4 text-gray-500">
                    {item.date}
                </td>

                <td className="px-6 py-4 text-right font-medium text-gray-900">
                    {formatCurrency(item.total)}
                </td>
                <td className="px-6 py-4">
                    <span className={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium ${
                        item.status === 'COMPLETED' ? 'bg-green-100 text-green-800' :
                        isVoided ? 'bg-red-200 text-red-900 font-bold border border-red-300' :
                        'bg-yellow-100 text-yellow-800'
                    }`}>
                        {t(`statistics.status.${item.status}`) || item.status}
                    </span>
                </td>
              </tr>
            );})}
          </tbody>
        </table>
      </div>
    );
  };

  return (
    <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden flex flex-col h-full">
      <div className="p-6 border-b border-gray-100 flex flex-col gap-4">
        <div className="flex justify-between items-center">
            <div>
                <h2 className="text-lg font-semibold text-gray-800">{t("statistics.report.sales")}</h2>
                <p className="text-sm text-gray-500 mt-1">
                    {totalCount} {t('statistics.metric.orders')}
                </p>
            </div>
            <div className="flex gap-2">
                <button 
                    onClick={handleExport}
                    className="flex items-center gap-2 px-4 py-2 bg-blue-50 text-blue-600 rounded-lg hover:bg-blue-100 transition-colors text-sm font-medium"
                >
                    <Download size={18} />
                    {t('common.action.export')}
                </button>
            </div>
        </div>

        <div className="flex gap-2">
            <div className="relative flex-1">
                <Search className="absolute left-3 top-1/2 transform -translate-y-1/2 text-gray-400" size={18} />
                <input 
                    type="text"
                    placeholder={t('common.action.search')}
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="w-full pl-10 pr-4 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent"
                />
            </div>
            
            <div className="relative">
                <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                  <Filter className="h-4 w-4 text-gray-400" />
                </div>
                <select
                  value={statusFilter}
                  onChange={(e) => setStatusFilter(e.target.value)}
                  className="pl-10 pr-8 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent appearance-none bg-white cursor-pointer"
                >
                  <option value="ALL">{t("common.status.all")}</option>
                  <option value="COMPLETED">{t("statistics.status.completed")}</option>
                  <option value="VOIDED">{t("statistics.status.voided")}</option>
                  <option value="MERGED">{t("statistics.status.merged")}</option>
                </select>
                <div className="absolute inset-y-0 right-0 flex items-center px-2 pointer-events-none">
                  <ChevronDown className="w-4 h-4 text-gray-400" />
                </div>
            </div>
        </div>
      </div>
      
      {renderContent()}

      {/* Pagination Controls */}
      {data.length > 0 && (
        <div className="p-4 border-t border-gray-100 flex items-center justify-between bg-gray-50">
            <div className="text-sm text-gray-500">
                {t('common.selection.showing')} {((page - 1) * pageSize) + 1} {t('common.label.to')} {Math.min(page * pageSize, totalCount)} {t('common.label.of')} {totalCount} {t('common.label.entries')}
            </div>
            <div className="flex gap-2">
                <button 
                    onClick={() => setPage(p => Math.max(1, p - 1))}
                    disabled={page === 1}
                    className="p-2 rounded-md border border-gray-200 bg-white disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50 text-gray-600"
                >
                    <ChevronLeft size={18} />
                </button>
                <div className="flex items-center px-4 bg-white border border-gray-200 rounded-md text-sm font-medium text-gray-700">
                    {t('common.label.page')} {page} {t('common.label.of')} {totalPages}
                </div>
                <button 
                    onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                    disabled={page === totalPages}
                    className="p-2 rounded-md border border-gray-200 bg-white disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50 text-gray-600"
                >
                    <ChevronRight size={18} />
                </button>
            </div>
        </div>
      )}
      
      <OrderDetailModal 
        isOpen={!!selectedOrderId}
        onClose={() => setSelectedOrderId(null)}
        orderId={selectedOrderId}
      />
    </div>
  );
};
