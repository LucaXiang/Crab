import React, { useEffect, useState } from 'react';
import { useParams, useNavigate } from 'react-router-dom';
import {
  Calendar, Clock, User, AlertTriangle, CheckCircle2,
  Banknote, ShoppingBag, XCircle, Receipt,
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getReportDetail } from '@/infrastructure/api/stats';
import { ApiError } from '@/infrastructure/api/client';
import { formatCurrency } from '@/utils/format';
import { Spinner } from '@/presentation/components/ui/Spinner';
import type { DailyReportDetail, ShiftBreakdown } from '@/core/types/stats';

function formatTime(ts: number): string {
  return new Date(ts).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function formatDate(dateStr: string): string {
  const d = new Date(dateStr + 'T00:00:00');
  const days = ['日', '一', '二', '三', '四', '五', '六'];
  return `${dateStr} (周${days[d.getDay()]})`;
}

const ShiftCard: React.FC<{ shift: ShiftBreakdown; t: (key: string) => string }> = ({ shift, t }) => {
  const isAbnormal = shift.abnormal_close;
  const isClosed = shift.status === 'CLOSED';
  const hasVariance = shift.cash_variance != null && shift.cash_variance !== 0;

  return (
    <div className="bg-white rounded-xl border border-slate-200 overflow-hidden">
      {/* Header */}
      <div className={`px-4 py-3 border-b ${isAbnormal ? 'bg-amber-50 border-amber-200' : 'bg-slate-50 border-slate-200'}`}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <User className="w-4 h-4 text-slate-500" />
            <span className="font-semibold text-slate-900">{shift.operator_name === 'UNLINKED' ? t('reports.unlinked_shift') : shift.operator_name}</span>
          </div>
          <div className="flex items-center gap-1.5">
            {isAbnormal ? (
              <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-amber-100 text-amber-700">
                <AlertTriangle className="w-3 h-3" />
                {t('reports.abnormal_close')}
              </span>
            ) : isClosed ? (
              <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-green-100 text-green-700">
                <CheckCircle2 className="w-3 h-3" />
                {t('reports.shift_closed')}
              </span>
            ) : (
              <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium bg-blue-100 text-blue-700">
                <Clock className="w-3 h-3" />
                {t('reports.shift_open')}
              </span>
            )}
          </div>
        </div>
        <div className="flex items-center gap-1 mt-1 text-xs text-slate-500">
          <Clock className="w-3 h-3" />
          {formatTime(shift.start_time)}
          {shift.end_time && <> — {formatTime(shift.end_time)}</>}
        </div>
      </div>

      {/* Stats Grid */}
      <div className="p-4 space-y-4">
        <div className="grid grid-cols-2 md:grid-cols-3 gap-3">
          <StatItem icon={<ShoppingBag className="w-4 h-4 text-blue-500" />} label={t('reports.total_sales')} value={formatCurrency(shift.total_sales)} />
          <StatItem icon={<Receipt className="w-4 h-4 text-slate-500" />} label={t('reports.orders')} value={`${shift.completed_orders}`} sub={shift.void_orders > 0 ? `${shift.void_orders} ${t('reports.voided')}` : undefined} />
          <StatItem icon={<XCircle className="w-4 h-4 text-red-400" />} label={t('reports.void_amount')} value={formatCurrency(shift.void_amount)} highlight={shift.void_amount > 0 ? 'red' : undefined} />
          <StatItem label={t('reports.discount')} value={formatCurrency(shift.total_discount)} highlight={shift.total_discount > 0 ? 'orange' : undefined} />
          <StatItem label={t('reports.surcharge')} value={formatCurrency(shift.total_surcharge)} />
          <StatItem label={t('reports.tax')} value={formatCurrency(shift.total_tax)} />
        </div>

        {/* Cash Reconciliation */}
        <div className={`rounded-lg p-3 ${hasVariance ? 'bg-amber-50 border border-amber-200' : 'bg-slate-50'}`}>
          <div className="flex items-center gap-1.5 mb-2">
            <Banknote className="w-4 h-4 text-slate-500" />
            <span className="text-xs font-medium text-slate-600">{t('reports.cash_reconciliation')}</span>
          </div>
          <div className="grid grid-cols-2 md:grid-cols-4 gap-2 text-sm">
            <div>
              <p className="text-xs text-slate-400">{t('reports.starting_cash')}</p>
              <p className="font-medium text-slate-700">{formatCurrency(shift.starting_cash)}</p>
            </div>
            <div>
              <p className="text-xs text-slate-400">{t('reports.expected_cash')}</p>
              <p className="font-medium text-slate-700">{formatCurrency(shift.expected_cash)}</p>
            </div>
            {shift.actual_cash != null && (
              <div>
                <p className="text-xs text-slate-400">{t('reports.actual_cash')}</p>
                <p className="font-medium text-slate-700">{formatCurrency(shift.actual_cash)}</p>
              </div>
            )}
            {shift.cash_variance != null && (
              <div>
                <p className="text-xs text-slate-400">{t('reports.cash_variance')}</p>
                <p className={`font-medium ${shift.cash_variance < 0 ? 'text-red-600' : shift.cash_variance > 0 ? 'text-green-600' : 'text-slate-700'}`}>
                  {shift.cash_variance > 0 ? '+' : ''}{formatCurrency(shift.cash_variance)}
                  {shift.cash_variance !== 0 && <AlertTriangle className="w-3 h-3 inline ml-1" />}
                </p>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};

const StatItem: React.FC<{
  icon?: React.ReactNode;
  label: string;
  value: string;
  sub?: string;
  highlight?: 'red' | 'orange';
}> = ({ icon, label, value, sub, highlight }) => (
  <div>
    <div className="flex items-center gap-1 mb-0.5">
      {icon}
      <p className="text-xs text-slate-400">{label}</p>
    </div>
    <p className={`font-semibold ${
      highlight === 'red' ? 'text-red-600' :
      highlight === 'orange' ? 'text-orange-500' :
      'text-slate-900'
    }`}>{value}</p>
    {sub && <p className="text-xs text-slate-400">{sub}</p>}
  </div>
);

export const ReportDetailScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const { date: dateStr } = useParams<{ date: string }>();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [report, setReport] = useState<DailyReportDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!token || !dateStr) return;
    (async () => {
      try {
        setReport(await getReportDetail(token, storeId, dateStr));
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
        setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      } finally {
        setLoading(false);
      }
    })();
  }, [token, storeId, dateStr, clearAuth, navigate, t]);

  if (loading) return <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>;
  if (error) return <div className="max-w-5xl mx-auto px-6 py-8"><div className="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div></div>;
  if (!report) return null;

  const totalCashVariance = report.shift_breakdowns.reduce((sum, s) => sum + (s.cash_variance ?? 0), 0);

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4 md:space-y-6">
      {/* Header */}
      <div>
        <div className="flex items-center gap-2">
          <h1 className="text-lg md:text-xl font-bold text-slate-900">{t('stats.daily_report')}</h1>
          <span className="text-slate-300">/</span>
          <span className="text-slate-600 font-medium flex items-center gap-1">
            <Calendar className="w-4 h-4" />
            {dateStr && formatDate(dateStr)}
          </span>
        </div>
        {report.generated_by_name && (
          <p className="text-sm text-slate-500 mt-1">
            {t('reports.generated_by')}: {report.generated_by_name}
          </p>
        )}
      </div>

      {/* Shift Cards */}
      {report.shift_breakdowns.length > 0 ? (
        <div className="space-y-4">
          {report.shift_breakdowns.map((shift, idx) => (
            <ShiftCard key={shift.shift_source_id ?? idx} shift={shift} t={t} />
          ))}
        </div>
      ) : (
        <div className="bg-white rounded-xl border border-slate-200 p-6 text-center text-sm text-slate-500">
          {t('reports.no_shift_data')}
        </div>
      )}

      {/* Daily Summary */}
      <div className="bg-white rounded-xl border border-slate-200 p-4 md:p-6">
        <h2 className="text-base font-semibold text-slate-900 mb-4">{t('reports.daily_summary')}</h2>

        <div className="grid grid-cols-2 md:grid-cols-4 gap-4 mb-6">
          <div>
            <p className="text-xs text-slate-400">{t('reports.total_sales')}</p>
            <p className="text-lg font-bold text-slate-900">{formatCurrency(report.total_sales)}</p>
          </div>
          <div>
            <p className="text-xs text-slate-400">{t('reports.orders')}</p>
            <p className="text-lg font-bold text-slate-900">{report.completed_orders}</p>
            {report.void_orders > 0 && <p className="text-xs text-red-500">{report.void_orders} {t('reports.voided')}</p>}
          </div>
          <div>
            <p className="text-xs text-slate-400">{t('reports.total_paid')}</p>
            <p className="text-lg font-bold text-slate-900">{formatCurrency(report.total_paid)}</p>
          </div>
          <div>
            <p className="text-xs text-slate-400">{t('reports.cash_variance')}</p>
            <p className={`text-lg font-bold ${totalCashVariance < 0 ? 'text-red-600' : totalCashVariance > 0 ? 'text-green-600' : 'text-slate-900'}`}>
              {totalCashVariance > 0 ? '+' : ''}{formatCurrency(totalCashVariance)}
            </p>
          </div>
        </div>

        {/* Payment Breakdown */}
        {report.payment_breakdowns.length > 0 && (
          <div className="mb-4">
            <h3 className="text-sm font-medium text-slate-700 mb-2">{t('reports.payment_methods')}</h3>
            <div className="space-y-1">
              {report.payment_breakdowns.map(pb => (
                <div key={pb.method} className="flex items-center justify-between text-sm">
                  <span className="text-slate-600">{pb.method}</span>
                  <span className="font-medium text-slate-900">{formatCurrency(pb.amount)} ({pb.count})</span>
                </div>
              ))}
            </div>
          </div>
        )}

        {/* Tax Breakdown */}
        {report.tax_breakdowns.length > 0 && (
          <div>
            <h3 className="text-sm font-medium text-slate-700 mb-2">{t('reports.tax_breakdown')}</h3>
            <div className="space-y-1">
              {report.tax_breakdowns.map(tb => (
                <div key={tb.tax_rate} className="flex items-center justify-between text-sm">
                  <span className="text-slate-600">IVA {tb.tax_rate}%</span>
                  <span className="font-medium text-slate-900">
                    {formatCurrency(tb.net_amount)} + {formatCurrency(tb.tax_amount)} = {formatCurrency(tb.gross_amount)}
                  </span>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      {/* Note */}
      {report.note && (
        <div className="bg-white rounded-xl border border-slate-200 p-4">
          <p className="text-sm text-slate-600">{report.note}</p>
        </div>
      )}
    </div>
  );
};
