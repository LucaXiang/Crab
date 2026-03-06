import React, { useState, useEffect, useCallback } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { formatCurrency } from '@/utils/currency/formatCurrency';
import { DailyReportManagement } from '@/features/daily-report/DailyReportManagement';
import { TimeRangeSelector, useTimeRange } from './TimeRangeSelector';
import type { Shift } from '@/core/domain/types/api/models';

export const ReportsAndShifts: React.FC = () => {
  const [range, setRange] = useTimeRange();
  const { from, to } = range;
  const { t } = useI18n();
  const [shifts, setShifts] = useState<Shift[]>([]);
  const [loading, setLoading] = useState(false);

  const fetchShifts = useCallback(async () => {
    setLoading(true);
    try {
      // Convert millis to YYYY-MM-DD for the shifts API
      const startDate = new Date(from).toISOString().split('T')[0];
      const endDate = new Date(to).toISOString().split('T')[0];
      const result = await invokeApi<Shift[]>('list_shifts', {
        startDate,
        endDate,
        limit: 100,
      });
      setShifts(result);
    } catch {
      setShifts([]);
    } finally {
      setLoading(false);
    }
  }, [from, to]);

  useEffect(() => { fetchShifts(); }, [fetchShifts]);

  const formatTime = (millis: number | null) => {
    if (!millis) return '-';
    return new Date(millis).toLocaleString(undefined, {
      month: '2-digit', day: '2-digit',
      hour: '2-digit', minute: '2-digit',
    });
  };

  const formatDuration = (start: number, end: number | null) => {
    if (!end) return '-';
    const mins = Math.round((end - start) / 60_000);
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return `${h}h ${m}m`;
  };

  return (
    <div className="space-y-8">
      <TimeRangeSelector value={range} onChange={setRange} />
      {/* Daily Reports Section */}
      <DailyReportManagement />

      {/* Shifts Section */}
      <div>
        <h2 className="text-lg font-semibold text-gray-800 mb-4">{t('statistics.shifts.title')}</h2>
        <div className="bg-white rounded-xl border border-gray-100 shadow-sm overflow-hidden">
          <table className="w-full text-sm">
            <thead>
              <tr className="bg-gray-50 text-left text-gray-600 border-b border-gray-100">
                <th className="px-4 py-3 font-medium">{t('statistics.shifts.operator')}</th>
                <th className="px-4 py-3 font-medium">{t('statistics.shifts.status')}</th>
                <th className="px-4 py-3 font-medium">{t('statistics.shifts.start')}</th>
                <th className="px-4 py-3 font-medium">{t('statistics.shifts.end')}</th>
                <th className="px-4 py-3 font-medium">{t('statistics.shifts.duration')}</th>
                <th className="px-4 py-3 font-medium text-right">{t('statistics.shifts.starting_cash')}</th>
                <th className="px-4 py-3 font-medium text-right">{t('statistics.shifts.expected_cash')}</th>
                <th className="px-4 py-3 font-medium text-right">{t('statistics.shifts.actual_cash')}</th>
                <th className="px-4 py-3 font-medium text-right">{t('statistics.shifts.variance')}</th>
              </tr>
            </thead>
            <tbody>
              {loading && shifts.length === 0 && (
                <tr><td colSpan={9} className="px-4 py-8 text-center text-gray-400">...</td></tr>
              )}
              {!loading && shifts.length === 0 && (
                <tr><td colSpan={9} className="px-4 py-8 text-center text-gray-400">{t('common.empty.no_data')}</td></tr>
              )}
              {shifts.map(shift => (
                <tr key={shift.id} className="border-b border-gray-50 hover:bg-gray-50">
                  <td className="px-4 py-3 font-medium">{shift.operator_name}</td>
                  <td className="px-4 py-3">
                    <span className={`inline-block px-2 py-0.5 rounded text-xs font-medium ${
                      shift.status === 'OPEN'
                        ? 'bg-green-100 text-green-700'
                        : 'bg-gray-100 text-gray-600'
                    }`}>
                      {shift.status === 'OPEN' ? t('statistics.shifts.open') : t('statistics.shifts.closed')}
                    </span>
                  </td>
                  <td className="px-4 py-3 text-gray-500 text-xs">{formatTime(shift.start_time)}</td>
                  <td className="px-4 py-3 text-gray-500 text-xs">{formatTime(shift.end_time)}</td>
                  <td className="px-4 py-3 text-gray-500">{formatDuration(shift.start_time, shift.end_time)}</td>
                  <td className="px-4 py-3 text-right">{formatCurrency(shift.starting_cash)}</td>
                  <td className="px-4 py-3 text-right">{formatCurrency(shift.expected_cash)}</td>
                  <td className="px-4 py-3 text-right">
                    {shift.actual_cash != null ? formatCurrency(shift.actual_cash) : '-'}
                  </td>
                  <td className={`px-4 py-3 text-right font-medium ${
                    shift.cash_variance != null && shift.cash_variance < 0 ? 'text-red-600' : ''
                  }`}>
                    {shift.cash_variance != null ? formatCurrency(shift.cash_variance) : '-'}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
