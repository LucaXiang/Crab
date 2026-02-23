import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { User, Lock, Mail, CreditCard, Globe, Save, ExternalLink } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getProfile, updateProfile, createBillingPortal } from '@/infrastructure/api/profile';
import { changePassword, changeEmail, confirmEmailChange } from '@/infrastructure/api/auth';
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

  // Email
  const [emailPwd, setEmailPwd] = useState('');
  const [newEmail, setNewEmail] = useState('');
  const [emailCode, setEmailCode] = useState('');
  const [emailStep, setEmailStep] = useState<'form' | 'code'>('form');
  const [emailMsg, setEmailMsg] = useState('');
  const [emailSaving, setEmailSaving] = useState(false);

  // Language
  const [lang, setLang] = useState<Locale>(getLocale());

  useEffect(() => {
    if (!token) return;
    (async () => {
      try {
        const res = await getProfile(token);
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
  }, [token, clearAuth, navigate]);

  const handleSaveProfile = async () => {
    if (!token) return;
    setSaving(true); setSaveMsg('');
    try {
      await updateProfile(token, name);
      setSaveMsg(t('settings.saved'));
    } catch (err) {
      setSaveMsg(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message) : t('auth.error_generic'));
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
      setPwdMsg(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message) : t('auth.error_generic'));
    } finally { setPwdSaving(false); }
  };

  const handleChangeEmail = async () => {
    if (!token) return;
    setEmailSaving(true); setEmailMsg('');
    try {
      if (emailStep === 'form') {
        await changeEmail(token, emailPwd, newEmail);
        setEmailMsg(t('settings.email_code_sent'));
        setEmailStep('code');
      } else {
        await confirmEmailChange(token, newEmail, emailCode);
        setEmailMsg(t('settings.email_changed'));
        setEmailStep('form'); setNewEmail(''); setEmailPwd(''); setEmailCode('');
        // Refresh profile
        const res = await getProfile(token);
        setProfile(res.profile);
      }
    } catch (err) {
      setEmailMsg(err instanceof ApiError ? apiErrorMessage(t, err.code, err.message) : t('auth.error_generic'));
    } finally { setEmailSaving(false); }
  };

  const handleBilling = async () => {
    if (!token) return;
    try {
      const res = await createBillingPortal(token);
      window.open(res.url, '_blank');
    } catch { /* ignore */ }
  };

  const handleLangChange = (newLang: Locale) => {
    setLang(newLang);
    setLocale(newLang);
  };

  if (loading) return <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>;

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

      {/* Email */}
      <Section icon={Mail} title={t('settings.change_email')}>
        <div className="space-y-3">
          {emailStep === 'form' ? (
            <>
              <div>
                <label className="block text-sm font-medium text-slate-700 mb-1">{t('auth.password')}</label>
                <input type="password" value={emailPwd} onChange={e => setEmailPwd(e.target.value)} className="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30" />
              </div>
              <div>
                <label className="block text-sm font-medium text-slate-700 mb-1">{t('settings.new_email')}</label>
                <input type="email" value={newEmail} onChange={e => setNewEmail(e.target.value)} className="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30" />
              </div>
            </>
          ) : (
            <div>
              <label className="block text-sm font-medium text-slate-700 mb-1">{t('settings.confirm_code')}</label>
              <input value={emailCode} onChange={e => setEmailCode(e.target.value)} className="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/30" />
            </div>
          )}
          <div className="flex items-center gap-3">
            <button onClick={handleChangeEmail} disabled={emailSaving} className="px-4 py-2 bg-slate-800 text-white text-sm font-medium rounded-lg hover:bg-slate-700 transition-colors disabled:opacity-50">
              {emailSaving ? '...' : emailStep === 'form' ? t('settings.change_email') : t('common.action.confirm')}
            </button>
            {emailMsg && <span className="text-sm text-slate-500">{emailMsg}</span>}
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
          <div className="space-y-3">
            <div className="flex items-center gap-4 text-sm">
              <span className="text-slate-500">{t('dash.plan')}:</span>
              <span className="font-semibold text-slate-900 capitalize">{subscription.plan}</span>
              <span className={`px-2 py-0.5 rounded text-xs font-medium ${subscription.status === 'active' ? 'bg-green-100 text-green-700' : 'bg-amber-100 text-amber-700'}`}>
                {subscription.status}
              </span>
            </div>
            {subscription.current_period_end && (
              <p className="text-sm text-slate-500">{t('dash.next_billing')}: {new Date(subscription.current_period_end * 1000).toLocaleDateString()}</p>
            )}
            <button onClick={handleBilling} className="px-4 py-2 bg-slate-800 text-white text-sm font-medium rounded-lg hover:bg-slate-700 transition-colors flex items-center gap-1.5">
              <ExternalLink className="w-4 h-4" />{t('settings.manage_billing')}
            </button>
          </div>
        ) : (
          <p className="text-sm text-slate-500">{t('dash.no_stores_hint')}</p>
        )}
      </Section>

      {/* P12 */}
      {p12?.has_p12 && (
        <Section icon={Lock} title={t('onboard.p12_uploaded')}>
          <div className="text-sm space-y-1">
            {p12.subject && <p><span className="text-slate-500">{t('onboard.p12_subject')}:</span> <span className="text-slate-900 font-medium">{p12.subject}</span></p>}
            {p12.expires_at && <p><span className="text-slate-500">{t('onboard.p12_expires')}:</span> <span className="text-slate-900">{new Date(p12.expires_at * 1000).toLocaleDateString()}</span></p>}
          </div>
        </Section>
      )}
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

const Field: React.FC<{ label: string; value: string; disabled?: boolean }> = ({ label, value, disabled }) => (
  <div>
    <label className="block text-sm font-medium text-slate-700 mb-1">{label}</label>
    <input value={value} disabled={disabled} className="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm bg-slate-50 text-slate-500" readOnly />
  </div>
);
