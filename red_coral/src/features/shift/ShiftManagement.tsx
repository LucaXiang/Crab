/**
 * Shift Management Component (班次管理)
 *
 * 功能:
 * - 查看班次列表
 * - 开班/收班操作
 * - 查看班次详情
 */

import React, { useEffect, useMemo, useState, useCallback } from 'react';
import { Clock, Play, Square, AlertCircle, CheckCircle, XCircle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';
import { DataTable, Column } from '@/shared/components/DataTable';
import { toast } from '@/presentation/components/Toast';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { formatCurrency } from '@/utils/currency';
import type { Shift, ShiftStatus } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

// Extracted components
import { ManagementHeader, FilterBar } from '@/screens/Settings/components';

// Modal for opening/closing shift
import { ShiftActionModal } from './ShiftActionModal';

export const ShiftManagement: React.FC = React.memo(() => {
  const { t } = useI18n();
  const user = useAuthStore(state => state.user);

  // State
  const [shifts, setShifts] = useState<Shift[]>([]);
  const [loading, setLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [currentShift, setCurrentShift] = useState<Shift | null>(null);

  // Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [modalAction, setModalAction] = useState<'open' | 'close' | 'force_close'>('open');
  const [selectedShift, setSelectedShift] = useState<Shift | null>(null);

  // Load shifts
  const loadShifts = useCallback(async () => {
    setLoading(true);
    try {
      const [allShifts, current] = await Promise.all([
        getApi().listShifts({ limit: 50 }),
        user ? getApi().getCurrentShift(user.id) : Promise.resolve(null),
      ]);
      setShifts(allShifts);
      setCurrentShift(current);
    } catch (err) {
      console.error('Failed to load shifts:', err);
      toast.error(t('settings.shift.load_failed'));
    } finally {
      setLoading(false);
    }
  }, [user, t]);

  // Load on mount
  useEffect(() => {
    loadShifts();
  }, [loadShifts]);

  // Filter shifts
  const filteredShifts = useMemo(() => {
    if (!searchQuery.trim()) return shifts;
    const q = searchQuery.toLowerCase();
    return shifts.filter(
      (shift) =>
        shift.operator_name.toLowerCase().includes(q) ||
        shift.start_time.includes(q)
    );
  }, [shifts, searchQuery]);

  // Handle open shift
  const handleOpenShift = useCallback(() => {
    setModalAction('open');
    setSelectedShift(null);
    setModalOpen(true);
  }, []);

  // Handle close shift
  const handleCloseShift = useCallback((shift: Shift) => {
    setModalAction('close');
    setSelectedShift(shift);
    setModalOpen(true);
  }, []);

  // Handle force close
  const handleForceClose = useCallback((shift: Shift) => {
    setModalAction('force_close');
    setSelectedShift(shift);
    setModalOpen(true);
  }, []);

  // Format time
  const formatTime = (isoString: string) => {
    try {
      return new Date(isoString).toLocaleString('zh-CN', {
        month: '2-digit',
        day: '2-digit',
        hour: '2-digit',
        minute: '2-digit',
      });
    } catch {
      return isoString;
    }
  };

  // Status badge
  const StatusBadge: React.FC<{ status: ShiftStatus; abnormalClose?: boolean }> = ({
    status,
    abnormalClose,
  }) => {
    if (status === 'OPEN') {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium bg-green-100 text-green-700 whitespace-nowrap">
          <Play size={12} />
          {t('settings.shift.status.open')}
        </span>
      );
    }
    if (abnormalClose) {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium bg-orange-100 text-orange-700 whitespace-nowrap">
          <AlertCircle size={12} />
          {t('settings.shift.status.abnormal_closed')}
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1 px-2 py-1 rounded-full text-xs font-medium bg-gray-100 text-gray-600 whitespace-nowrap">
        <Square size={12} />
        {t('settings.shift.status.closed')}
      </span>
    );
  };

  // Variance indicator
  const VarianceIndicator: React.FC<{ variance: number | null }> = ({ variance }) => {
    if (variance === null || variance === undefined) return <span className="text-gray-400">-</span>;
    if (variance === 0) {
      return (
        <span className="inline-flex items-center gap-1 text-green-600">
          <CheckCircle size={14} />
          {t('settings.shift.variance.balanced')}
        </span>
      );
    }
    const isPositive = variance > 0;
    return (
      <span className={`inline-flex items-center gap-1 ${isPositive ? 'text-blue-600' : 'text-red-600'}`}>
        {isPositive ? '+' : ''}{formatCurrency(variance)}
      </span>
    );
  };

  // Columns
  const columns: Column<Shift>[] = useMemo(
    () => [
      {
        key: 'status',
        header: t('settings.shift.header.status'),
        width: '10%',
        render: (item) => <StatusBadge status={item.status} abnormalClose={item.abnormal_close} />,
      },
      {
        key: 'operator',
        header: t('settings.shift.header.operator'),
        width: '12%',
        render: (item) => (
          <span className="font-medium text-gray-900">{item.operator_name}</span>
        ),
      },
      {
        key: 'start_time',
        header: t('settings.shift.header.start_time'),
        width: '12%',
        render: (item) => (
          <span className="text-gray-600">{formatTime(item.start_time)}</span>
        ),
      },
      {
        key: 'end_time',
        header: t('settings.shift.header.end_time'),
        width: '12%',
        render: (item) => (
          <span className="text-gray-600">
            {item.end_time ? formatTime(item.end_time) : '-'}
          </span>
        ),
      },
      {
        key: 'expected_cash',
        header: t('settings.shift.header.expected_cash'),
        width: '14%',
        align: 'right',
        render: (item) => (
          <span className="font-mono text-gray-700">{formatCurrency(item.expected_cash)}</span>
        ),
      },
      {
        key: 'actual_cash',
        header: t('settings.shift.header.actual_cash'),
        width: '14%',
        align: 'right',
        render: (item) => (
          <span className="font-mono text-gray-700">
            {item.actual_cash !== null ? formatCurrency(item.actual_cash) : '-'}
          </span>
        ),
      },
      {
        key: 'variance',
        header: t('settings.shift.header.variance'),
        width: '12%',
        align: 'right',
        render: (item) => <VarianceIndicator variance={item.cash_variance} />,
      },
      {
        key: 'actions',
        header: t('common.actions'),
        width: '100px',
        align: 'right',
        render: (item) => {
          if (item.status !== 'OPEN') return null;
          // Only show actions for current user's shift
          if (user && item.operator_id !== user.id) return null;
          return (
            <div className="w-full flex items-center justify-end gap-1">
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleCloseShift(item);
                }}
                className="p-2 bg-green-50 text-green-600 rounded-lg hover:bg-green-100 transition-colors border border-green-200/50"
                title={t('settings.shift.action.close')}
              >
                <CheckCircle size={14} />
              </button>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleForceClose(item);
                }}
                className="p-2 bg-orange-50 text-orange-600 rounded-lg hover:bg-orange-100 transition-colors border border-orange-200/50"
                title={t('settings.shift.action.force_close')}
              >
                <XCircle size={14} />
              </button>
            </div>
          );
        },
      },
    ],
    [t, user, handleCloseShift, handleForceClose]
  );

  // Check if can open new shift
  const canOpenShift = !currentShift && user;

  return (
    <div className="space-y-5">
      <ManagementHeader
        icon={Clock}
        title={t('settings.shift.title')}
        description={t('settings.shift.description')}
        addButtonText={t('settings.shift.open_shift')}
        onAdd={canOpenShift ? handleOpenShift : undefined}
        themeColor="teal"
      />

      {/* Current shift info */}
      {currentShift && (
        <div className="bg-emerald-50 border border-emerald-200 rounded-xl p-4">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 bg-emerald-100 rounded-lg flex items-center justify-center">
                <Play className="text-emerald-600" size={20} />
              </div>
              <div>
                <p className="font-medium text-emerald-900">
                  {t('settings.shift.current_shift')}
                </p>
                <p className="text-sm text-emerald-700">
                  {t('settings.shift.started_at')}: {formatTime(currentShift.start_time)}
                  {' | '}
                  {t('settings.shift.expected_cash')}: {formatCurrency(currentShift.expected_cash)}
                </p>
              </div>
            </div>
            <div className="flex gap-2">
              <button
                onClick={() => handleCloseShift(currentShift)}
                className="px-4 py-2 bg-emerald-600 text-white rounded-lg hover:bg-emerald-700 transition-colors flex items-center gap-2"
              >
                <CheckCircle size={16} />
                {t('settings.shift.action.close')}
              </button>
              <button
                onClick={() => handleForceClose(currentShift)}
                className="px-4 py-2 bg-orange-500 text-white rounded-lg hover:bg-orange-600 transition-colors flex items-center gap-2"
              >
                <XCircle size={16} />
                {t('settings.shift.action.force_close')}
              </button>
            </div>
          </div>
        </div>
      )}

      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder={t('settings.shift.search_placeholder')}
        totalCount={filteredShifts.length}
        countUnit={t('settings.shift.unit')}
        themeColor="teal"
      />

      <DataTable
        data={filteredShifts}
        columns={columns}
        loading={loading}
        getRowKey={(item) => item.id || item.start_time}
        emptyText={t('settings.shift.empty')}
        themeColor="teal"
      />

      {/* Action Modal */}
      <ShiftActionModal
        open={modalOpen}
        action={modalAction}
        shift={selectedShift}
        onClose={() => setModalOpen(false)}
        onSuccess={() => {
          setModalOpen(false);
          loadShifts();
        }}
      />
    </div>
  );
});
