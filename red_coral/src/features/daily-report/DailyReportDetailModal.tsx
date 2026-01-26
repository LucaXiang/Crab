/**
 * Daily Report Detail Modal (日结报告详情)
 *
 * 显示日结报告的详细信息:
 * - 销售汇总
 * - 税率分类 (Spain: 0%, 4%, 10%, 21%)
 * - 支付方式分类
 */

import React from 'react';
import { X, Calendar, TrendingUp, Receipt, CreditCard, Percent } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { DailyReport, TaxBreakdown, PaymentMethodBreakdown } from '@/core/domain/types/api';

interface DailyReportDetailModalProps {
  open: boolean;
  report: DailyReport | null;
  onClose: () => void;
}

export const DailyReportDetailModal: React.FC<DailyReportDetailModalProps> = ({
  open,
  report,
  onClose,
}) => {
  const { t } = useI18n();

  if (!open || !report) return null;

  const formatCurrency = (amount: number) => `¥${amount.toFixed(2)}`;

  const formatDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('zh-CN', {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });
    } catch {
      return dateStr;
    }
  };

  // Tax rate colors (Spain IVA)
  const getTaxRateColor = (rate: number) => {
    switch (rate) {
      case 0:
        return 'bg-gray-100 text-gray-700';
      case 4:
        return 'bg-green-100 text-green-700';
      case 10:
        return 'bg-blue-100 text-blue-700';
      case 21:
        return 'bg-violet-100 text-violet-700';
      default:
        return 'bg-gray-100 text-gray-700';
    }
  };

  // Payment method colors
  const getPaymentMethodColor = (method: string) => {
    const methodLower = method.toLowerCase();
    if (methodLower.includes('cash') || methodLower.includes('efectivo')) {
      return 'bg-emerald-100 text-emerald-700';
    }
    if (methodLower.includes('card') || methodLower.includes('tarjeta')) {
      return 'bg-blue-100 text-blue-700';
    }
    return 'bg-gray-100 text-gray-700';
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-white rounded-2xl shadow-2xl w-full max-w-2xl mx-4 max-h-[90vh] overflow-hidden flex flex-col">
        {/* Header */}
        <div className="bg-violet-500 px-6 py-4 flex items-center justify-between shrink-0">
          <div className="flex items-center gap-3 text-white">
            <Receipt size={24} />
            <div>
              <h2 className="text-lg font-bold">{t('settings.daily_report.detail_title')}</h2>
              <p className="text-sm text-violet-100 flex items-center gap-1">
                <Calendar size={14} />
                {formatDate(report.business_date)}
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-white/20 rounded-lg transition-colors text-white"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {/* Summary Cards */}
          <div className="grid grid-cols-3 gap-4">
            <div className="bg-emerald-50 rounded-xl p-4">
              <div className="flex items-center gap-2 text-emerald-600 mb-2">
                <TrendingUp size={18} />
                <span className="text-sm font-medium">
                  {t('settings.daily_report.summary.total_sales')}
                </span>
              </div>
              <p className="text-2xl font-bold text-emerald-700">
                {formatCurrency(report.total_sales)}
              </p>
            </div>

            <div className="bg-blue-50 rounded-xl p-4">
              <div className="flex items-center gap-2 text-blue-600 mb-2">
                <CreditCard size={18} />
                <span className="text-sm font-medium">
                  {t('settings.daily_report.summary.total_paid')}
                </span>
              </div>
              <p className="text-2xl font-bold text-blue-700">
                {formatCurrency(report.total_paid)}
              </p>
            </div>

            <div className="bg-red-50 rounded-xl p-4">
              <div className="flex items-center gap-2 text-red-600 mb-2">
                <Receipt size={18} />
                <span className="text-sm font-medium">
                  {t('settings.daily_report.summary.total_unpaid')}
                </span>
              </div>
              <p className="text-2xl font-bold text-red-700">
                {formatCurrency(report.total_unpaid)}
              </p>
            </div>
          </div>

          {/* Order Stats */}
          <div className="bg-gray-50 rounded-xl p-4">
            <h3 className="text-sm font-semibold text-gray-700 mb-3">
              {t('settings.daily_report.section.orders')}
            </h3>
            <div className="grid grid-cols-4 gap-4 text-center">
              <div>
                <p className="text-2xl font-bold text-gray-800">{report.total_orders}</p>
                <p className="text-xs text-gray-500">{t('settings.daily_report.stat.total')}</p>
              </div>
              <div>
                <p className="text-2xl font-bold text-emerald-600">{report.completed_orders}</p>
                <p className="text-xs text-gray-500">{t('settings.daily_report.stat.completed')}</p>
              </div>
              <div>
                <p className="text-2xl font-bold text-orange-600">{report.void_orders}</p>
                <p className="text-xs text-gray-500">{t('settings.daily_report.stat.void')}</p>
              </div>
              <div>
                <p className="text-2xl font-bold text-gray-600">
                  {formatCurrency(report.void_amount)}
                </p>
                <p className="text-xs text-gray-500">{t('settings.daily_report.stat.void_amount')}</p>
              </div>
            </div>
          </div>

          {/* Tax Breakdown */}
          <div className="bg-gray-50 rounded-xl p-4">
            <h3 className="text-sm font-semibold text-gray-700 mb-3 flex items-center gap-2">
              <Percent size={16} />
              {t('settings.daily_report.section.tax')}
            </h3>
            {report.tax_breakdowns && report.tax_breakdowns.length > 0 ? (
              <div className="space-y-2">
                {report.tax_breakdowns.map((tax: TaxBreakdown, index: number) => (
                  <div
                    key={index}
                    className="flex items-center justify-between p-3 bg-white rounded-lg"
                  >
                    <div className="flex items-center gap-3">
                      <span
                        className={`px-2 py-1 rounded text-sm font-bold ${getTaxRateColor(
                          tax.tax_rate
                        )}`}
                      >
                        {tax.tax_rate}%
                      </span>
                      <span className="text-sm text-gray-600">
                        {tax.order_count} {t('settings.daily_report.orders_count')}
                      </span>
                    </div>
                    <div className="text-right">
                      <p className="font-mono font-medium text-gray-800">
                        {formatCurrency(tax.gross_amount)}
                      </p>
                      <p className="text-xs text-gray-500">
                        {t('settings.daily_report.tax_amount')}: {formatCurrency(tax.tax_amount)}
                      </p>
                    </div>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-gray-500 text-center py-4">
                {t('settings.daily_report.no_tax_data')}
              </p>
            )}
          </div>

          {/* Payment Breakdown */}
          <div className="bg-gray-50 rounded-xl p-4">
            <h3 className="text-sm font-semibold text-gray-700 mb-3 flex items-center gap-2">
              <CreditCard size={16} />
              {t('settings.daily_report.section.payment')}
            </h3>
            {report.payment_breakdowns && report.payment_breakdowns.length > 0 ? (
              <div className="space-y-2">
                {report.payment_breakdowns.map((payment: PaymentMethodBreakdown, index: number) => (
                  <div
                    key={index}
                    className="flex items-center justify-between p-3 bg-white rounded-lg"
                  >
                    <div className="flex items-center gap-3">
                      <span
                        className={`px-3 py-1 rounded-full text-sm font-medium ${getPaymentMethodColor(
                          payment.method
                        )}`}
                      >
                        {payment.method}
                      </span>
                      <span className="text-sm text-gray-600">
                        {payment.count} {t('settings.daily_report.payments_count')}
                      </span>
                    </div>
                    <p className="font-mono font-medium text-gray-800">
                      {formatCurrency(payment.amount)}
                    </p>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-sm text-gray-500 text-center py-4">
                {t('settings.daily_report.no_payment_data')}
              </p>
            )}
          </div>

          {/* Additional Info */}
          <div className="bg-gray-50 rounded-xl p-4">
            <h3 className="text-sm font-semibold text-gray-700 mb-3">
              {t('settings.daily_report.section.additional')}
            </h3>
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-500">{t('settings.daily_report.total_discount')}</span>
                <span className="font-mono">{formatCurrency(report.total_discount)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">{t('settings.daily_report.total_surcharge')}</span>
                <span className="font-mono">{formatCurrency(report.total_surcharge)}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">{t('settings.daily_report.generated_by')}</span>
                <span>{report.generated_by_name || '-'}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">{t('settings.daily_report.generated_at')}</span>
                <span>
                  {report.generated_at
                    ? new Date(report.generated_at).toLocaleString('zh-CN')
                    : '-'}
                </span>
              </div>
            </div>
            {report.note && (
              <div className="mt-3 pt-3 border-t border-gray-200">
                <span className="text-gray-500 text-sm">{t('settings.daily_report.note')}: </span>
                <span className="text-gray-700 text-sm">{report.note}</span>
              </div>
            )}
          </div>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-gray-100 bg-gray-50 shrink-0">
          <button
            onClick={onClose}
            className="w-full px-4 py-2 bg-violet-500 text-white rounded-lg hover:bg-violet-600 transition-colors"
          >
            {t('common.close')}
          </button>
        </div>
      </div>
    </div>
  );
};
