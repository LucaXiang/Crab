import React, { useState, useEffect, useCallback } from 'react';
import { invokeApi } from '@/infrastructure/api';
import { formatCurrency } from '@/utils/currency';
import { useI18n } from '@/hooks/useI18n';
import { FileText, Clock } from 'lucide-react';
import type { Invoice, AeatStatus, ArchivedOrderDetail } from '@/core/domain/types';

interface InvoiceSectionProps {
  order: ArchivedOrderDetail;
}

const AEAT_STATUS_STYLE: Record<AeatStatus, { bg: string; text: string }> = {
  PENDING: { bg: 'bg-yellow-100', text: 'text-yellow-700' },
  SUBMITTED: { bg: 'bg-blue-100', text: 'text-blue-700' },
  ACCEPTED: { bg: 'bg-green-100', text: 'text-green-700' },
  REJECTED: { bg: 'bg-red-100', text: 'text-red-700' },
};

export const InvoiceSection: React.FC<InvoiceSectionProps> = ({ order }) => {
  const { t } = useI18n();
  const [invoices, setInvoices] = useState<Invoice[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchInvoices = useCallback(async () => {
    setLoading(true);
    try {
      const data = await invokeApi<Invoice[]>('fetch_order_invoices', {
        orderPk: order.order_id,
      });
      setInvoices(data);
    } catch {
      setInvoices([]);
    } finally {
      setLoading(false);
    }
  }, [order.order_id]);

  useEffect(() => {
    fetchInvoices();
  }, [fetchInvoices]);

  if (!loading && invoices.length === 0) return null;

  return (
    <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
      <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
        <FileText size={18} />
        <span>{t('invoice.title')}</span>
      </div>

      {loading ? (
        <div className="p-4 text-center text-gray-400 text-sm">...</div>
      ) : (
        <div className="divide-y divide-gray-100">
          {invoices.map((inv) => (
            <InvoiceRow key={inv.id} invoice={inv} t={t} />
          ))}
        </div>
      )}
    </div>
  );
};

const InvoiceRow: React.FC<{
  invoice: Invoice;
  t: (key: string) => string;
}> = ({ invoice, t }) => {
  const isR5 = invoice.tipo_factura === 'R5';
  const statusStyle = AEAT_STATUS_STYLE[invoice.aeat_status];

  return (
    <div className="px-4 py-3 flex justify-between items-center">
      <div className="flex items-center gap-3">
        <div className={`p-2 rounded-full ${isR5 ? 'bg-red-100 text-red-600' : 'bg-indigo-100 text-indigo-600'}`}>
          <FileText size={16} />
        </div>
        <div>
          <div className="font-medium text-gray-800 flex items-center gap-2">
            <span className="text-xs font-mono bg-gray-100 text-gray-600 px-1.5 py-0.5 rounded">
              {invoice.invoice_number}
            </span>
            <span className={`text-[0.625rem] font-bold px-1.5 py-0.5 rounded ${isR5 ? 'bg-red-100 text-red-700' : 'bg-indigo-100 text-indigo-700'}`}>
              {invoice.tipo_factura}
            </span>
            <span className={`text-[0.625rem] font-bold px-1.5 py-0.5 rounded ${statusStyle.bg} ${statusStyle.text}`}>
              {t(`invoice.aeat_status.${invoice.aeat_status}`)}
            </span>
          </div>
          <div className="text-xs text-gray-400 flex items-center gap-2 mt-0.5">
            <Clock size={12} />
            <span>{invoice.fecha_expedicion}</span>
            {isR5 && invoice.factura_rectificada_num && (
              <>
                <span>·</span>
                <span className="text-red-500">
                  {t('invoice.rectifies')} {invoice.factura_rectificada_num}
                </span>
              </>
            )}
          </div>
        </div>
      </div>
      <div className={`font-bold ${isR5 ? 'text-red-500' : 'text-gray-800'}`}>
        {isR5 ? '-' : ''}{formatCurrency(invoice.total)}
      </div>
    </div>
  );
};
