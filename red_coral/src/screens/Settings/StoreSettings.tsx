import React, { useEffect, useState } from 'react';
import { useStoreInfo, useStoreInfoStore } from '@/core/stores/settings';
import { useI18n } from '@/hooks/useI18n';
import { Save, Store, Phone, Mail, Globe, CreditCard, ImageIcon, Loader2, Clock } from 'lucide-react';
import { useDirtyForm } from '@/shared/hooks/useDirtyForm';
import { toast } from '@/presentation/components/Toast';
import { invoke } from '@tauri-apps/api/core';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
import { ProductImage } from '@/features/product/ProductImage';
import { getErrorMessage } from '@/utils/error';

export const StoreSettings: React.FC = () => {
  const info = useStoreInfo();
  const { fetchAll, updateStoreInfo, isLoading, isLoaded } = useStoreInfoStore();
  const { t } = useI18n();
  const [isSaving, setIsSaving] = useState(false);

  useEffect(() => {
    fetchAll();
  }, []);

  const formInfo = {
    name: info.name,
    address: info.address,
    nif: info.nif,
    logoUrl: info.logo_url || '',
    phone: info.phone || '',
    email: info.email || '',
    website: info.website || '',
    businessDayCutoff: info.business_day_cutoff || '02:00',
  };

  const { values: formData, handleChange, isDirty, reset } = useDirtyForm(formInfo);

  const onInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    handleChange(name as keyof typeof formData, value);
  };

  const handleSelectLogo = async () => {
    try {
      const file = await dialogOpen({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });
      if (!file || Array.isArray(file)) return;
      const hash = await invoke<string>('save_image', { sourcePath: file });
      handleChange('logoUrl', hash);
    } catch (e) {
      toast.error(getErrorMessage(e));
    }
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await updateStoreInfo({
        name: formData.name,
        address: formData.address,
        nif: formData.nif,
        logo_url: formData.logoUrl || null,
        phone: formData.phone || null,
        email: formData.email || null,
        website: formData.website || null,
        business_day_cutoff: formData.businessDayCutoff,
      });
      reset(formData);
      toast.success(t('common.message.save_success'));
    } catch {
      toast.error(t('common.message.error'));
    } finally {
      setIsSaving(false);
    }
  };

  if (isLoading && !isLoaded) {
    return (
      <div className="flex items-center justify-center h-64">
        <Loader2 className="w-8 h-8 animate-spin text-blue-600" />
      </div>
    );
  }

  const inputClass = "block w-full rounded-lg border border-gray-200 bg-gray-50/50 text-sm p-2.5 focus:bg-white focus:border-blue-400 focus:ring-1 focus:ring-blue-400 transition-all outline-none";
  const labelClass = "block text-xs font-medium text-gray-500 mb-1";

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="bg-white rounded-xl border border-gray-200 p-5 shadow-sm">
        <div className="flex justify-between items-center">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-blue-50 rounded-lg flex items-center justify-center">
              <Store className="w-5 h-5 text-blue-600" />
            </div>
            <div>
              <h2 className="text-lg font-bold text-gray-900">{t('settings.store.title')}</h2>
              <p className="text-xs text-gray-500">{t('settings.store.description')}</p>
            </div>
          </div>
          <button
            onClick={handleSave}
            disabled={!isDirty || isSaving}
            className={`flex items-center px-4 py-1.5 rounded-lg text-sm font-semibold transition-all ${
              isDirty && !isSaving
                ? 'bg-blue-600 text-white hover:bg-blue-700 shadow-sm shadow-blue-200'
                : 'bg-gray-100 text-gray-400 cursor-not-allowed'
            }`}
          >
            {isSaving ? (
              <Loader2 className="w-3.5 h-3.5 mr-1.5 animate-spin" />
            ) : (
              <Save className="w-3.5 h-3.5 mr-1.5" />
            )}
            {isSaving ? t('common.message.saving') : t('common.action.save')}
          </button>
        </div>
      </div>

      {/* Form Card */}
      <div className="bg-white rounded-xl border border-gray-200 shadow-sm p-5 md:p-6">

        {/* Row 1: Logo + Name + Address */}
        <div className="grid grid-cols-[auto_1fr] gap-x-5 gap-y-3">
          {/* Logo - square, spanning two rows */}
          <div className="row-span-2 relative self-stretch">
            <div
              onClick={handleSelectLogo}
              className="h-full aspect-square rounded-xl border-2 border-dashed border-gray-200 bg-gray-50 relative overflow-hidden cursor-pointer hover:border-blue-300 hover:bg-blue-50/50 transition-all group"
            >
              {formData.logoUrl ? (
                <ProductImage src={formData.logoUrl} alt="logo" className="absolute inset-0 w-full h-full object-cover" />
              ) : (
                <div className="absolute inset-0 flex items-center justify-center">
                  <div className="text-center">
                    <ImageIcon size={22} className="text-gray-300 mx-auto group-hover:text-blue-400 transition-colors" />
                    <span className="text-[10px] text-gray-400 mt-1 block group-hover:text-blue-500">Logo</span>
                  </div>
                </div>
              )}
            </div>
            {formData.logoUrl && (
              <button type="button" onClick={() => handleChange('logoUrl', '')} className="absolute -bottom-5 inset-x-0 text-[11px] text-red-500 hover:text-red-600 transition-colors text-center">
                {t('common.action.remove')}
              </button>
            )}
          </div>

          {/* Name */}
          <div>
            <label className={labelClass}>{t('settings.store.form.establishment_name')}</label>
            <input type="text" name="name" value={formData.name} onChange={onInputChange} autoComplete="new-password" className={inputClass} />
          </div>
          {/* Address */}
          <div>
            <label className={labelClass}>{t('settings.store.form.address')}</label>
            <input type="text" name="address" value={formData.address} onChange={onInputChange} autoComplete="new-password" className={inputClass} />
          </div>
        </div>

        <div className="border-t border-gray-100 my-5" />

        {/* Row 2: 4-column grid - NIF / Phone / Email / Web */}
        <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div>
            <label className={labelClass}>
              <span className="inline-flex items-center gap-1"><CreditCard className="w-3 h-3 text-gray-400" />{t('settings.store.form.tax_id')}</span>
            </label>
            <input type="text" name="nif" value={formData.nif} onChange={onInputChange} autoComplete="new-password" className={inputClass} />
          </div>
          <div>
            <label className={labelClass}>
              <span className="inline-flex items-center gap-1"><Phone className="w-3 h-3 text-gray-400" />{t('settings.store.form.phone')}</span>
            </label>
            <input type="text" name="phone" value={formData.phone || ''} onChange={onInputChange} autoComplete="new-password" className={inputClass} />
          </div>
          <div>
            <label className={labelClass}>
              <span className="inline-flex items-center gap-1"><Mail className="w-3 h-3 text-gray-400" />{t('settings.store.form.email')}</span>
            </label>
            <input type="email" name="email" value={formData.email || ''} onChange={onInputChange} autoComplete="new-password" className={inputClass} />
          </div>
          <div>
            <label className={labelClass}>
              <span className="inline-flex items-center gap-1"><Globe className="w-3 h-3 text-gray-400" />{t('settings.store.form.website')}</span>
            </label>
            <input type="text" name="website" value={formData.website || ''} onChange={onInputChange} autoComplete="new-password" className={inputClass} />
          </div>
        </div>

        <div className="border-t border-gray-100 my-5" />

        {/* Row 3: Business day cutoff */}
        <div className="flex items-start gap-6">
          <div>
            <label className={labelClass}>
              <span className="inline-flex items-center gap-1"><Clock className="w-3 h-3 text-gray-400" />{t('settings.store.form.business_day_cutoff')}</span>
            </label>
            <input type="time" name="businessDayCutoff" value={formData.businessDayCutoff} onChange={onInputChange} className={`${inputClass} w-32`} />
          </div>
          <div className="pt-5">
            <div className="flex gap-2">
              {[
                { value: '00:00', label: t('settings.store.form.cutoff_presets.midnight') },
                { value: '06:00', label: t('settings.store.form.cutoff_presets.early_morning') },
              ].map((preset) => (
                <button
                  key={preset.value}
                  type="button"
                  onClick={() => handleChange('businessDayCutoff', preset.value)}
                  className={`px-3 py-1 text-xs rounded-full border transition-colors ${
                    formData.businessDayCutoff === preset.value
                      ? 'bg-blue-50 border-blue-300 text-blue-700 font-medium'
                      : 'border-gray-200 text-gray-500 hover:border-gray-300'
                  }`}
                >
                  {preset.label}
                </button>
              ))}
            </div>
            <p className="mt-1.5 text-[11px] text-gray-400">{t('settings.store.form.business_day_cutoff_help')}</p>
          </div>
        </div>

      </div>
    </div>
  );
};
