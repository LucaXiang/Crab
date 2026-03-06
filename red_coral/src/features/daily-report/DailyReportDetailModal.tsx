/**
 * Daily Report Detail Modal (日结报告详情)
 *
 * 显示日结报告的精简信息:
 * - 营收/退款汇总
 * - 各班次交接明细
 */

import React from 'react';
import { X, Calendar, TrendingUp, Receipt, Users, AlertTriangle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { formatCurrency } from '@/utils/currency';
import { getLocale } from '@/infrastructure/i18n';
import type { DailyReport, ShiftBreakdown } from '@/core/domain/types/api';

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

  const formatDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString(getLocale(), {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });
    } catch {
      return dateStr;
    }
  };

  const formatTime = (millis: number) => {
    return new Date(millis).toLocaleTimeString(getLocale(), {
      hour: '2-digit',
      minute: '2-digit',
    });
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
                {report.auto_generated && (
                  <span className="ml-2 px-2 py-0.5 bg-white/20 rounded text-xs">
                    {t('settings.daily_report.auto_generated')}
                  </span>
                )}
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
                  {t('settings.daily_report.summary.net_revenue')}
                </span>
              </div>
              <p className="text-2xl font-bold text-emerald-700">
                {formatCurrency(report.net_revenue)}
              </p>
            </div>

            <div className="bg-blue-50 rounded-xl p-4">
              <div className="flex items-center gap-2 text-blue-600 mb-2">
                <Receipt size={18} />
                <span className="text-sm font-medium">
                  {t('settings.daily_report.summary.total_orders')}
                </span>
              </div>
              <p className="text-2xl font-bold text-blue-700">{report.total_orders}</p>
            </div>

            <div className="bg-red-50 rounded-xl p-4">
              <div className="flex items-center gap-2 text-red-600 mb-2">
                <AlertTriangle size={18} />
                <span className="text-sm font-medium">
                  {t('settings.daily_report.summary.refunds')}
                </span>
              </div>
              <p className="text-2xl font-bold text-red-700">
                {formatCurrency(report.refund_amount)}
              </p>
              <p className="text-xs text-red-500">
                {report.refund_count} {t('settings.daily_report.refund_count_unit')}
              </p>
            </div>
          </div>

          {/* Shift Breakdowns */}
          {report.shift_breakdowns && report.shift_breakdowns.length > 0 && (
            <div className="bg-gray-50 rounded-xl p-4">
              <h3 className="text-sm font-semibold text-gray-700 mb-3 flex items-center gap-2">
                <Users size={16} />
                {t('settings.daily_report.section.shifts')}
              </h3>
              <div className="space-y-3">
                {report.shift_breakdowns.map((shift: ShiftBreakdown) => (
                  <div
                    key={shift.id}
                    className={`bg-white rounded-lg p-4 border ${
                      shift.abnormal_close ? 'border-red-200' : 'border-gray-100'
                    }`}
                  >
                    {/* Shift header */}
                    <div className="flex items-center justify-between mb-3">
                      <div className="flex items-center gap-2">
                        <span className="font-medium text-gray-800">{shift.operator_name}</span>
                        <span
                          className={`px-2 py-0.5 rounded text-xs font-medium ${
                            shift.status === 'CLOSED'
                              ? 'bg-green-100 text-green-700'
                              : 'bg-amber-100 text-amber-700'
                          }`}
                        >
                          {shift.status}
                        </span>
                        {shift.abnormal_close && (
                          <span className="px-2 py-0.5 rounded text-xs font-medium bg-red-100 text-red-700">
                            {t('settings.daily_report.shift.abnormal')}
                          </span>
                        )}
                      </div>
                      <span className="text-sm text-gray-500">
                        {formatTime(shift.start_time)}
                        {shift.end_time ? ` - ${formatTime(shift.end_time)}` : ''}
                      </span>
                    </div>

                    {/* Shift stats */}
                    <div className="grid grid-cols-4 gap-3 text-sm">
                      <div>
                        <p className="text-gray-500">{t('settings.daily_report.shift.orders')}</p>
                        <p className="font-mono font-medium">{shift.completed_orders}</p>
                      </div>
                      <div>
                        <p className="text-gray-500">{t('settings.daily_report.shift.sales')}</p>
                        <p className="font-mono font-medium text-emerald-600">
                          {formatCurrency(shift.total_sales)}
                        </p>
                      </div>
                      <div>
                        <p className="text-gray-500">
                          {t('settings.daily_report.shift.expected_cash')}
                        </p>
                        <p className="font-mono font-medium">{formatCurrency(shift.expected_cash)}</p>
                      </div>
                      <div>
                        <p className="text-gray-500">
                          {t('settings.daily_report.shift.cash_variance')}
                        </p>
                        <p
                          className={`font-mono font-medium ${
                            shift.cash_variance != null && shift.cash_variance !== 0
                              ? shift.cash_variance > 0
                                ? 'text-emerald-600'
                                : 'text-red-600'
                              : 'text-gray-500'
                          }`}
                        >
                          {shift.cash_variance != null
                            ? formatCurrency(shift.cash_variance)
                            : '-'}
                        </p>
                      </div>
                    </div>
                  </div>
                ))}
              </div>
            </div>
          )}

          {/* Additional Info */}
          <div className="bg-gray-50 rounded-xl p-4">
            <h3 className="text-sm font-semibold text-gray-700 mb-3">
              {t('settings.daily_report.section.additional')}
            </h3>
            <div className="grid grid-cols-2 gap-4 text-sm">
              <div className="flex justify-between">
                <span className="text-gray-500">{t('settings.daily_report.generated_by')}</span>
                <span>{report.generated_by_name || '-'}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-gray-500">{t('settings.daily_report.generated_at')}</span>
                <span>
                  {report.generated_at
                    ? new Date(report.generated_at).toLocaleString(getLocale())
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
