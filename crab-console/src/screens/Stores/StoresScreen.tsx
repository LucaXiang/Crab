import React, { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { Server, Clock, ArrowRight, AlertTriangle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getStores } from '@/infrastructure/api/stores';
import { ApiError } from '@/infrastructure/api/client';
import { apiErrorMessage } from '@/infrastructure/i18n';
import { timeAgo } from '@/utils/format';
import { Spinner } from '@/presentation/components/ui/Spinner';
import type { StoreDetail } from '@/core/types/store';

export const StoresScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [stores, setStores] = useState<StoreDetail[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');

  useEffect(() => {
    if (!token) return;
    (async () => {
      try {
        setStores(await getStores(token));
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) {
          clearAuth();
          navigate('/login');
          return;
        }
        setError(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message) : t('auth.error_generic'));
      } finally {
        setLoading(false);
      }
    })();
  }, [token, clearAuth, navigate, t]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <Spinner className="w-8 h-8 text-primary-500" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="max-w-5xl mx-auto px-6 py-8">
        <div className="flex flex-col items-center justify-center py-16">
          <div className="w-14 h-14 bg-red-50 rounded-2xl flex items-center justify-center mb-4">
            <AlertTriangle className="w-7 h-7 text-red-400" />
          </div>
          <p className="text-sm text-slate-600 mb-4">{error}</p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-5xl mx-auto px-6 py-8">
      <div className="bg-white rounded-2xl border border-slate-200 p-6">
        <h2 className="font-bold text-lg text-slate-900 mb-4">{t('nav.stores')}</h2>
        {stores.length === 0 ? (
          <div className="text-center py-8">
            <Server className="w-10 h-10 text-slate-300 mx-auto mb-3" />
            <p className="text-sm text-slate-500">{t('dash.no_stores')}</p>
            <p className="text-xs text-slate-400 mt-1">{t('dash.no_stores_hint')}</p>
          </div>
        ) : (
          <div className="space-y-3">
            {stores.map(store => (
              <Link
                key={store.id}
                to={`/stores/${store.id}`}
                className="flex items-center justify-between p-4 bg-slate-50 rounded-xl border border-slate-100 hover:border-slate-200 transition-colors duration-150"
              >
                <div className="flex items-center gap-3">
                  <div className="w-10 h-10 bg-primary-100 rounded-lg flex items-center justify-center">
                    <Server className="w-5 h-5 text-primary-600" />
                  </div>
                  <div>
                    <p className="text-sm font-medium text-slate-900">{store.name ?? `Store #${store.id}`}</p>
                    <p className="text-xs text-slate-400">ID: {store.device_id.slice(0, 12)}...</p>
                  </div>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  <div className="hidden sm:flex items-center gap-1 text-xs text-slate-500">
                    <Clock className="w-3.5 h-3.5" />
                    <span>{t('dash.last_sync')}: {store.last_sync_at ? timeAgo(store.last_sync_at) : t('dash.never')}</span>
                  </div>
                  <ArrowRight className="w-4 h-4 text-slate-400" />
                </div>
              </Link>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
