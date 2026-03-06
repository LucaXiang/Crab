import React, { useState, useEffect, useCallback } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { ChevronLeft, ChevronRight } from 'lucide-react';
import { TimeRangeSelector, useTimeRange } from './TimeRangeSelector';

interface InvoiceRow {
  id: number;
  invoice_number: string;
  tipo_factura: string;
  source_type: string;
  source_pk: number;
  total: number;
  tax: number;
  aeat_status: string;
  created_at: number;
}

interface InvoiceListResponse {
  invoices: InvoiceRow[];
  total: number;
  page: number;
  page_size: number;
}

const PAGE_SIZE = 20;

const AEAT_STATUS_COLORS: Record<string, string> = {
  PENDING: 'bg-yellow-100 text-yellow-800',
  SUBMITTED: 'bg-blue-100 text-blue-800',
  ACCEPTED: 'bg-green-100 text-green-800',
  REJECTED: 'bg-red-100 text-red-800',
};

export const InvoiceList: React.FC = () => {
  const [range, setRange] = useTimeRange();
  const { from, to } = range;
  const { t } = useI18n();
  const [data, setData] = useState<InvoiceListResponse | null>(null);
  const [page, setPage] = useState(1);
  const [tipoFilter, setTipoFilter] = useState<string>('');
  const [statusFilter, setStatusFilter] = useState<string>('');
  const [loading, setLoading] = useState(false);

  const fetchInvoices = useCallback(async () => {
    setLoading(true);
    try {
      const result = await invokeApi<InvoiceListResponse>('list_invoices', {
        from,
        to,
        tipo: tipoFilter || undefined,
        aeat_status: statusFilter || undefined,
        page,
      });
      setData(result);
    } catch {
      setData(null);
    } finally {
      setLoading(false);
    }
  }, [from, to, page, tipoFilter, statusFilter]);

  useEffect(() => { fetchInvoices(); }, [fetchInvoices]);
  useEffect(() => { setPage(1); }, [from, to, tipoFilter, statusFilter]);

  const totalPages = data ? Math.ceil(data.total / PAGE_SIZE) : 0;

  const formatDate = (millis: number) => {
    return new Date(millis).toLocaleString(undefined, {
      year: 'numeric', month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit',
    });
  };

  return (
    <div className="space-y-4">
      <TimeRangeSelector value={range} onChange={setRange} />
      {/* Filters */}
      <div className="flex items-center gap-3">
        <select
          value={tipoFilter}
          onChange={e => setTipoFilter(e.target.value)}
          className="text-sm border-gray-200 rounded-md bg-white py-2 pl-3 pr-8 shadow-sm"
        >
          <option value="">{t('statistics.invoices.all_types')}</option>
          <option value="F2">F2 - {t('statistics.invoices.type_sale')}</option>
          <option value="R5">R5 - {t('statistics.invoices.type_correction')}</option>
        </select>

        <select
          value={statusFilter}
          onChange={e => setStatusFilter(e.target.value)}
          className="text-sm border-gray-200 rounded-md bg-white py-2 pl-3 pr-8 shadow-sm"
        >
          <option value="">{t('statistics.invoices.all_statuses')}</option>
          <option value="PENDING">PENDING</option>
          <option value="SUBMITTED">SUBMITTED</option>
          <option value="ACCEPTED">ACCEPTED</option>
          <option value="REJECTED">REJECTED</option>
        </select>

        {data && (
          <span className="text-sm text-gray-500 ml-auto">
            {data.total} {t('statistics.invoices.total_count')}
          </span>
        )}
      </div>

      {/* Table */}
      <div className="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden">
        <table className="w-full text-sm">
          <thead>
            <tr className="bg-gray-50 text-left text-gray-600 border-b border-gray-100">
              <th className="px-4 py-3 font-medium">{t('statistics.invoices.number')}</th>
              <th className="px-4 py-3 font-medium">{t('statistics.invoices.type')}</th>
              <th className="px-4 py-3 font-medium text-right">{t('statistics.invoices.amount')}</th>
              <th className="px-4 py-3 font-medium text-right">{t('statistics.invoices.tax')}</th>
              <th className="px-4 py-3 font-medium text-center">{t('statistics.invoices.aeat_status')}</th>
              <th className="px-4 py-3 font-medium">{t('statistics.invoices.date')}</th>
            </tr>
          </thead>
          <tbody>
            {loading && !data && (
              <tr><td colSpan={6} className="px-4 py-8 text-center text-gray-400">...</td></tr>
            )}
            {data && data.invoices.length === 0 && (
              <tr><td colSpan={6} className="px-4 py-8 text-center text-gray-400">{t('common.empty.no_data')}</td></tr>
            )}
            {data?.invoices.map(inv => (
              <tr
                key={inv.id}
                className={`border-b border-gray-50 hover:bg-gray-50 ${
                  inv.aeat_status === 'REJECTED' ? 'bg-red-50' : ''
                }`}
              >
                <td className="px-4 py-3 font-mono text-xs">{inv.invoice_number}</td>
                <td className="px-4 py-3">
                  <span className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${
                    inv.tipo_factura === 'R5' ? 'bg-orange-100 text-orange-700' : 'bg-blue-100 text-blue-700'
                  }`}>
                    {inv.tipo_factura}
                  </span>
                </td>
                <td className="px-4 py-3 text-right font-medium">{formatCurrency(inv.total)}</td>
                <td className="px-4 py-3 text-right text-gray-500">{formatCurrency(inv.tax)}</td>
                <td className="px-4 py-3 text-center">
                  <span className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${
                    AEAT_STATUS_COLORS[inv.aeat_status] || 'bg-gray-100 text-gray-600'
                  }`}>
                    {inv.aeat_status}
                  </span>
                </td>
                <td className="px-4 py-3 text-gray-500 text-xs">{formatDate(inv.created_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>

      {/* Pagination */}
      {totalPages > 1 && (
        <div className="flex items-center justify-center gap-3">
          <button
            onClick={() => setPage(p => Math.max(1, p - 1))}
            disabled={page <= 1}
            className="p-2 rounded-lg hover:bg-gray-100 disabled:opacity-30 disabled:cursor-not-allowed"
          >
            <ChevronLeft className="w-4 h-4" />
          </button>
          <span className="text-sm text-gray-600">
            {page} / {totalPages}
          </span>
          <button
            onClick={() => setPage(p => Math.min(totalPages, p + 1))}
            disabled={page >= totalPages}
            className="p-2 rounded-lg hover:bg-gray-100 disabled:opacity-30 disabled:cursor-not-allowed"
          >
            <ChevronRight className="w-4 h-4" />
          </button>
        </div>
      )}
    </div>
  );
};
