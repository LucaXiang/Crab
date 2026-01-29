import React, { useEffect, useState } from 'react';
import { useStoreInfo, useStoreInfoStore } from '@/core/stores/settings';
import { useI18n } from '@/hooks/useI18n';
import { Save, Store, Building2, MapPin, Phone, Mail, Globe, CreditCard, ImageIcon, Loader2, Clock, Bug } from 'lucide-react';
import { useDirtyForm } from '@/shared/hooks/useDirtyForm';
import { toast } from '@/presentation/components/Toast';
import { createTauriClient } from '@/infrastructure/api';

export const StoreSettings: React.FC = () => {
  const info = useStoreInfo();
  const { fetchStoreInfo, updateStoreInfo, isLoading, isLoaded } = useStoreInfoStore();
  const { t } = useI18n();
  const [isSaving, setIsSaving] = useState(false);

  // Fetch store info on mount
  useEffect(() => {
    fetchStoreInfo();
  }, []);

  // Map API snake_case to form camelCase for consistency
  const formInfo = {
    name: info.name,
    address: info.address,
    nif: info.nif,
    logoUrl: info.logo_url || '',
    phone: info.phone || '',
    email: info.email || '',
    website: info.website || '',
    businessDayCutoff: info.business_day_cutoff || '00:00',
  };

  const { values: formData, handleChange, isDirty, reset } = useDirtyForm(formInfo);

  // Re-sync form when data is loaded
  useEffect(() => {
    if (isLoaded) {
      reset({
        name: info.name,
        address: info.address,
        nif: info.nif,
        logoUrl: info.logo_url || '',
        phone: info.phone || '',
        email: info.email || '',
        website: info.website || '',
        businessDayCutoff: info.business_day_cutoff || '00:00',
      });
    }
  }, [isLoaded, info]);

  const onInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    handleChange(name as keyof typeof formData, value);
  };

  const handleSave = async () => {
    setIsSaving(true);
    try {
      // Map form camelCase back to API snake_case
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

  return (
    <div className="space-y-6">
      {/* Header Section */}
      <div className="bg-white rounded-xl border border-gray-200 p-6 shadow-sm">
        <div className="flex justify-between items-center">
          <div className="flex items-center gap-4">
            <div className="w-12 h-12 bg-blue-50 rounded-xl flex items-center justify-center">
              <Store className="w-6 h-6 text-blue-600" />
            </div>
            <div>
              <h2 className="text-xl font-bold text-gray-900">
                {t('settings.store.title')}
              </h2>
              <p className="text-sm text-gray-500 mt-1">
                {t('settings.store.description')}
              </p>
            </div>
          </div>
          <button
            onClick={handleSave}
            disabled={!isDirty || isSaving}
            className={`flex items-center px-6 py-2.5 rounded-xl text-sm font-semibold transition-all shadow-sm ${
              isDirty && !isSaving
                ? 'bg-blue-600 text-white hover:bg-blue-700 shadow-blue-200'
                : 'bg-gray-100 text-gray-400 cursor-not-allowed'
            }`}
          >
            {isSaving ? (
              <Loader2 className="w-4 h-4 mr-2 animate-spin" />
            ) : (
              <Save className="w-4 h-4 mr-2" />
            )}
            {isSaving ? t('common.message.saving') : t('common.action.save')}
          </button>
        </div>
      </div>

      {/* Form Section */}
      <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
        <div className="p-6 md:p-8">
          <div className="grid grid-cols-1 gap-y-8 gap-x-8 sm:grid-cols-2">
            
            {/* Basic Info Group */}
            <div className="col-span-2">
              <h3 className="text-sm font-bold text-gray-900 uppercase tracking-wider mb-6 flex items-center gap-2">
                <Building2 className="w-4 h-4 text-gray-400" />
                {t('settings.store.form.basic_info')}
              </h3>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
                 <div className="col-span-2">
                  <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.store.form.establishment_name')}</label>
                  <div className="relative">
                    <input
                    type="text"
                    name="name"
                    value={formData.name}
                    onChange={onInputChange}
                    autoComplete="new-password"
                    className="block w-full rounded-lg border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm p-2.5 border transition-colors"
                  />
                  </div>
                </div>

                <div className="col-span-2">
                  <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.store.form.logo_url')}</label>
                  <div className="relative">
                    <div className="absolute inset-y-0 left-0 pl-3 flex items-center pointer-events-none">
                      <ImageIcon className="h-4 w-4 text-gray-400" />
                    </div>
                    <input
                      type="text"
                      name="logoUrl"
                      value={formData.logoUrl || ''}
                      onChange={onInputChange}
                      autoComplete="new-password"
                      placeholder={t('settings.store.form.logo_help')}
                      className="block w-full pl-10 rounded-lg border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm p-2.5 border transition-colors"
                    />
                  </div>
                  <p className="mt-1.5 text-xs text-gray-500">
                    {t('settings.store.form.logo_help')}
                  </p>
                </div>
              </div>
            </div>

            <div className="col-span-2 border-t border-gray-100 my-2"></div>

            {/* Contact Info Group */}
            <div className="col-span-2">
              <h3 className="text-sm font-bold text-gray-900 uppercase tracking-wider mb-6 flex items-center gap-2">
                <MapPin className="w-4 h-4 text-gray-400" />
                {t('settings.store.form.contact_info')}
              </h3>
              
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
                <div className="col-span-2">
                  <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.store.form.address')}</label>
                  <input
                    type="text"
                    name="address"
                    value={formData.address}
                    onChange={onInputChange}
                    autoComplete="new-password"
                    className="block w-full rounded-lg border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm p-2.5 border transition-colors"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    <div className="flex items-center gap-2">
                      <CreditCard className="w-3.5 h-3.5 text-gray-400" />
                      {t('settings.store.form.tax_id')}
                    </div>
                  </label>
                  <input
                    type="text"
                    name="nif"
                    value={formData.nif}
                    onChange={onInputChange}
                    autoComplete="new-password"
                    className="block w-full rounded-lg border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm p-2.5 border transition-colors"
                  />
                </div>

                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    <div className="flex items-center gap-2">
                      <Phone className="w-3.5 h-3.5 text-gray-400" />
                      {t('settings.store.form.phone')}
                    </div>
                  </label>
                  <input
                    type="text"
                    name="phone"
                    value={formData.phone || ''}
                    onChange={onInputChange}
                    autoComplete="new-password"
                    className="block w-full rounded-lg border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm p-2.5 border transition-colors"
                  />
                </div>

                <div className="col-span-2 sm:col-span-1">
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    <div className="flex items-center gap-2">
                      <Mail className="w-3.5 h-3.5 text-gray-400" />
                      {t('settings.store.form.email')}
                    </div>
                  </label>
                  <input
                    type="email"
                    name="email"
                    value={formData.email || ''}
                    onChange={onInputChange}
                    autoComplete="new-password"
                    className="block w-full rounded-lg border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm p-2.5 border transition-colors"
                  />
                </div>

                <div className="col-span-2 sm:col-span-1">
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    <div className="flex items-center gap-2">
                      <Globe className="w-3.5 h-3.5 text-gray-400" />
                      {t('settings.store.form.website')}
                    </div>
                  </label>
                  <input
                    type="text"
                    name="website"
                    value={formData.website || ''}
                    onChange={onInputChange}
                    autoComplete="new-password"
                    className="block w-full rounded-lg border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm p-2.5 border transition-colors"
                  />
                </div>
              </div>
            </div>

            <div className="col-span-2 border-t border-gray-100 my-2"></div>

            {/* Business Settings Group */}
            <div className="col-span-2">
              <h3 className="text-sm font-bold text-gray-900 uppercase tracking-wider mb-6 flex items-center gap-2">
                <Clock className="w-4 h-4 text-gray-400" />
                {t('settings.store.form.business_settings')}
              </h3>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    {t('settings.store.form.business_day_cutoff')}
                  </label>
                  <input
                    type="time"
                    name="businessDayCutoff"
                    value={formData.businessDayCutoff}
                    onChange={onInputChange}
                    className="block w-full rounded-lg border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm p-2.5 border transition-colors"
                  />
                  <p className="mt-1.5 text-xs text-gray-500">
                    {t('settings.store.form.business_day_cutoff_help')}
                  </p>
                  {/* 快捷预设 */}
                  <div className="mt-2 flex gap-2">
                    <button
                      type="button"
                      onClick={() => handleChange('businessDayCutoff', '00:00')}
                      className={`px-3 py-1 text-xs rounded-full border transition-colors ${
                        formData.businessDayCutoff === '00:00'
                          ? 'bg-blue-50 border-blue-300 text-blue-700'
                          : 'border-gray-200 text-gray-500 hover:border-gray-300'
                      }`}
                    >
                      {t('settings.store.form.cutoff_presets.midnight')}
                    </button>
                    <button
                      type="button"
                      onClick={() => handleChange('businessDayCutoff', '06:00')}
                      className={`px-3 py-1 text-xs rounded-full border transition-colors ${
                        formData.businessDayCutoff === '06:00'
                          ? 'bg-blue-50 border-blue-300 text-blue-700'
                          : 'border-gray-200 text-gray-500 hover:border-gray-300'
                      }`}
                    >
                      {t('settings.store.form.cutoff_presets.early_morning')}
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <div className="col-span-2 border-t border-gray-100 my-2"></div>

            {/* @TEST 上线前删除 - Debug Tools */}
            <div className="col-span-2">
              <h3 className="text-sm font-bold text-gray-900 uppercase tracking-wider mb-6 flex items-center gap-2">
                <Bug className="w-4 h-4 text-gray-400" />
                Debug
              </h3>

              <div className="flex gap-3">
                <button
                  type="button"
                  onClick={async () => {
                    try {
                      const api = createTauriClient();
                      const shifts = await api.debugSimulateShiftAutoClose();
                      if (shifts.length > 0) {
                        toast.success(`已模拟关闭 ${shifts.length} 个班次（已广播）`);
                      } else {
                        toast.warning('没有打开的班次');
                      }
                    } catch (err) {
                      toast.error(`模拟失败: ${err}`);
                    }
                  }}
                  className="px-4 py-2 text-sm bg-amber-50 border border-amber-300 text-amber-700 rounded-lg hover:bg-amber-100 transition-colors"
                >
                  模拟班次自动关闭（广播推送）
                </button>
              </div>
            </div>

          </div>
        </div>
      </div>
    </div>
  );
};
