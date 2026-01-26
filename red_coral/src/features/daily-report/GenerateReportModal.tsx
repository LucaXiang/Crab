/**
 * Generate Report Modal (生成日结报告)
 */

import React, { useState } from 'react';
import { X, FileText, Calendar, AlertCircle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';
import { toast } from '@/presentation/components/Toast';

const api = createTauriClient();

interface GenerateReportModalProps {
  open: boolean;
  onClose: () => void;
  onSuccess: () => void;
}

export const GenerateReportModal: React.FC<GenerateReportModalProps> = ({
  open,
  onClose,
  onSuccess,
}) => {
  const { t } = useI18n();

  // Form state - default to today
  const [businessDate, setBusinessDate] = useState(() => {
    return new Date().toISOString().split('T')[0];
  });
  const [note, setNote] = useState('');
  const [loading, setLoading] = useState(false);

  // Handle submit
  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!businessDate) {
      toast.error(t('settings.daily_report.generate.date_required'));
      return;
    }

    setLoading(true);
    try {
      await api.generateDailyReport({
        business_date: businessDate,
        note: note || undefined,
      });
      toast.success(t('settings.daily_report.generate.success'));
      onSuccess();
    } catch (err: any) {
      console.error('Failed to generate report:', err);
      // Check if it's a duplicate error
      if (err?.message?.includes('already exists') || err?.code === 1003) {
        toast.error(t('settings.daily_report.generate.already_exists'));
      } else {
        toast.error(t('settings.daily_report.generate.failed'));
      }
    } finally {
      setLoading(false);
    }
  };

  if (!open) return null;

  // Check if selected date is in the future
  const today = new Date().toISOString().split('T')[0];
  const isFutureDate = businessDate > today;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-black/50" onClick={onClose} />

      {/* Modal */}
      <div className="relative bg-white rounded-2xl shadow-2xl w-full max-w-md mx-4 overflow-hidden">
        {/* Header */}
        <div className="bg-violet-500 px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-3 text-white">
            <FileText size={24} />
            <h2 className="text-lg font-bold">{t('settings.daily_report.generate.title')}</h2>
          </div>
          <button
            onClick={onClose}
            className="p-1 hover:bg-white/20 rounded-lg transition-colors text-white"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('settings.daily_report.generate.date_label')}
            </label>
            <div className="relative">
              <Calendar
                size={18}
                className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400"
              />
              <input
                type="date"
                value={businessDate}
                onChange={(e) => setBusinessDate(e.target.value)}
                max={today}
                className="w-full pl-10 pr-4 py-2 border border-gray-200 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500"
              />
            </div>
            <p className="mt-1 text-xs text-gray-500">
              {t('settings.daily_report.generate.date_hint')}
            </p>
          </div>

          {/* Future date warning */}
          {isFutureDate && (
            <div className="bg-amber-50 border border-amber-200 rounded-lg p-3">
              <div className="flex gap-2">
                <AlertCircle className="text-amber-500 shrink-0" size={18} />
                <p className="text-sm text-amber-700">
                  {t('settings.daily_report.generate.future_date_warning')}
                </p>
              </div>
            </div>
          )}

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('settings.daily_report.generate.note_label')}
            </label>
            <textarea
              value={note}
              onChange={(e) => setNote(e.target.value)}
              placeholder={t('settings.daily_report.generate.note_placeholder')}
              rows={3}
              className="w-full px-4 py-2 border border-gray-200 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500 resize-none"
            />
          </div>

          {/* Info box */}
          <div className="bg-gray-50 rounded-lg p-3 text-sm text-gray-600">
            <p>{t('settings.daily_report.generate.info')}</p>
          </div>

          {/* Actions */}
          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              disabled={loading}
              className="flex-1 px-4 py-2 border border-gray-200 text-gray-700 rounded-lg hover:bg-gray-50 transition-colors disabled:opacity-50"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={loading || isFutureDate}
              className="flex-1 px-4 py-2 bg-violet-500 text-white rounded-lg hover:bg-violet-600 transition-colors disabled:opacity-50 flex items-center justify-center gap-2"
            >
              {loading ? (
                <>
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  {t('common.loading')}
                </>
              ) : (
                <>
                  <FileText size={18} />
                  {t('settings.daily_report.generate.submit')}
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
