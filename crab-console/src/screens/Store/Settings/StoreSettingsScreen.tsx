import React, { useCallback, useEffect, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { Save, Copy, Check, MapPin, Phone, Mail, Globe, Clock, FileText, Fingerprint, CalendarDays, Monitor, Trash2, AlertTriangle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getStores, updateStore, deleteStore, getStoreDevices } from '@/infrastructure/api/stores';
import { getStoreInfo, updateStoreInfo } from '@/infrastructure/api/store';
import { ApiError } from '@/infrastructure/api/client';
import { apiErrorMessage } from '@/infrastructure/i18n';
import { Spinner } from '@/presentation/components/ui/Spinner';
import { ImageUpload } from '@/shared/components/ImageUpload';
import { useLiveOrders } from '@/core/stores/useLiveOrdersStore';
import type { StoreDetail, DeviceRecord } from '@/core/types/store';
import type { StoreInfoSnapshot } from '@/core/types/live';

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
    alias: '',
    name: '',
    address: '',
    phone: '',
    nif: '',
    email: '',
    website: '',
    business_day_cutoff: 0,
  });

  const [devices, setDevices] = useState<DeviceRecord[]>([]);
  const [devicesLoading, setDevicesLoading] = useState(false);
  const [showDeleteConfirm, setShowDeleteConfirm] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Real-time store_info updates via WebSocket
  const handleStoreInfoUpdated = useCallback((info: StoreInfoSnapshot) => {
    setForm(prev => ({
      ...prev,
      name: info.name ?? prev.name,
      address: info.address ?? prev.address,
      phone: info.phone ?? prev.phone,
      nif: info.nif ?? prev.nif,
      email: info.email ?? prev.email,
      website: info.website ?? prev.website,
      business_day_cutoff: info.business_day_cutoff ?? prev.business_day_cutoff ?? 0,
    }));
    if (info.logo_url !== undefined) {
      setFormLogo(info.logo_url ?? '');
    }
  }, []);
  useLiveOrders(token, storeId, handleStoreInfoUpdated);

  useEffect(() => {
    const tk = useAuthStore.getState().token;
    if (!tk) return;
    (async () => {
      try {
        const [stores, info] = await Promise.all([
          getStores(tk),
          getStoreInfo(tk, storeId),
        ]);
        const s = stores.find(s => s.id === storeId);
        if (s) {
          setStore(s);
          setForm({
            alias: s.alias,
            name: s.name ?? '',
            address: s.address ?? '',
            phone: s.phone ?? '',
            nif: s.nif ?? '',
            email: s.email ?? '',
            website: s.website ?? '',
            business_day_cutoff: s.business_day_cutoff ?? 0,
          });
        }
        setFormLogo(info.logo_url ?? '');
      } catch (err) {
        if (err instanceof ApiError && err.status === 401) { clearAuth(); navigate('/login'); }
      } finally {
        setLoading(false);
      }
    })();

    // 并行加载设备列表（静默失败，设备列表是辅助信息）
    setDevicesLoading(true);
    getStoreDevices(tk, storeId)
      .then(data => setDevices(data))
      .catch(() => { /* 静默失败 */ })
      .finally(() => setDevicesLoading(false));
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [storeId]);

  const handleDeleteStore = async () => {
    if (!token) return;
    setDeleting(true);
    try {
      await deleteStore(token, storeId);
      navigate('/stores');
    } catch (e: unknown) {
      if (e instanceof ApiError) {
        setError(apiErrorMessage(t, e.code, e.message, e.status));
      } else {
        setError(t('auth.error_generic'));
      }
    } finally {
      setDeleting(false);
      setShowDeleteConfirm(false);
    }
  };

  const handleSave = async () => {
    if (!token) return;
    setSaving(true); setMsg(null);
    try {
      await Promise.all([
        updateStore(token, storeId, form),
        updateStoreInfo(token, storeId, { logo_url: formLogo || undefined }),
      ]);
      setMsg({ text: t('store.saved'), ok: true });
    } catch (err) {
      setMsg({ text: err instanceof ApiError ? apiErrorMessage(t, err.code, err.message, err.status) : t('auth.error_generic'), ok: false });
    } finally { setSaving(false); }
  };

  const update = (key: keyof typeof form, value: string | number) => setForm(prev => ({ ...prev, [key]: value }));

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

      {/* Alias + Name + Address */}
      <div className="bg-white rounded-2xl border border-slate-200 p-5 space-y-4">
        <Field label={t('store.alias')} value={form.alias} onChange={v => update('alias', v)} desc={t('store.alias_desc')} />
        <Field label={t('store.name')} value={form.name} onChange={v => update('name', v)} desc={t('store.name_desc')} />
        <Field label={t('store.address')} value={form.address} onChange={v => update('address', v)} icon={MapPin} />
      </div>

      {/* Contact + Web */}
      <div className="bg-white rounded-2xl border border-slate-200 p-5 space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Field label={t('store.phone')} value={form.phone} onChange={v => update('phone', v)} icon={Phone} />
          <Field label={t('store.email')} value={form.email} onChange={v => update('email', v)} icon={Mail} type="email" />
        </div>
        <Field label={t('store.website')} value={form.website} onChange={v => update('website', v)} icon={Globe} />
      </div>

      {/* Fiscal + Operations */}
      <div className="bg-white rounded-2xl border border-slate-200 p-5 space-y-4">
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <Field label={t('store.nif')} value={form.nif} onChange={v => update('nif', v)} icon={FileText} />
          <div>
            <label className="flex items-center gap-1.5 text-xs font-medium text-slate-500 mb-1.5">
              <Clock className="w-3.5 h-3.5" />
              {t('store.business_day_cutoff')}
            </label>
            <div className="flex items-center gap-2">
              <select
                value={form.business_day_cutoff}
                onChange={e => update('business_day_cutoff', Number(e.target.value))}
                className="flex-1 px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
              >
                {[0, 60, 120, 180, 210, 240, 300, 360, 420, 480].map(m => (
                  <option key={m} value={m}>{`${String(Math.floor(m / 60)).padStart(2, '0')}:${String(m % 60).padStart(2, '0')}`}</option>
                ))}
              </select>
            </div>
          </div>
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

      {/* Devices */}
      <div className="bg-white rounded-2xl border border-slate-200 p-5">
        <div className="flex items-center gap-2 mb-4">
          <Monitor className="w-4.5 h-4.5 text-slate-400" />
          <h3 className="font-bold text-slate-900">{t('store.devices')}</h3>
        </div>
        {devicesLoading ? (
          <p className="text-slate-400 text-sm">{t('store.devices_loading')}</p>
        ) : devices.length === 0 ? (
          <p className="text-slate-400 text-sm">{t('store.no_devices')}</p>
        ) : (
          <div className="divide-y divide-slate-100">
            {devices.map((d) => (
              <div key={d.entity_id} className="py-3 flex items-center justify-between">
                <div className="flex items-center gap-2.5">
                  <span className={`inline-flex items-center px-2 py-0.5 rounded-md text-xs font-medium ${
                    d.device_type === 'server' ? 'bg-blue-50 text-blue-700' : 'bg-purple-50 text-purple-700'
                  }`}>
                    {d.device_type === 'server' ? t('store.device_type_server') : t('store.device_type_client')}
                  </span>
                  <span className="font-mono text-sm text-slate-600">{d.device_id.slice(0, 8)}</span>
                  <span className={`inline-flex items-center px-2 py-0.5 rounded-md text-xs font-medium ${
                    d.status === 'active' ? 'bg-emerald-50 text-emerald-700' :
                    d.status === 'replaced' ? 'bg-slate-100 text-slate-400' :
                    'bg-red-50 text-red-600'
                  }`}>
                    {d.status}
                  </span>
                </div>
                <span className="text-xs text-slate-400">
                  {new Date(d.activated_at).toLocaleDateString()}
                </span>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Danger Zone */}
      <div className="bg-white rounded-2xl border border-red-200/60 p-5">
        <div className="flex items-center gap-2 mb-3">
          <AlertTriangle className="w-4.5 h-4.5 text-red-400" />
          <h3 className="font-bold text-red-600">{t('store.danger_zone')}</h3>
        </div>
        <p className="text-sm text-slate-500 mb-4">{t('store.danger_zone_desc')}</p>
        {error && <p className="text-sm text-red-500 mb-3">{error}</p>}
        <button
          onClick={() => setShowDeleteConfirm(true)}
          className="px-4 py-2.5 bg-red-500 text-white rounded-xl text-sm font-semibold hover:bg-red-600 active:scale-[0.98] transition-all flex items-center gap-2"
        >
          <Trash2 className="w-4 h-4" />
          {t('store.delete_store')}
        </button>
      </div>

      {/* Delete Confirmation Modal */}
      {showDeleteConfirm && (
        <div className="fixed inset-0 bg-black/40 backdrop-blur-sm flex items-center justify-center z-50">
          <div className="bg-white rounded-2xl p-6 max-w-md mx-4 shadow-xl">
            <h3 className="text-lg font-bold text-slate-900 mb-2">{t('store.confirm_delete')}</h3>
            <p className="text-sm text-slate-500 mb-5">
              {t('store.confirm_delete_desc').replace('{alias}', form.alias)}
            </p>
            <div className="flex justify-end gap-3">
              <button
                onClick={() => setShowDeleteConfirm(false)}
                className="px-4 py-2.5 text-sm font-medium text-slate-700 bg-slate-100 rounded-xl hover:bg-slate-200 transition-colors"
              >
                {t('common.action.cancel')}
              </button>
              <button
                onClick={handleDeleteStore}
                disabled={deleting}
                className="px-4 py-2.5 text-sm font-semibold text-white bg-red-500 rounded-xl hover:bg-red-600 disabled:opacity-50 transition-colors"
              >
                {deleting ? t('store.deleting') : t('store.confirm_delete')}
              </button>
            </div>
          </div>
        </div>
      )}
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
  desc?: string;
}> = ({ label, value, onChange, type = 'text', placeholder, icon: Icon, desc }) => (
  <div>
    <label className="block text-xs font-medium text-slate-500 mb-1.5">{label}</label>
    <div className="relative">
      {Icon && (
        <div className="absolute left-3.5 top-1/2 -translate-y-1/2 pointer-events-none">
          <Icon className="w-4 h-4 text-slate-400" />
        </div>
      )}
      <input
        type={type}
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder={placeholder}
        className={`w-full py-2.5 border border-slate-200 rounded-xl text-sm text-slate-800 placeholder:text-slate-300 focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-400 transition-all ${Icon ? 'pl-10 pr-3' : 'px-3'}`}
      />
    </div>
    {desc && <p className="mt-1 text-[11px] text-slate-400">{desc}</p>}
  </div>
);
