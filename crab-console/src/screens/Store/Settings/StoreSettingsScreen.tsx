import React, { useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Save, Copy, Check, MapPin, Phone, Mail, Globe, Clock, FileText, Fingerprint, CalendarDays } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getStores, updateStore } from '@/infrastructure/api/stores';
import { getStoreInfo, updateStoreInfo } from '@/infrastructure/api/store';
import { ApiError } from '@/infrastructure/api/client';
import { apiErrorMessage } from '@/infrastructure/i18n';
import { Spinner } from '@/presentation/components/ui/Spinner';
import { ImageUpload } from '@/shared/components/ImageUpload';
import type { StoreDetail } from '@/core/types/store';

export const StoreSettingsScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const clearAuth = useAuthStore(s => s.clearAuth);

  const [store, setStore] = useState<StoreDetail | null>(null);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [msg, setMsg] = useState<{ text: string; ok: boolean } | null>(null);

  const [formLogo, setFormLogo] = useState('');
  const [form, setForm] = useState({
    name: '',
    address: '',
    phone: '',
    nif: '',
    email: '',
    website: '',
    business_day_cutoff: '',
  });

  useEffect(() => {
    if (!token) return;
    (async () => {
      try {
        const [stores, info] = await Promise.all([
          getStores(token),
          getStoreInfo(token, storeId),
        ]);
        const s = stores.find(s => s.id === storeId);
        if (s) {
          setStore(s);
          setForm({
            name: s.name ?? '',
            address: s.address ?? '',
            phone: s.phone ?? '',
            nif: s.nif ?? '',
            email: s.email ?? '',
            website: s.website ?? '',
            business_day_cutoff: s.business_day_cutoff ?? '',
          });
        }
        setFormLogo(info.logo ?? '');
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); }
      } finally {
        setLoading(false);
      }
    })();
  }, [token, storeId, clearAuth, navigate]);

  const handleSave = async () => {
    if (!token) return;
    setSaving(true); setMsg(null);
    try {
      await Promise.all([
        updateStore(token, storeId, form),
        updateStoreInfo(token, storeId, { logo: formLogo || undefined }),
      ]);
      setMsg({ text: t('store.saved'), ok: true });
    } catch (err) {
      setMsg({ text: err instanceof ApiError ? apiErrorMessage(t, err.code, err.message) : t('auth.error_generic'), ok: false });
    } finally { setSaving(false); }
  };

  const update = (key: keyof typeof form, value: string) => setForm(prev => ({ ...prev, [key]: value }));

  if (loading) return <div className="flex items-center justify-center py-20"><Spinner className="w-8 h-8 text-primary-500" /></div>;

  return (
    <div className="max-w-2xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-5">

      {/* Device info strip */}
      {store && (
        <div className="bg-slate-50 rounded-xl border border-slate-200/80 px-5 py-4">
          <div className="flex flex-col sm:flex-row sm:items-center gap-3">
            <div className="flex items-center gap-3 min-w-0 flex-1">
              <div className="w-9 h-9 rounded-lg bg-primary-500/10 flex items-center justify-center shrink-0">
                <Fingerprint className="w-4.5 h-4.5 text-primary-500" />
              </div>
              <div className="min-w-0">
                <p className="text-[11px] font-medium text-slate-400 uppercase tracking-wider">{t('store.device_id')}</p>
                <CopyableId value={store.device_id} />
              </div>
            </div>
            <div className="flex items-center gap-2 text-xs text-slate-400 sm:border-l sm:border-slate-200 sm:pl-4">
              <CalendarDays className="w-3.5 h-3.5" />
              <span>{t('store.registered')}: {new Date(store.registered_at).toLocaleDateString()}</span>
            </div>
          </div>
        </div>
      )}

      {/* Logo */}
      <div className="bg-white rounded-2xl border border-slate-200 p-5 space-y-3">
        <label className="block text-xs font-medium text-slate-500">{t('store.logo')}</label>
        <ImageUpload value={formLogo} onChange={setFormLogo} />
      </div>

      {/* Name + Address */}
      <div className="bg-white rounded-2xl border border-slate-200 p-5 space-y-4">
        <Field label={t('store.name')} value={form.name} onChange={v => update('name', v)} />
        <Field label={t('store.address')} value={form.address} onChange={v => update('address', v)} icon={MapPin} />
      </div>

      {/* Contact + Web */}
      <div className="bg-white rounded-2xl border border-slate-200 p-5 space-y-4">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <Field label={t('store.phone')} value={form.phone} onChange={v => update('phone', v)} icon={Phone} />
          <Field label={t('store.email')} value={form.email} onChange={v => update('email', v)} icon={Mail} type="email" />
        </div>
        <Field label={t('store.website')} value={form.website} onChange={v => update('website', v)} icon={Globe} />
      </div>

      {/* Fiscal + Operations */}
      <div className="bg-white rounded-2xl border border-slate-200 p-5 space-y-4">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-4">
          <Field label={t('store.nif')} value={form.nif} onChange={v => update('nif', v)} icon={FileText} />
          <Field label={t('store.business_day_cutoff')} value={form.business_day_cutoff} onChange={v => update('business_day_cutoff', v)} icon={Clock} placeholder="04:00" />
        </div>
      </div>

      {/* Save bar */}
      <div className="flex items-center gap-3 pt-1">
        <button
          onClick={handleSave}
          disabled={saving}
          className="px-5 py-2.5 bg-primary-500 text-white text-sm font-semibold rounded-xl hover:bg-primary-600 active:scale-[0.98] transition-all disabled:opacity-50 flex items-center gap-2 shadow-sm shadow-primary-500/20"
        >
          {saving ? <Spinner className="w-4 h-4" /> : <Save className="w-4 h-4" />}
          {saving ? t('store.saving') : t('store.save')}
        </button>
        {msg && (
          <span className={`text-sm font-medium ${msg.ok ? 'text-emerald-600' : 'text-red-500'}`}>{msg.text}</span>
        )}
      </div>
    </div>
  );
};

/* ── Copyable device ID ── */
const CopyableId: React.FC<{ value: string }> = ({ value }) => {
  const [copied, setCopied] = useState(false);
  const short = value.length > 16 ? `${value.slice(0, 8)}···${value.slice(-8)}` : value;

  const copy = async () => {
    await navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  return (
    <button onClick={copy} className="flex items-center gap-1.5 group" title={value}>
      <span className="text-sm font-mono text-slate-600 group-hover:text-primary-600 transition-colors">{short}</span>
      {copied
        ? <Check className="w-3.5 h-3.5 text-emerald-500" />
        : <Copy className="w-3.5 h-3.5 text-slate-300 group-hover:text-primary-400 transition-colors" />
      }
    </button>
  );
};

/* ── Form field with optional icon ── */
const Field: React.FC<{
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  placeholder?: string;
  icon?: React.FC<{ className?: string }>;
}> = ({ label, value, onChange, type = 'text', placeholder, icon: Icon }) => (
  <div>
    <label className="block text-xs font-medium text-slate-500 mb-1.5">{label}</label>
    <div className="relative">
      {Icon && (
        <div className="absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none">
          <Icon className="w-4 h-4 text-slate-300" />
        </div>
      )}
      <input
        type={type}
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder={placeholder}
        className={`w-full py-2.5 border border-slate-200 rounded-xl text-sm text-slate-800 placeholder:text-slate-300 focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400 transition-all ${Icon ? 'pl-9 pr-3' : 'px-3'}`}
      />
    </div>
  </div>
);
