import React, { useEffect, useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import {
  DollarSign, ShoppingBag, Users, TrendingUp,
  Download, Server, Clock, ArrowRight, Sparkles, CreditCard,
  AlertTriangle, XCircle, CheckCircle, Upload, ShieldCheck,
  FileKey, MapPin, Phone,
} from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getProfile, createBillingPortal, createCheckout, type ProfileResponse } from '@/infrastructure/api/profile';
import { uploadP12, type P12UploadResponse } from '@/infrastructure/api/auth';
import { getStores } from '@/infrastructure/api/stores';
import { getTenantOverview } from '@/infrastructure/api/stats';
import { ApiError } from '@/infrastructure/api/client';
import { apiErrorMessage } from '@/infrastructure/i18n';
import { formatCurrency, formatDate, timeAgo } from '@/utils/format';
import { Spinner } from '@/presentation/components/ui/Spinner';
import type { StoreDetail } from '@/core/types/store';
import type { StoreOverview } from '@/core/types/stats';

function isSafeStripeUrl(url: string): boolean {
  try {
    const parsed = new URL(url);
    return parsed.protocol === 'https:' &&
      (parsed.hostname === 'checkout.stripe.com' || parsed.hostname === 'billing.stripe.com');
  } catch { return false; }
}

function getTodayRange(): { from: number; to: number } {
  const now = new Date();
  const start = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  return { from: start.getTime(), to: now.getTime() + 60000 };
}

