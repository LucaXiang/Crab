import React, { useState, useEffect, useCallback } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { createTauriClient } from '@/infrastructure/api/tauri-client';
import type { AuditEntry, AuditAction, AuditChainVerification } from '@/core/domain/types/api';
import {
  Search,
  ChevronLeft,
  ChevronRight,
  Filter,
  ChevronDown,
  Loader2,
  AlertCircle,
  FileText,
  ShieldCheck,
  ShieldAlert,
  Info,
  Calendar,
} from 'lucide-react';

const PAGE_SIZE = 20;

/** action → 分组 key */
const ACTION_GROUPS: Record<string, AuditAction[]> = {
  system: ['system_startup', 'system_shutdown', 'system_abnormal_shutdown', 'system_long_downtime', 'resolve_system_issue'],
  auth: ['login_success', 'login_failed', 'logout'],
  order: ['order_completed', 'order_voided', 'order_payment_added', 'order_payment_cancelled', 'order_merged', 'order_moved', 'order_split', 'order_restored'],
  management: ['employee_created', 'employee_updated', 'employee_deleted', 'role_created', 'role_updated', 'role_deleted', 'product_price_changed', 'price_rule_changed'],
  shift: ['shift_opened', 'shift_closed'],
  config: ['print_config_changed', 'store_info_changed'],
};

/** resource_type 选项 */
const RESOURCE_TYPES = ['system', 'auth', 'order', 'employee', 'role', 'product', 'price_rule', 'shift', 'print_config', 'store_info'];

/** action 的显示颜色 */
function getActionColor(action: string): string {
  if (action.includes('deleted') || action.includes('voided') || action.includes('failed') || action.includes('abnormal')) {
    return 'bg-red-100 text-red-800';
  }
  if (action.includes('created') || action.includes('success') || action.includes('startup') || action.includes('opened')) {
    return 'bg-green-100 text-green-800';
  }
  if (action.includes('updated') || action.includes('changed') || action.includes('moved') || action.includes('merged') || action.includes('split')) {
    return 'bg-yellow-100 text-yellow-800';
  }
  return 'bg-gray-100 text-gray-800';
}

