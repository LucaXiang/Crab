import React, { useEffect, useState, useCallback } from 'react';
import { useNavigate } from 'react-router-dom';
import { Shield, ChevronDown } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getAuditLog } from '@/infrastructure/api/audit';
import { ApiError } from '@/infrastructure/api/client';
import { Spinner } from '@/presentation/components/ui/Spinner';
import type { AuditEntry } from '@/infrastructure/api/audit';

const ACTION_COLORS: Record<string, string> = {
  login: 'bg-blue-100 text-blue-700',
  password_changed: 'bg-amber-100 text-amber-700',
  password_reset: 'bg-orange-100 text-orange-700',
  email_changed: 'bg-purple-100 text-purple-700',
  command_created: 'bg-green-100 text-green-700',
  order_detail_fetched: 'bg-slate-100 text-slate-600',
};

function formatTs(ms: number): string {
  const d = new Date(ms);
  return d.toLocaleString();
}

export const AuditScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [entries, setEntries] = useState<AuditEntry[]>([]);
  const [page, setPage] = useState(1);
  const [loading, setLoading] = useState(true);
  const [loadingMore, setLoadingMore] = useState(false);
  const [hasMore, setHasMore] = useState(true);
  const [error, setError] = useState('');

  const load = useCallback(async (p: number, append: boolean) => {
    if (!token) return;
    if (p === 1) setLoading(true); else setLoadingMore(true);
    try {
      const data = await getAuditLog(token, p, 20);
      setEntries(prev => append ? [...prev, ...data] : data);
      setHasMore(data.length === 20);
    } catch (err) {
      if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); return; }
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setLoading(false);
      setLoadingMore(false);
    }
  }, [token, clearAuth, navigate, t]);

  useEffect(() => { load(1, false); }, [load]);

  const handleLoadMore = () => {
    const next = page + 1;
    setPage(next);
    load(next, true);
  };

  if (loading) return <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>;
  if (error) return <div className="max-w-4xl mx-auto px-6 py-8"><div className="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div></div>;

  return (
    <div className="max-w-4xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4">
      <div className="flex items-center gap-2">
        <Shield className="w-5 h-5 text-primary-500" />
        <h1 className="text-xl font-bold text-slate-900">{t('audit.title')}</h1>
      </div>

      {entries.length > 0 ? (
        <div className="bg-white rounded-2xl border border-slate-200 divide-y divide-slate-100">
          {entries.map(entry => (
            <div key={entry.id} className="px-4 md:px-5 py-3 flex flex-col sm:flex-row sm:items-start gap-1 sm:gap-4">
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1 flex-wrap">
                  <span className={`text-xs font-medium px-2 py-0.5 rounded ${ACTION_COLORS[entry.action] ?? 'bg-slate-100 text-slate-600'}`}>
                    {t(`audit.${entry.action}`) !== `audit.${entry.action}` ? t(`audit.${entry.action}`) : entry.action}
                  </span>
                  {entry.ip_address && (
                    <span className="text-xs text-slate-400">{entry.ip_address}</span>
                  )}
                </div>
                {entry.detail && (
                  <p className="text-xs text-slate-500 truncate">
                    {Object.entries(entry.detail).map(([k, v]) => `${k}: ${v}`).join(' · ')}
                  </p>
                )}
              </div>
              <span className="text-xs text-slate-400 shrink-0">{formatTs(entry.created_at)}</span>
            </div>
          ))}
        </div>
      ) : (
        <div className="bg-white rounded-2xl border border-slate-200 p-8 text-center">
          <Shield className="w-10 h-10 text-slate-300 mx-auto mb-3" />
          <p className="text-sm text-slate-500">{t('audit.empty')}</p>
        </div>
      )}

      {hasMore && entries.length > 0 && (
        <div className="text-center">
          <button onClick={handleLoadMore} disabled={loadingMore} className="px-4 py-2 text-sm text-primary-500 hover:text-primary-600 font-medium flex items-center gap-1 mx-auto disabled:opacity-50">
            {loadingMore ? <Spinner className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
            {t('audit.load_more')}
          </button>
        </div>
      )}
    </div>
  );
};
