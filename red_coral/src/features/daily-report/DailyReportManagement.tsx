/**
 * Daily Report Management Component (日结报告)
 *
 * 功能:
 * - 查看日结报告列表
 * - 生成当日日结
 * - 查看报告详情
 */

import React, { useEffect, useMemo, useState, useCallback } from 'react';
import { FileText, Plus, Calendar, TrendingUp, CreditCard, Receipt } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';
import { DataTable, Column } from '@/shared/components/DataTable';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { formatCurrency } from '@/utils/currency';
import type { DailyReport } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

// Extracted components
import { ManagementHeader, FilterBar } from '@/screens/Settings/components';
import { DailyReportDetailModal } from './DailyReportDetailModal';
import { GenerateReportModal } from './GenerateReportModal';

export const DailyReportManagement: React.FC = React.memo(() => {
  const { t } = useI18n();

  // State
  const [reports, setReports] = useState<DailyReport[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');

  // Modal state
  const [detailModalOpen, setDetailModalOpen] = useState(false);
  const [generateModalOpen, setGenerateModalOpen] = useState(false);
  const [selectedReport, setSelectedReport] = useState<DailyReport | null>(null);

  // Load reports
  const loadReports = useCallback(async () => {
    setLoading(true);
    try {
      const data = await getApi().listDailyReports({ limit: 100 });
      setReports(data);
    } catch (err) {
      logger.error('Failed to load daily reports', err);
      toast.error(t('settings.daily_report.load_failed'));
    } finally {
      setLoading(false);
    }
  }, [t]);

  // Load on mount
  useEffect(() => {
    loadReports();
  }, [loadReports]);

  // Filter reports
  const filteredReports = useMemo(() => {
    if (!searchQuery.trim()) return reports;
    const q = searchQuery.toLowerCase();
    return reports.filter((report) => report.business_date.includes(q));
  }, [reports, searchQuery]);

  // Handle view detail
  const handleViewDetail = useCallback((report: DailyReport) => {
    setSelectedReport(report);
    setDetailModalOpen(true);
  }, []);

  // Handle generate
  const handleGenerate = useCallback(() => {
    setGenerateModalOpen(true);
  }, []);

  // Format date
  const formatDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return date.toLocaleDateString('zh-CN', {
        year: 'numeric',
        month: '2-digit',
        day: '2-digit',
      });
    } catch {
      return dateStr;
    }
  };

  // Columns
  const columns: Column<DailyReport>[] = useMemo(
    () => [
      {
        key: 'business_date',
        header: t('settings.daily_report.header.date'),
        width: '120px',
        render: (item) => (
          <div className="flex items-center gap-2">
            <Calendar size={16} className="text-gray-400" />
            <span className="font-medium text-gray-900">{formatDate(item.business_date)}</span>
          </div>
        ),
      },
      {
        key: 'total_orders',
        header: t('settings.daily_report.header.orders'),
        width: '100px',
        align: 'center',
        render: (item) => (
          <div className="text-center">
            <span className="text-lg font-bold text-gray-800">{item.completed_orders}</span>
            <span className="text-xs text-gray-500">/{item.total_orders}</span>
          </div>
        ),
      },
      {
        key: 'total_sales',
        header: t('settings.daily_report.header.sales'),
        width: '140px',
        align: 'right',
        render: (item) => (
          <div className="flex items-center justify-end gap-2">
            <TrendingUp size={16} className="text-emerald-500" />
            <span className="font-mono font-bold text-emerald-600">
              {formatCurrency(item.total_sales)}
            </span>
          </div>
        ),
      },
      {
        key: 'total_paid',
        header: t('settings.daily_report.header.paid'),
        width: '120px',
        align: 'right',
        render: (item) => (
          <span className="font-mono text-gray-700">{formatCurrency(item.total_paid)}</span>
        ),
      },
      {
        key: 'total_unpaid',
        header: t('settings.daily_report.header.unpaid'),
        width: '120px',
        align: 'right',
        render: (item) => (
          <span
            className={`font-mono ${
              item.total_unpaid > 0 ? 'text-red-600 font-medium' : 'text-gray-500'
            }`}
          >
            {formatCurrency(item.total_unpaid)}
          </span>
        ),
      },
      {
        key: 'void_orders',
        header: t('settings.daily_report.header.void'),
        width: '100px',
        align: 'center',
        render: (item) => (
          <span className={item.void_orders > 0 ? 'text-orange-600 font-medium' : 'text-gray-400'}>
            {item.void_orders}
          </span>
        ),
      },
      {
        key: 'total_tax',
        header: t('settings.daily_report.header.tax'),
        width: '120px',
        align: 'right',
        render: (item) => (
          <span className="font-mono text-gray-600">{formatCurrency(item.total_tax)}</span>
        ),
      },
    ],
    [t]
  );

  // Check if today's report exists
  const today = new Date().toISOString().split('T')[0];
  const hasTodayReport = reports.some((r) => r.business_date === today);

  return (
    <div className="space-y-5">
      <ManagementHeader
        icon={FileText}
        title={t('settings.daily_report.title')}
        description={t('settings.daily_report.description')}
        addButtonText={t('settings.daily_report.generate')}
        onAdd={handleGenerate}
        themeColor="purple"
      />

      {/* Today's report status */}
      {!hasTodayReport && (
        <div className="bg-amber-50 border border-amber-200 rounded-xl p-4">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-amber-100 rounded-lg flex items-center justify-center">
              <Receipt className="text-amber-600" size={20} />
            </div>
            <div className="flex-1">
              <p className="font-medium text-amber-900">
                {t('settings.daily_report.no_today_report')}
              </p>
              <p className="text-sm text-amber-700">
                {t('settings.daily_report.no_today_report_hint')}
              </p>
            </div>
            <button
              onClick={handleGenerate}
              className="px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors flex items-center gap-2"
            >
              <Plus size={16} />
              {t('settings.daily_report.generate_today')}
            </button>
          </div>
        </div>
      )}

      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder={t('settings.daily_report.search_placeholder')}
        totalCount={filteredReports.length}
        countUnit={t('settings.daily_report.unit')}
        themeColor="purple"
      />

      <DataTable
        data={filteredReports}
        columns={columns}
        loading={loading}
        getRowKey={(item) => item.id || item.business_date}
        onEdit={handleViewDetail}
        emptyText={t('settings.daily_report.empty')}
        themeColor="purple"
      />

      {/* Detail Modal */}
      <DailyReportDetailModal
        open={detailModalOpen}
        report={selectedReport}
        onClose={() => setDetailModalOpen(false)}
      />

      {/* Generate Modal */}
      <GenerateReportModal
        open={generateModalOpen}
        onClose={() => setGenerateModalOpen(false)}
        onSuccess={() => {
          setGenerateModalOpen(false);
          loadReports();
        }}
      />
    </div>
  );
});