export const DashboardScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [profile, setProfile] = useState<ProfileResponse | null>(null);
  const [stores, setStores] = useState<StoreDetail[]>([]);
  const [overview, setOverview] = useState<StoreOverview | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [billingLoading, setBillingLoading] = useState(false);
  const [checkoutLoading, setCheckoutLoading] = useState('');

  // P12 onboarding
  const [onboardStep, setOnboardStep] = useState<'p12' | 'plan'>('p12');
  const [p12File, setP12File] = useState<File | null>(null);
  const [p12Password, setP12Password] = useState('');
  const [p12Uploading, setP12Uploading] = useState(false);
  const [p12Error, setP12Error] = useState('');
  const [p12Uploaded, setP12Uploaded] = useState(false);
  const [p12Subject, setP12Subject] = useState('');
  const [p12Expires, setP12Expires] = useState<number | null>(null);
  const [isAnnual, setIsAnnual] = useState(true);

  const needsOnboarding = profile !== null && profile.profile.status === 'verified' && !profile.subscription;
  const isCanceled = profile !== null && profile.subscription?.status === 'canceled';

  useEffect(() => {
    if (!token) return;
    (async () => {
      try {
        const profileRes = await getProfile(token);
        setProfile(profileRes);

        if (profileRes.p12?.has_p12) {
          setP12Uploaded(true);
          setP12Subject(profileRes.p12.subject ?? '');
          setP12Expires(profileRes.p12.expires_at);
          setOnboardStep('plan');
        }

        if (profileRes.subscription && profileRes.subscription.status !== 'canceled') {
          const { from, to } = getTodayRange();
          const [storeList, ov] = await Promise.all([
            getStores(token),
            getTenantOverview(token, from, to),
          ]);
          setStores(storeList);
          setOverview(ov);
        }
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

  const handleBillingPortal = async () => {
    if (!token) return;
    setBillingLoading(true);
    try {
      const res = await createBillingPortal(token);
      if (!isSafeStripeUrl(res.url)) { setError('Invalid billing URL'); return; }
      window.location.href = res.url;
    } catch (err) {
      setError(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message, err.status) : t('auth.error_generic'));
    } finally {
      setBillingLoading(false);
    }
  };

  const handleP12Upload = async () => {
    if (!p12File || !token) return;
    setP12Uploading(true);
    setP12Error('');
    try {
      const res: P12UploadResponse = await uploadP12(token, p12File, p12Password);
      setP12Uploaded(true);
      setP12Subject(res.common_name);
      setP12Expires(res.expires_at);
      setOnboardStep('plan');
    } catch (err) {
      setP12Error(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message, err.status) : t('auth.error_generic'));
    } finally {
      setP12Uploading(false);
    }
  };

  const handleChoosePlan = async (planBase: string) => {
    if (!token) return;
    const plan = isAnnual ? `${planBase}_yearly` : planBase;
    setCheckoutLoading(planBase);
    setError('');
    try {
      const res = await createCheckout(token, plan);
      if (!isSafeStripeUrl(res.checkout_url)) { setError('Invalid checkout URL'); return; }
      window.location.href = res.checkout_url;
    } catch (err) {
      setError(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message, err.status) : t('auth.error_generic'));
    } finally {
      setCheckoutLoading('');
    }
  };

  if (loading) {
    return (
      <div className="flex items-center justify-center py-20">
        <Spinner className="w-8 h-8 text-primary-500" />
      </div>
    );
  }

  if (error && !profile) {
    return (
      <div className="max-w-5xl mx-auto px-6 py-8">
        <div className="flex flex-col items-center justify-center py-16">
          <div className="w-14 h-14 bg-red-50 rounded-2xl flex items-center justify-center mb-4">
            <AlertTriangle className="w-7 h-7 text-red-400" />
          </div>
          <p className="text-sm text-slate-600 mb-4">{error}</p>
          <button
            onClick={() => window.location.reload()}
            className="text-sm font-medium text-primary-600 hover:text-primary-700 transition-colors cursor-pointer"
          >
            {t('auth.error_retry') || 'Retry'}
          </button>
        </div>
      </div>
    );
  }

  if (needsOnboarding) {
    return (
      <div className="max-w-5xl mx-auto px-6 py-8 space-y-6">
        {/* Step indicator */}
        <div className="flex items-center justify-center gap-4 mb-6">
          <button
            onClick={() => { if (p12Uploaded) setOnboardStep('p12'); }}
            className={`flex items-center gap-2 text-sm font-medium cursor-pointer ${
              onboardStep === 'p12' ? 'text-primary-600' : p12Uploaded ? 'text-green-600' : 'text-slate-400'
            }`}
          >
            <span className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold ${
              onboardStep === 'p12' ? 'bg-primary-100 text-primary-600' : p12Uploaded ? 'bg-green-100 text-green-600' : 'bg-slate-100 text-slate-400'
            }`}>
              {p12Uploaded && onboardStep !== 'p12' ? <CheckCircle className="w-4 h-4" /> : <>1</>}
            </span>
            {t('onboard.step_certificate')}
          </button>
          <div className={`w-8 h-px ${p12Uploaded ? 'bg-green-300' : 'bg-slate-200'}`} />
          <span className={`flex items-center gap-2 text-sm font-medium ${
            onboardStep === 'plan' ? 'text-primary-600' : 'text-slate-400'
          }`}>
            <span className={`w-7 h-7 rounded-full flex items-center justify-center text-xs font-bold ${
              onboardStep === 'plan' ? 'bg-primary-100 text-primary-600' : 'bg-slate-100 text-slate-400'
            }`}>2</span>
            {t('onboard.step_plan')}
          </span>
        </div>

        {onboardStep === 'p12' ? (
          <P12Step
            t={t}
            p12Uploaded={p12Uploaded}
            p12Subject={p12Subject}
            p12Expires={p12Expires}
            p12File={p12File}
            p12Password={p12Password}
            p12Uploading={p12Uploading}
            p12Error={p12Error}
            onFileChange={setP12File}
            onPasswordChange={setP12Password}
            onUpload={handleP12Upload}
            onContinue={() => setOnboardStep('plan')}
            onReset={() => { setP12Uploaded(false); setP12File(null); setP12Password(''); }}
          />
        ) : (
          <PlanStep
            t={t}
            isAnnual={isAnnual}
            onToggleAnnual={() => setIsAnnual(!isAnnual)}
            checkoutLoading={checkoutLoading}
            error={error}
            onChoosePlan={handleChoosePlan}
          />
        )}
      </div>
    );
  }

  if (isCanceled && profile) {
    return (
      <div className="max-w-5xl mx-auto px-6 py-8 space-y-6">
        <div className="bg-white rounded-2xl border border-red-200 p-6">
          <div className="flex items-start justify-between">
            <div>
              <h2 className="font-bold text-lg text-slate-900 mb-1">{t('dash.subscription')}</h2>
              <p className="text-sm text-slate-500">{profile.profile.email}</p>
            </div>
            <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-full text-xs font-medium text-red-600 bg-red-50">
              <XCircle className="w-3.5 h-3.5" />
              {t('dash.cancelled')}
            </span>
          </div>
          <div className="mt-4 grid grid-cols-2 gap-4">
            <div>
              <p className="text-xs text-slate-400 mb-0.5">{t('dash.plan')}</p>
              <p className="text-sm font-semibold text-slate-900 capitalize">{profile.subscription?.plan}</p>
            </div>
          </div>
          <div className="mt-4">
            <button
              onClick={handleBillingPortal}
              disabled={billingLoading}
              className="inline-flex items-center gap-1.5 bg-primary-500 hover:bg-primary-600 text-white font-medium text-sm px-4 py-2 rounded-lg transition-colors duration-150 cursor-pointer disabled:opacity-50"
            >
              <CreditCard className="w-4 h-4" />
              <span>{t('dash.manage_billing')}</span>
            </button>
          </div>
        </div>
      </div>
    );
  }

  // Normal dashboard with KPIs + stores
  return (
    <div className="max-w-6xl mx-auto px-4 py-6 md:px-8 md:py-10 space-y-8 animate-in fade-in duration-500">
      {error && (
        <div className="p-4 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600 flex items-center gap-2 shadow-sm">
          <AlertTriangle className="w-5 h-5 text-red-500 shrink-0" />
          {error}
        </div>
      )}

      {/* Header & Welcome */}
      <div className="flex flex-col md:flex-row md:items-end justify-between gap-4">
        <div>
          <h1 className="text-2xl md:text-3xl font-bold text-slate-900 tracking-tight">
            {t('dash.welcome')}, <span className="text-primary-600">{profile?.profile.email.split('@')[0]}</span>
          </h1>
          <p className="text-slate-500 mt-1 flex items-center gap-2 text-sm md:text-base">
            <span className="inline-block w-2 h-2 rounded-full bg-emerald-500 animate-pulse"></span>
            {t('dash.system_operational')}
          </p>
        </div>
        <div className="flex items-center gap-3 bg-white p-1.5 rounded-xl border border-slate-100 shadow-sm self-start md:self-auto">
          <div className="px-3 py-1.5 bg-slate-50 rounded-lg text-xs font-medium text-slate-600">
            {new Date().toLocaleDateString(undefined, { weekday: 'short', month: 'short', day: 'numeric' })}
          </div>
          <div className="h-4 w-px bg-slate-200"></div>
          <div className="px-2 text-xs font-semibold text-slate-900">
            {t('stats.today_summary')}
          </div>
        </div>
      </div>

      {/* Today's KPI summary */}
      <div className="grid grid-cols-2 lg:grid-cols-4 gap-3">
        <KpiCard icon={DollarSign} bg="bg-primary-100" color="text-primary-600" value={formatCurrency(overview?.revenue ?? 0)} label={t('stats.total_sales')} accent />
        <KpiCard icon={ShoppingBag} bg="bg-green-100" color="text-green-600" value={String(overview?.orders ?? 0)} label={t('stats.completed_orders')} />
        <KpiCard icon={Users} bg="bg-violet-100" color="text-violet-600" value={String(overview?.guests ?? 0)} label={t('stats.guests')} />
        <KpiCard icon={TrendingUp} bg="bg-amber-100" color="text-amber-600" value={formatCurrency(overview?.average_order_value ?? 0)} label={t('stats.average_order')} />
      </div>

      {/* Stores list */}
      <div className="space-y-4">
        <div className="flex items-center justify-between px-1">
          <h2 className="text-lg font-bold text-slate-900 flex items-center gap-2">
            <Server className="w-5 h-5 text-slate-400" />
            {t('nav.stores')}
          </h2>
          {stores.length > 0 && (
            <span className="text-xs font-medium text-slate-500 bg-slate-100 px-2 py-1 rounded-md">
              {stores.length} {t('common.label.active')}
            </span>
          )}
        </div>
        
        <div className="bg-white rounded-2xl border border-slate-200/60 shadow-sm overflow-hidden">
          {stores.length === 0 ? (
            <div className="text-center py-12 px-6">
              <div className="w-16 h-16 bg-slate-50 rounded-full flex items-center justify-center mx-auto mb-4">
                <Server className="w-8 h-8 text-slate-300" />
              </div>
              <h3 className="text-slate-900 font-medium mb-1">{t('dash.no_stores')}</h3>
              <p className="text-sm text-slate-500 max-w-xs mx-auto">{t('dash.no_stores_hint')}</p>
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

      {/* Download app */}
      <div className="relative overflow-hidden bg-gradient-to-br from-slate-900 to-slate-800 rounded-2xl p-6 md:p-8 shadow-xl shadow-slate-900/10">
        <div className="absolute top-0 right-0 -mt-10 -mr-10 w-40 h-40 bg-white/5 rounded-full blur-3xl"></div>
        <div className="absolute bottom-0 left-0 -mb-10 -ml-10 w-40 h-40 bg-primary-500/20 rounded-full blur-3xl"></div>
        
        <div className="relative flex flex-col md:flex-row md:items-center justify-between gap-6">
          <div className="flex items-start gap-5">
            <div className="w-14 h-14 bg-white/10 backdrop-blur-md rounded-2xl flex items-center justify-center shrink-0 border border-white/10 shadow-lg">
              <Download className="w-7 h-7 text-white" />
            </div>
            <div>
              <h3 className="text-lg font-bold text-white mb-2">{t('dash.download_app')}</h3>
              <p className="text-sm text-slate-300 max-w-md leading-relaxed">{t('dash.download_desc')}</p>
            </div>
          </div>
          <a
            href="https://cloud.redcoral.app/api/download/latest"
            className="inline-flex items-center justify-center gap-2 bg-white hover:bg-slate-50 text-slate-900 font-bold text-sm px-6 py-3.5 rounded-xl transition-all hover:scale-105 hover:shadow-lg active:scale-95 w-full md:w-auto min-w-[200px]"
          >
            <Download className="w-4 h-4" />
            {t('dash.download_windows')}
          </a>
        </div>
      </div>
    </div>
  );
};

// --- Sub-components ---

const KpiCard: React.FC<{
  icon: React.FC<{ className?: string }>;
  bg: string;
  color: string;
  value: string;
  label: string;
  accent?: boolean;
}> = ({ icon: Icon, bg, color, value, label, accent }) => (
  <div className={`bg-white rounded-xl border ${accent ? 'border-primary-200 ring-1 ring-primary-100' : 'border-slate-200'} p-4`}>
    <div className={`w-8 h-8 ${bg} rounded-lg flex items-center justify-center mb-2`}>
      <Icon className={`w-4 h-4 ${color}`} />
    </div>
    <p className={`text-lg font-bold ${accent ? 'text-primary-600' : 'text-slate-900'}`}>{value}</p>
    <p className="text-xs text-slate-400">{label}</p>
  </div>
);

const P12Step: React.FC<{
  t: (key: string) => string;
  p12Uploaded: boolean;
  p12Subject: string;
  p12Expires: number | null;
  p12File: File | null;
  p12Password: string;
  p12Uploading: boolean;
  p12Error: string;
  onFileChange: (f: File | null) => void;
  onPasswordChange: (v: string) => void;
  onUpload: () => void;
  onContinue: () => void;
  onReset: () => void;
}> = ({ t, p12Uploaded, p12Subject, p12Expires, p12File, p12Password, p12Uploading, p12Error, onFileChange, onPasswordChange, onUpload, onContinue, onReset }) => (
  <>
    <div className="text-center mb-6">
      <div className="w-14 h-14 bg-primary-100 rounded-2xl flex items-center justify-center mx-auto mb-4">
        <FileKey className="w-7 h-7 text-primary-500" />
      </div>
      <h1 className="text-2xl font-bold text-slate-900 mb-2">{t('onboard.p12_title')}</h1>
      <p className="text-sm text-slate-500 max-w-md mx-auto">{t('onboard.p12_subtitle')}</p>
    </div>

    <div className="max-w-md mx-auto">
      {p12Uploaded ? (
        <>
          <div className="bg-green-50 border border-green-200 rounded-xl p-5 mb-4">
            <div className="flex items-center gap-3 mb-3">
              <ShieldCheck className="w-6 h-6 text-green-600" />
              <span className="font-semibold text-green-800">{t('onboard.p12_uploaded')}</span>
            </div>
            <div className="space-y-1 text-sm text-green-700">
              <p><span className="font-medium">{t('onboard.p12_subject')}:</span> {p12Subject}</p>
              {p12Expires && <p><span className="font-medium">{t('onboard.p12_expires')}:</span> {formatDate(p12Expires * 1000)}</p>}
            </div>
          </div>
          <div className="flex gap-3">
            <button onClick={onReset} className="flex-1 py-3 bg-slate-100 hover:bg-slate-200 text-slate-700 font-semibold rounded-lg transition-colors cursor-pointer">
              {t('onboard.p12_change')}
            </button>
            <button onClick={onContinue} className="flex-1 py-3 bg-primary-500 hover:bg-primary-600 text-white font-semibold rounded-lg transition-colors cursor-pointer flex items-center justify-center gap-2">
              {t('onboard.p12_continue')}
              <ArrowRight className="w-4 h-4" />
            </button>
          </div>
        </>
      ) : (
        <div className="bg-white rounded-2xl border border-slate-200 p-6 space-y-4">
          {p12Error && <div className="p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{p12Error}</div>}
          <div>
            <label htmlFor="p12-file" className="block text-sm font-medium text-slate-700 mb-1.5">{t('onboard.p12_label')}</label>
            <input
              id="p12-file"
              type="file"
              accept=".p12,.pfx"
              onChange={e => onFileChange(e.target.files?.[0] ?? null)}
              className="w-full text-sm text-slate-600 file:mr-4 file:py-2 file:px-4 file:rounded-lg file:border-0 file:text-sm file:font-medium file:bg-primary-50 file:text-primary-600 hover:file:bg-primary-100 file:cursor-pointer"
            />
          </div>
          <div>
            <label htmlFor="p12-password" className="block text-sm font-medium text-slate-700 mb-1.5">{t('onboard.p12_password')}</label>
            <input
              id="p12-password"
              type="password"
              value={p12Password}
              onChange={e => onPasswordChange(e.target.value)}
              placeholder={t('onboard.p12_password_placeholder')}
              className="w-full px-3 py-2.5 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
            />
          </div>
          <button
            onClick={onUpload}
            disabled={!p12File || p12Uploading}
            className="w-full py-3 bg-primary-500 hover:bg-primary-600 text-white font-semibold rounded-lg transition-colors cursor-pointer disabled:opacity-60 flex items-center justify-center gap-2"
          >
            {p12Uploading ? <><Spinner />{t('onboard.p12_uploading')}</> : <><Upload className="w-4 h-4" />{t('onboard.p12_upload')}</>}
          </button>
          <p className="text-xs text-slate-400 text-center">{t('onboard.p12_skip_info')}</p>
        </div>
      )}
    </div>
  </>
);

const PlanStep: React.FC<{
  t: (key: string) => string;
  isAnnual: boolean;
  onToggleAnnual: () => void;
  checkoutLoading: string;
  error: string;
  onChoosePlan: (plan: string) => void;
}> = ({ t, isAnnual, onToggleAnnual, checkoutLoading, error, onChoosePlan }) => (
  <>
    <div className="text-center mb-2">
      <div className="w-14 h-14 bg-primary-100 rounded-2xl flex items-center justify-center mx-auto mb-4">
        <Sparkles className="w-7 h-7 text-primary-500" />
      </div>
      <h1 className="text-2xl font-bold text-slate-900 mb-2">{t('onboard.title')}</h1>
      <p className="text-sm text-slate-500 max-w-md mx-auto">{t('onboard.subtitle')}</p>
    </div>

    {error && <div className="p-4 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600 max-w-3xl mx-auto mb-4">{error}</div>}

    <div className="flex items-center justify-center gap-3 mb-8">
      <span className={`text-sm font-medium transition-colors ${isAnnual ? 'text-slate-400' : 'text-slate-900'}`}>{t('onboard.monthly')}</span>
      <button
        onClick={onToggleAnnual}
        className={`relative w-12 h-6 rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2 cursor-pointer ${isAnnual ? 'bg-primary-500' : 'bg-slate-300'}`}
      >
        <span className={`absolute top-0.5 left-0.5 w-5 h-5 bg-white rounded-full shadow-sm transition-transform ${isAnnual ? 'translate-x-6' : 'translate-x-0'}`} />
      </button>
      <span className={`text-sm font-medium transition-colors ${isAnnual ? 'text-slate-900' : 'text-slate-400'}`}>
        {t('onboard.yearly')}
        <span className="text-primary-500 text-xs font-bold ml-1">{t('onboard.save_20')}</span>
      </span>
    </div>

    <div className="grid grid-cols-1 md:grid-cols-2 gap-6 max-w-3xl mx-auto">
      {/* Basic */}
      <div className="bg-white rounded-2xl border border-slate-200 p-6 flex flex-col">
        <h3 className="font-bold text-lg text-slate-900">Basic</h3>
        <p className="text-sm text-slate-500 mt-1">{t('onboard.basic_desc')}</p>
        <div className="mt-4 mb-6">
          <span className="text-3xl font-bold text-slate-900">&euro;{isAnnual ? '31' : '39'}</span>
          <span className="text-sm text-slate-500">/{t('onboard.month')}</span>
          {isAnnual && <p className="text-xs text-slate-400 mt-1">{t('onboard.billed_yearly')}</p>}
        </div>
        <ul className="space-y-2 text-sm text-slate-600 mb-6 flex-1">
          <li className="flex items-center gap-2"><CheckCircle className="w-4 h-4 text-green-500 shrink-0" /> 1 {t('onboard.location')}</li>
          <li className="flex items-center gap-2"><CheckCircle className="w-4 h-4 text-green-500 shrink-0" /> 5 {t('onboard.devices')}</li>
          <li className="flex items-center gap-2"><CheckCircle className="w-4 h-4 text-green-500 shrink-0" /> {t('onboard.cloud_sync')}</li>
        </ul>
        <button
          onClick={() => onChoosePlan('basic')}
          disabled={checkoutLoading !== ''}
          className="w-full py-3 bg-slate-800 hover:bg-slate-900 text-white font-semibold rounded-lg transition-colors cursor-pointer disabled:opacity-60 flex items-center justify-center gap-2"
        >
          {checkoutLoading === 'basic' && <Spinner />}
          {t('onboard.choose')}
        </button>
      </div>

      {/* Pro */}
      <div className="bg-white rounded-2xl border-2 border-primary-500 p-6 flex flex-col relative">
        <span className="absolute -top-3 left-6 bg-primary-500 text-white text-xs font-bold px-3 py-0.5 rounded-full">{t('onboard.popular')}</span>
        <h3 className="font-bold text-lg text-slate-900">Pro</h3>
        <p className="text-sm text-slate-500 mt-1">{t('onboard.pro_desc')}</p>
        <div className="mt-4 mb-6">
          <span className="text-3xl font-bold text-slate-900">&euro;{isAnnual ? '55' : '69'}</span>
          <span className="text-sm text-slate-500">/{t('onboard.month')}</span>
          {isAnnual && <p className="text-xs text-slate-400 mt-1">{t('onboard.billed_yearly')}</p>}
        </div>
        <ul className="space-y-2 text-sm text-slate-600 mb-6 flex-1">
          <li className="flex items-center gap-2"><CheckCircle className="w-4 h-4 text-green-500 shrink-0" /> 3 {t('onboard.location')}</li>
          <li className="flex items-center gap-2"><CheckCircle className="w-4 h-4 text-green-500 shrink-0" /> 10 {t('onboard.devices')}</li>
          <li className="flex items-center gap-2"><CheckCircle className="w-4 h-4 text-green-500 shrink-0" /> {t('onboard.cloud_sync')}</li>
          <li className="flex items-center gap-2"><CheckCircle className="w-4 h-4 text-green-500 shrink-0" /> {t('onboard.priority_support')}</li>
        </ul>
        <button
          onClick={() => onChoosePlan('pro')}
          disabled={checkoutLoading !== ''}
          className="w-full py-3 bg-primary-500 hover:bg-primary-600 text-white font-semibold rounded-lg transition-colors cursor-pointer disabled:opacity-60 flex items-center justify-center gap-2"
        >
          {checkoutLoading === 'pro' && <Spinner />}
          {t('onboard.choose')}
        </button>
      </div>
    </div>
  </>
);
