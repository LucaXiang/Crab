import React, { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { Server, Clock, ArrowRight, AlertTriangle, MapPin } from 'lucide-react';
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
        setError(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message, err.status) : t('auth.error_generic'));
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
      <div className="bg-white rounded-2xl border border-slate-200/60 shadow-sm overflow-hidden">
        <div className="px-6 pt-6 pb-4">
          <h2 className="font-bold text-lg text-slate-900">{t('nav.stores')}</h2>
        </div>
        {stores.length === 0 ? (
          <div className="text-center py-8">
            <Server className="w-10 h-10 text-slate-300 mx-auto mb-3" />
            <p className="text-sm text-slate-500">{t('dash.no_stores')}</p>
            <p className="text-xs text-slate-400 mt-1">{t('dash.no_stores_hint')}</p>
          </div>
        ) : (
          <div className="divide-y divide-slate-100">
            {stores.map(store => (
              <Link
                key={store.id}
                to={`/stores/${store.id}`}
                className="group flex flex-col sm:flex-row sm:items-center justify-between p-5 hover:bg-slate-50/80 transition-all duration-200 gap-4"
              >
                <div className="flex items-start sm:items-center gap-4">
                  <div className="w-12 h-12 bg-gradient-to-br from-slate-100 to-slate-200 rounded-xl flex items-center justify-center shrink-0 group-hover:scale-105 transition-transform duration-200 shadow-inner">
                    <Server className="w-6 h-6 text-slate-500" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-center gap-2 mb-1">
                      <p className="text-base font-semibold text-slate-900 truncate group-hover:text-primary-600 transition-colors">
                        {store.alias}
                      </p>
                      {store.is_online && (
                        <span className="inline-flex w-2 h-2 bg-green-500 rounded-full ring-2 ring-white shadow-sm" title="Online"></span>
                      )}
                    </div>
                    <div className="flex flex-wrap items-center gap-x-3 gap-y-1 text-xs text-slate-500">
                      {store.name && <span className="truncate max-w-[200px]">{store.name}</span>}
                      <span className="font-mono bg-slate-100 px-1.5 py-0.5 rounded text-slate-600">ID: {store.device_id.slice(0, 8)}</span>
                      {store.address && (
                        <span className="flex items-center gap-1 truncate max-w-[200px]">
                          <MapPin className="w-3 h-3" />
                          {store.address}
                        </span>
                      )}
                    </div>
                  </div>
                </div>
                <div className="flex items-center justify-between sm:justify-end gap-4 pl-[4rem] sm:pl-0">
                  <div className="text-right">
                    <p className="text-xs font-medium text-slate-500 mb-0.5">{t('dash.last_sync')}</p>
                    <div className="flex items-center gap-1.5 text-xs text-slate-700 font-medium bg-slate-100/50 px-2 py-1 rounded-md">
                      <Clock className="w-3.5 h-3.5 text-slate-400" />
                      <span>{store.last_sync_at ? timeAgo(store.last_sync_at) : t('dash.never')}</span>
                    </div>
                  </div>
                  <div className="w-8 h-8 rounded-full bg-white border border-slate-200 flex items-center justify-center text-slate-400 group-hover:border-primary-200 group-hover:text-primary-500 transition-colors shadow-sm">
                    <ArrowRight className="w-4 h-4" />
                  </div>
                </div>
              </Link>
            ))}
          </div>
        )}
      </div>
    </div>
  );
};