export const AuditLog: React.FC = () => {
  const { t } = useI18n();
  const api = createTauriClient();

  // Data
  const [items, setItems] = useState<AuditEntry[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Pagination
  const [page, setPage] = useState(1);
  const totalPages = Math.max(1, Math.ceil(total / PAGE_SIZE));

  // Filters
  const [dateFrom, setDateFrom] = useState('');
  const [dateTo, setDateTo] = useState('');
  const [actionFilter, setActionFilter] = useState('');
  const [resourceTypeFilter, setResourceTypeFilter] = useState('');
  const [searchQuery, setSearchQuery] = useState('');
  const [debouncedSearch, setDebouncedSearch] = useState('');

  // Chain verification
  const [verifying, setVerifying] = useState(false);
  const [verification, setVerification] = useState<AuditChainVerification | null>(null);

  // Detail expansion
  const [expandedId, setExpandedId] = useState<number | null>(null);

  // Debounce search
  useEffect(() => {
    const timer = setTimeout(() => {
      setDebouncedSearch(searchQuery);
      setPage(1);
    }, 500);
    return () => clearTimeout(timer);
  }, [searchQuery]);

  const fetchLogs = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const query: Record<string, unknown> = {
        offset: (page - 1) * PAGE_SIZE,
        limit: PAGE_SIZE,
      };
      if (dateFrom) {
        query.from = new Date(dateFrom).getTime();
      }
      if (dateTo) {
        query.to = new Date(dateTo).getTime();
      }
      if (actionFilter) query.action = actionFilter;
      if (resourceTypeFilter) query.resource_type = resourceTypeFilter;
      if (debouncedSearch) query.operator_id = debouncedSearch;

      const result = await api.listAuditLogs(query as {
        from?: number; to?: number; action?: string;
        operator_id?: string; resource_type?: string;
        offset?: number; limit?: number;
      });
      setItems(result.items);
      setTotal(result.total);
    } catch (err) {
      console.error('Failed to fetch audit logs:', err);
      setError(t('audit.error.load'));
      toast.error(t('audit.error.load'));
    } finally {
      setLoading(false);
    }
  }, [page, dateFrom, dateTo, actionFilter, resourceTypeFilter, debouncedSearch]);

  useEffect(() => {
    fetchLogs();
  }, [fetchLogs]);

  const handleVerifyChain = async () => {
    setVerifying(true);
    setVerification(null);
    try {
      const from = dateFrom ? new Date(dateFrom).getTime() : undefined;
      const to = dateTo ? new Date(dateTo).getTime() : undefined;
      const result = await api.verifyAuditChain(from, to);
      setVerification(result);
      if (result.chain_intact) {
        toast.success(t('audit.verify.intact'));
      } else {
        toast.error(t('audit.verify.broken'));
      }
    } catch (err) {
      console.error('Failed to verify chain:', err);
      toast.error(t('audit.verify.error'));
    } finally {
      setVerifying(false);
    }
  };

  const formatTimestamp = (ts: number) => {
    return new Date(ts).toLocaleString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    });
  };

  const renderContent = () => {
    if (loading && items.length === 0) {
      return (
        <div className="flex flex-col items-center justify-center h-96 text-gray-400">
          <Loader2 size={48} className="animate-spin mb-4 text-indigo-500" />
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
            onClick={fetchLogs}
            className="mt-4 px-4 py-2 bg-white border border-red-200 rounded-md shadow-sm text-red-600 hover:bg-red-50"
          >
            {t('common.action.retry')}
          </button>
        </div>
      );
    }

    if (items.length === 0) {
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
              <th className="px-4 py-3 bg-gray-50 w-12">#</th>
              <th className="px-4 py-3 bg-gray-50">{t('audit.column.time')}</th>
              <th className="px-4 py-3 bg-gray-50">{t('audit.column.action')}</th>
              <th className="px-4 py-3 bg-gray-50">{t('audit.column.resource')}</th>
              <th className="px-4 py-3 bg-gray-50">{t('audit.column.operator')}</th>
              <th className="px-4 py-3 bg-gray-50 w-16"></th>
            </tr>
          </thead>
          <tbody className="divide-y divide-gray-100">
            {items.map((item) => (
              <React.Fragment key={item.id}>
                <tr
                  className="hover:bg-indigo-50/50 transition-colors cursor-pointer"
                  onClick={() => setExpandedId(expandedId === item.id ? null : item.id)}
                >
                  <td className="px-4 py-3 text-gray-400 text-xs font-mono">{item.id}</td>
                  <td className="px-4 py-3 text-gray-500 text-xs whitespace-nowrap">
                    {formatTimestamp(item.timestamp)}
                  </td>
                  <td className="px-4 py-3">
                    <span className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-medium ${getActionColor(item.action)}`}>
                      {t(`audit.action.${item.action}`) || item.action}
                    </span>
                  </td>
                  <td className="px-4 py-3">
                    <span className="text-xs text-gray-500">{item.resource_type}</span>
                    <span className="text-gray-300 mx-1">/</span>
                    <span className="text-xs font-mono text-gray-600">{item.resource_id}</span>
                  </td>
                  <td className="px-4 py-3 text-sm">
                    {item.operator_name || <span className="text-gray-400 italic">{t('audit.system')}</span>}
                  </td>
                  <td className="px-4 py-3 text-center">
                    <ChevronDown
                      size={14}
                      className={`text-gray-400 transition-transform ${expandedId === item.id ? 'rotate-180' : ''}`}
                    />
                  </td>
                </tr>
                {expandedId === item.id && (
                  <tr>
                    <td colSpan={6} className="bg-gray-50 px-6 py-4">
                      <div className="grid grid-cols-2 gap-4 text-xs">
                        <div>
                          <span className="font-semibold text-gray-500">{t('audit.detail.details')}:</span>
                          <pre className="mt-1 p-2 bg-white rounded border border-gray-200 text-gray-600 overflow-auto max-h-40 whitespace-pre-wrap">
                            {JSON.stringify(item.details, null, 2)}
                          </pre>
                        </div>
                        <div className="space-y-2">
                          <div>
                            <span className="font-semibold text-gray-500">{t('audit.detail.operator_id')}:</span>
                            <span className="ml-2 font-mono text-gray-600">{item.operator_id || '-'}</span>
                          </div>
                          <div>
                            <span className="font-semibold text-gray-500">{t('audit.detail.prev_hash')}:</span>
                            <span className="ml-2 font-mono text-gray-400 text-[10px] break-all">{item.prev_hash}</span>
                          </div>
                          <div>
                            <span className="font-semibold text-gray-500">{t('audit.detail.curr_hash')}:</span>
                            <span className="ml-2 font-mono text-gray-400 text-[10px] break-all">{item.curr_hash}</span>
                          </div>
                        </div>
                      </div>
                    </td>
                  </tr>
                )}
              </React.Fragment>
            ))}
          </tbody>
        </table>
      </div>
    );
  };

  return (
    <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden flex flex-col h-full">
      {/* Header */}
      <div className="p-6 border-b border-gray-100 flex flex-col gap-4">
        <div className="flex justify-between items-center">
          <div>
            <h2 className="text-lg font-semibold text-gray-800">{t('audit.title')}</h2>
            <p className="text-sm text-gray-500 mt-1">
              {total} {t('audit.entries')}
            </p>
          </div>
          <button
            onClick={handleVerifyChain}
            disabled={verifying}
            className="flex items-center gap-2 px-4 py-2 bg-indigo-50 text-indigo-600 rounded-lg hover:bg-indigo-100 transition-colors text-sm font-medium disabled:opacity-50"
          >
            {verifying ? <Loader2 size={16} className="animate-spin" /> : <ShieldCheck size={16} />}
            {t('audit.verify.button')}
          </button>
        </div>

        {/* Verification result */}
        {verification && (
          <div
            className={`flex items-center gap-3 px-4 py-3 rounded-lg text-sm ${
              verification.chain_intact
                ? 'bg-green-50 text-green-700 border border-green-200'
                : 'bg-red-50 text-red-700 border border-red-200'
            }`}
          >
            {verification.chain_intact ? <ShieldCheck size={18} /> : <ShieldAlert size={18} />}
            <div>
              <span className="font-medium">
                {verification.chain_intact ? t('audit.verify.intact') : t('audit.verify.broken')}
              </span>
              <span className="ml-2 text-xs opacity-75">
                ({t('audit.verify.checked')} {verification.total_entries} {t('audit.entries')})
              </span>
              {verification.breaks.length > 0 && (
                <div className="mt-1 text-xs">
                  {verification.breaks.slice(0, 3).map((b, i) => (
                    <div key={i}>
                      #{b.entry_id}: {b.kind} — {t('audit.verify.expected')}: {b.expected.substring(0, 16)}...
                    </div>
                  ))}
                  {verification.breaks.length > 3 && (
                    <div>{t('audit.verify.more_breaks', { count: verification.breaks.length - 3 })}</div>
                  )}
                </div>
              )}
            </div>
            <button
              onClick={() => setVerification(null)}
              className="ml-auto text-xs opacity-50 hover:opacity-100"
            >
              ✕
            </button>
          </div>
        )}

        {/* Filters */}
        <div className="flex flex-wrap gap-2">
          {/* Date range */}
          <div className="flex items-center gap-1 bg-gray-50 rounded-lg border border-gray-200 px-2 py-1">
            <Calendar size={14} className="text-gray-400" />
            <input
              type="datetime-local"
              value={dateFrom}
              onChange={(e) => { setDateFrom(e.target.value); setPage(1); }}
              className="text-xs border-none bg-transparent focus:ring-0 text-gray-600 p-0.5 outline-none w-36"
            />
            <span className="text-gray-300">-</span>
            <input
              type="datetime-local"
              value={dateTo}
              onChange={(e) => { setDateTo(e.target.value); setPage(1); }}
              className="text-xs border-none bg-transparent focus:ring-0 text-gray-600 p-0.5 outline-none w-36"
            />
          </div>

          {/* Action filter */}
          <div className="relative">
            <div className="absolute inset-y-0 left-0 pl-2.5 flex items-center pointer-events-none">
              <Filter className="h-3.5 w-3.5 text-gray-400" />
            </div>
            <select
              value={actionFilter}
              onChange={(e) => { setActionFilter(e.target.value); setPage(1); }}
              className="pl-8 pr-7 py-1.5 border border-gray-200 rounded-lg text-xs focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent appearance-none bg-white cursor-pointer"
            >
              <option value="">{t('audit.filter.all_actions')}</option>
              {Object.entries(ACTION_GROUPS).map(([group, actions]) => (
                <optgroup key={group} label={t(`audit.group.${group}`)}>
                  {actions.map((a) => (
                    <option key={a} value={a}>{t(`audit.action.${a}`) || a}</option>
                  ))}
                </optgroup>
              ))}
            </select>
            <div className="absolute inset-y-0 right-0 flex items-center px-1.5 pointer-events-none">
              <ChevronDown className="w-3.5 h-3.5 text-gray-400" />
            </div>
          </div>

          {/* Resource type filter */}
          <div className="relative">
            <select
              value={resourceTypeFilter}
              onChange={(e) => { setResourceTypeFilter(e.target.value); setPage(1); }}
              className="pl-3 pr-7 py-1.5 border border-gray-200 rounded-lg text-xs focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent appearance-none bg-white cursor-pointer"
            >
              <option value="">{t('audit.filter.all_resources')}</option>
              {RESOURCE_TYPES.map((rt) => (
                <option key={rt} value={rt}>{rt}</option>
              ))}
            </select>
            <div className="absolute inset-y-0 right-0 flex items-center px-1.5 pointer-events-none">
              <ChevronDown className="w-3.5 h-3.5 text-gray-400" />
            </div>
          </div>

          {/* Search (operator) */}
          <div className="relative flex-1 min-w-[160px]">
            <Search className="absolute left-2.5 top-1/2 transform -translate-y-1/2 text-gray-400" size={14} />
            <input
              type="text"
              placeholder={t('audit.filter.search_operator')}
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              className="w-full pl-8 pr-3 py-1.5 border border-gray-200 rounded-lg text-xs focus:outline-none focus:ring-2 focus:ring-indigo-500 focus:border-transparent"
            />
          </div>
        </div>
      </div>

      {/* Table */}
      {renderContent()}

      {/* Pagination */}
      {items.length > 0 && (
        <div className="p-4 border-t border-gray-100 flex items-center justify-between bg-gray-50">
          <div className="text-sm text-gray-500">
            {t('common.selection.showing')} {((page - 1) * PAGE_SIZE) + 1} {t('common.label.to')}{' '}
            {Math.min(page * PAGE_SIZE, total)} {t('common.label.of')} {total} {t('common.label.entries')}
          </div>
          <div className="flex gap-2">
            <button
              onClick={() => setPage((p) => Math.max(1, p - 1))}
              disabled={page === 1}
              className="p-2 rounded-md border border-gray-200 bg-white disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50 text-gray-600"
            >
              <ChevronLeft size={18} />
            </button>
            <div className="flex items-center px-4 bg-white border border-gray-200 rounded-md text-sm font-medium text-gray-700">
              {t('common.label.page')} {page} {t('common.label.of')} {totalPages}
            </div>
            <button
              onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
              disabled={page === totalPages}
              className="p-2 rounded-md border border-gray-200 bg-white disabled:opacity-50 disabled:cursor-not-allowed hover:bg-gray-50 text-gray-600"
            >
              <ChevronRight size={18} />
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
