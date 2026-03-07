import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { User, Lock, CreditCard, Globe, Save, ExternalLink, AlertTriangle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getProfile, updateProfile, createBillingPortal, cancelSubscription, resumeSubscription, changePlan } from '@/infrastructure/api/profile';
import { changePassword } from '@/infrastructure/api/auth';
import { ApiError } from '@/infrastructure/api/client';
import { apiErrorMessage } from '@/infrastructure/i18n';
import { Spinner } from '@/presentation/components/ui/Spinner';
import { SUPPORTED_LOCALES, LANG_LABELS, setLocale, getLocale } from '@/infrastructure/i18n';
import type { Locale } from '@/infrastructure/i18n';
import type { TenantProfile, Subscription, P12Info } from '@/core/types/auth';

export const SettingsScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [profile, setProfile] = useState<TenantProfile | null>(null);
  const [subscription, setSubscription] = useState<Subscription | null>(null);
  const [p12, setP12] = useState<P12Info | null>(null);
  const [loading, setLoading] = useState(true);

  // Profile edit
  const [name, setName] = useState('');
  const [saving, setSaving] = useState(false);
  const [saveMsg, setSaveMsg] = useState('');

  // Password
  const [curPwd, setCurPwd] = useState('');
  const [newPwd, setNewPwd] = useState('');
  const [pwdMsg, setPwdMsg] = useState('');
  const [pwdSaving, setPwdSaving] = useState(false);

  // Language
  const [lang, setLang] = useState<Locale>(getLocale());

  // Billing actions
  const [billingMsg, setBillingMsg] = useState('');
  const [billingErr, setBillingErr] = useState('');
  const [cancelLoading, setCancelLoading] = useState(false);
  const [resumeLoading, setResumeLoading] = useState(false);
  const [changePlanLoading, setChangePlanLoading] = useState(false);
  const [showCancelConfirm, setShowCancelConfirm] = useState(false);

  useEffect(() => {
    const tk = useAuthStore.getState().token;
    if (!tk) return;
    (async () => {
      try {
        const res = await getProfile(tk);
        setProfile(res.profile);
        setSubscription(res.subscription);
        setP12(res.p12);
        setName(res.profile.name ?? '');
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); }
      } finally {
        setLoading(false);
      }
    })();
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const handleSaveProfile = async () => {
    if (!token) return;
    setSaving(true); setSaveMsg('');
    try {
      await updateProfile(token, name);
      setSaveMsg(t('settings.saved'));
    } catch (err) {
      setSaveMsg(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message, err.status) : t('auth.error_generic'));
    } finally { setSaving(false); }
  };

  const handleChangePwd = async () => {
    if (!token) return;
    setPwdSaving(true); setPwdMsg('');
    try {
      await changePassword(token, curPwd, newPwd);
      setPwdMsg(t('settings.password_changed'));
      setCurPwd(''); setNewPwd('');
    } catch (err) {
      setPwdMsg(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message, err.status) : t('auth.error_generic'));
    } finally { setPwdSaving(false); }
  };

  const handleBilling = async () => {
    if (!token) return;
    try {
      const res = await createBillingPortal(token);
      try {
        const parsed = new URL(res.url);
        if (parsed.protocol !== 'https:' || (parsed.hostname !== 'billing.stripe.com' && parsed.hostname !== 'checkout.stripe.com')) return;
      } catch { return; }
      window.open(res.url, '_blank');
    } catch { /* ignore */ }
  };

  const handleCancel = async () => {
    if (!token) return;
    setCancelLoading(true); setBillingMsg(''); setBillingErr('');
    try {
      await cancelSubscription(token);
      setSubscription(s => s ? { ...s, cancel_at_period_end: true } : s);
      setBillingMsg(t('settings.subscription_canceled'));
      setShowCancelConfirm(false);
    } catch (err) {
      setBillingErr(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message, err.status) : t('auth.error_generic'));
    } finally { setCancelLoading(false); }
  };

  const handleResume = async () => {
    if (!token) return;
    setResumeLoading(true); setBillingMsg(''); setBillingErr('');
    try {
      await resumeSubscription(token);
      setSubscription(s => s ? { ...s, cancel_at_period_end: false } : s);
      setBillingMsg(t('settings.subscription_resumed'));
    } catch (err) {
      setBillingErr(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message, err.status) : t('auth.error_generic'));
    } finally { setResumeLoading(false); }
  };

  const handleChangePlan = async (plan: string) => {
    if (!token) return;
    setChangePlanLoading(true); setBillingMsg(''); setBillingErr('');
    try {
      await changePlan(token, plan);
      // Refresh profile to get updated subscription
      const res = await getProfile(token);
      setSubscription(res.subscription);
      setBillingMsg(t('settings.plan_changed'));
    } catch (err) {
      setBillingErr(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message, err.status) : t('auth.error_generic'));
    } finally { setChangePlanLoading(false); }
  };

  const handleLangChange = (newLang: Locale) => {
    setLang(newLang);
    setLocale(newLang);
  };

  if (loading) return <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>;

  const isActive = subscription && (subscription.status === 'active' || subscription.status === 'trialing');
  const currentPlanBase = subscription?.plan ?? 'basic';
  const currentInterval = subscription?.billing_interval ?? 'month';

  return (
    <div className="max-w-2xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-6">
      <h1 className="text-xl font-bold text-slate-900">{t('settings.title')}</h1>

      {/* Profile */}
      <Section icon={User} title={t('settings.profile')}>
        <div className="space-y-3">
          <Field label={t('settings.email')} value={profile?.email ?? ''} disabled />
          <div>
            <label className="block text-sm font-medium text-slate-700 mb-1">{t('settings.name')}</label>
            <input value={name} onChange={e => setName(e.target.value)} className="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30" />
          </div>
          <div className="flex items-center gap-3">
            <button onClick={handleSaveProfile} disabled={saving} className="px-4 py-2 bg-primary-500 text-white text-sm font-medium rounded-lg hover:bg-primary-600 transition-colors disabled:opacity-50 flex items-center gap-1.5">
              <Save className="w-4 h-4" />{saving ? '...' : t('settings.save')}
            </button>
            {saveMsg && <span className="text-sm text-slate-500">{saveMsg}</span>}
          </div>
        </div>
      </Section>

      {/* Password */}
      <Section icon={Lock} title={t('settings.change_password')}>
        <div className="space-y-3">
          <div>
            <label className="block text-sm font-medium text-slate-700 mb-1">{t('settings.current_password')}</label>
            <input type="password" value={curPwd} onChange={e => setCurPwd(e.target.value)} className="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30" />
          </div>
          <div>
            <label className="block text-sm font-medium text-slate-700 mb-1">{t('settings.new_password')}</label>
            <input type="password" value={newPwd} onChange={e => setNewPwd(e.target.value)} className="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30" />
          </div>
          <div className="flex items-center gap-3">
            <button onClick={handleChangePwd} disabled={pwdSaving || !curPwd || !newPwd} className="px-4 py-2 bg-slate-800 text-white text-sm font-medium rounded-lg hover:bg-slate-700 transition-colors disabled:opacity-50">
              {pwdSaving ? '...' : t('settings.change_password')}
            </button>
            {pwdMsg && <span className="text-sm text-slate-500">{pwdMsg}</span>}
          </div>
        </div>
      </Section>

      {/* Language */}
      <Section icon={Globe} title={t('settings.language')}>
        <div className="flex gap-2">
          {SUPPORTED_LOCALES.map(l => (
            <button key={l} onClick={() => handleLangChange(l)} className={`px-4 py-2 text-sm font-medium rounded-lg border transition-colors ${lang === l ? 'bg-primary-500 text-white border-primary-500' : 'bg-white text-slate-700 border-slate-200 hover:border-primary-300'}`}>
              {LANG_LABELS[l]}
            </button>
          ))}
        </div>
      </Section>

      {/* Billing */}
      <Section icon={CreditCard} title={t('settings.billing')}>
        {subscription ? (
          <div className="space-y-4">
            {/* Status row */}
            <div className="flex items-center gap-4 text-sm">
              <span className="text-slate-500">{t('dash.plan')}:</span>
              <span className="font-semibold text-slate-900 capitalize">{subscription.plan}</span>
              <span className={`px-2 py-0.5 rounded text-xs font-medium ${subscription.status === 'active' || subscription.status === 'trialing' ? 'bg-green-100 text-green-700' : subscription.status === 'canceled' ? 'bg-red-100 text-red-700' : 'bg-amber-100 text-amber-700'}`}>
                {subscription.status}
              </span>
              {subscription.billing_interval && (
                <span className="text-xs text-slate-400">
                  ({subscription.billing_interval === 'year' ? t('settings.billing_yearly') : t('settings.billing_monthly')})
                </span>
              )}
            </div>

            {/* Period end */}
            {subscription.current_period_end && (
              <p className="text-sm text-slate-500">
                {t('dash.next_billing')}: {new Date(subscription.current_period_end).toLocaleDateString()}
              </p>
            )}

            {/* Cancel warning */}
            {subscription.cancel_at_period_end && (
              <div className="flex items-center gap-2 px-3 py-2 bg-amber-50 border border-amber-200 rounded-lg text-sm text-amber-700">
                <AlertTriangle className="w-4 h-4 shrink-0" />
                {t('dash.cancel_warning')}
              </div>
            )}

            {/* Messages */}
            {billingMsg && <p className="text-sm text-green-600">{billingMsg}</p>}
            {billingErr && <p className="text-sm text-red-600">{billingErr}</p>}

            {/* Cancel confirm dialog */}
            {showCancelConfirm && (
              <div className="px-4 py-3 bg-red-50 border border-red-200 rounded-lg space-y-3">
                <p className="text-sm text-red-700">{t('settings.cancel_confirm')}</p>
                <div className="flex gap-2">
                  <button
                    onClick={handleCancel}
                    disabled={cancelLoading}
                    className="px-3 py-1.5 bg-red-600 text-white text-sm font-medium rounded-lg hover:bg-red-700 transition-colors disabled:opacity-50"
                  >
                    {cancelLoading ? t('settings.canceling') : t('settings.cancel_subscription')}
                  </button>
                  <button
                    onClick={() => setShowCancelConfirm(false)}
                    className="px-3 py-1.5 bg-white text-slate-700 text-sm font-medium rounded-lg border border-slate-200 hover:bg-slate-50 transition-colors"
                  >
                    {t('auth.back_to_login').replace(/.*/, '取消')}
                  </button>
                </div>
              </div>
            )}

            {/* Action buttons */}
            <div className="flex flex-wrap gap-2">
              <button onClick={handleBilling} className="px-4 py-2 bg-slate-800 text-white text-sm font-medium rounded-lg hover:bg-slate-700 transition-colors flex items-center gap-1.5">
                <ExternalLink className="w-4 h-4" />{t('settings.manage_billing')}
              </button>

              {isActive && !subscription.cancel_at_period_end && !showCancelConfirm && (
                <button
                  onClick={() => setShowCancelConfirm(true)}
                  className="px-4 py-2 text-red-600 text-sm font-medium rounded-lg border border-red-200 hover:bg-red-50 transition-colors"
                >
                  {t('settings.cancel_subscription')}
                </button>
              )}

              {isActive && subscription.cancel_at_period_end && (
                <button
                  onClick={handleResume}
                  disabled={resumeLoading}
                  className="px-4 py-2 bg-green-600 text-white text-sm font-medium rounded-lg hover:bg-green-700 transition-colors disabled:opacity-50"
                >
                  {resumeLoading ? t('settings.resuming') : t('settings.resume_subscription')}
                </button>
              )}
            </div>

            {/* Change plan */}
            {isActive && (
              <div className="pt-3 border-t border-slate-100">
                <p className="text-sm font-medium text-slate-700 mb-2">{t('settings.change_plan')}</p>
                <div className="flex flex-wrap gap-2">
                  {(['basic', 'pro'] as const).map(plan => {
                    const intervals = ['month', 'year'] as const;
                    return intervals.map(interval => {
                      const planKey = interval === 'year' ? `${plan}_yearly` : plan;
                      const isCurrent = currentPlanBase === plan && currentInterval === interval;
                      return (
                        <button
                          key={planKey}
                          onClick={() => !isCurrent && handleChangePlan(planKey)}
                          disabled={isCurrent || changePlanLoading}
                          className={`px-3 py-1.5 text-sm font-medium rounded-lg border transition-colors ${
                            isCurrent
                              ? 'bg-primary-50 text-primary-700 border-primary-300 cursor-default'
                              : 'bg-white text-slate-700 border-slate-200 hover:border-primary-300 disabled:opacity-50'
                          }`}
                        >
                          <span className="capitalize">{plan}</span>
                          {' '}
                          <span className="text-xs text-slate-400">
                            ({interval === 'year' ? t('settings.billing_yearly') : t('settings.billing_monthly')})
                          </span>
                          {isCurrent && <span className="ml-1 text-xs">✓</span>}
                        </button>
                      );
                    });
                  })}
                </div>
              </div>
            )}
          </div>
        ) : (
          <p className="text-sm text-slate-500">{t('dash.no_stores_hint')}</p>
        )}
      </Section>

      {/* P12 */}
      {p12?.has_p12 && (() => {
        const daysUntilExpiry = p12.expires_at ? Math.floor((p12.expires_at - Date.now()) / (1000 * 60 * 60 * 24)) : null;
        const isExpired = daysUntilExpiry !== null && daysUntilExpiry < 0;
        const isExpiringSoon = daysUntilExpiry !== null && daysUntilExpiry >= 0 && daysUntilExpiry <= 60;
        return (
          <Section icon={Lock} title={t('onboard.p12_uploaded')}>
            {(isExpired || isExpiringSoon) && (
              <div className={`mb-3 px-3 py-2 rounded-lg text-sm font-medium ${isExpired ? 'bg-red-50 text-red-700 border border-red-200' : 'bg-amber-50 text-amber-700 border border-amber-200'}`}>
                {isExpired
                  ? t('onboard.p12_expired')
                  : `${t('onboard.p12_expiring_soon')} (${daysUntilExpiry}d)`
                }
              </div>
            )}
            <div className="text-sm space-y-2">
              {p12.subject && <P12Row label={t('onboard.p12_subject')} value={p12.subject} bold />}
              {p12.serial_number && <P12Row label={t('onboard.p12_nif')} value={p12.serial_number} />}
              {p12.organization && <P12Row label={t('onboard.p12_organization')} value={p12.organization} />}
              {p12.issuer && <P12Row label={t('onboard.p12_issuer')} value={p12.issuer} />}
              {(p12.not_before || p12.expires_at) && (
                <P12Row
                  label={t('onboard.p12_validity')}
                  value={[
                    p12.not_before ? new Date(p12.not_before).toLocaleDateString() : '?',
                    p12.expires_at ? new Date(p12.expires_at).toLocaleDateString() : '?',
                  ].join(' — ')}
                />
              )}
              {p12.fingerprint && <P12Row label={t('onboard.p12_fingerprint')} value={p12.fingerprint.length > 20 ? `${p12.fingerprint.slice(0, 8)}...${p12.fingerprint.slice(-8)}` : p12.fingerprint} mono />}
            </div>
          </Section>
        );
      })()}
    </div>
  );
};

const Section: React.FC<{ icon: React.FC<{ className?: string }>; title: string; children: React.ReactNode }> = ({ icon: Icon, title, children }) => (
  <div className="bg-white rounded-2xl border border-slate-200 p-6">
    <div className="flex items-center gap-2 mb-4">
      <Icon className="w-5 h-5 text-slate-400" />
      <h2 className="font-bold text-slate-900">{title}</h2>
    </div>
    {children}
  </div>
);

const P12Row: React.FC<{ label: string; value: string; bold?: boolean; mono?: boolean }> = ({ label, value, bold, mono }) => (
  <p>
    <span className="text-slate-500">{label}:</span>{' '}
    <span className={`text-slate-900 ${bold ? 'font-medium' : ''} ${mono ? 'font-mono text-xs break-all' : ''}`}>{value}</span>
  </p>
);

const Field: React.FC<{ label: string; value: string; disabled?: boolean }> = ({ label, value, disabled }) => (
  <div>
    <label className="block text-sm font-medium text-slate-700 mb-1">{label}</label>
    <input value={value} disabled={disabled} className="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm bg-slate-50 text-slate-500" readOnly />
  </div>
);
