import React from 'react';
import { useStoreInfo } from '@/core/stores/settings';
import { useI18n } from '@/hooks/useI18n';
import { Save, Store, Building2, MapPin, Phone, Mail, Globe, CreditCard, ImageIcon } from 'lucide-react';
import { useDirtyForm } from '@/hooks/useDirtyForm';

export const StoreSettings: React.FC = () => {
  const { info, setInfo } = useStoreInfo();
  const { t } = useI18n();
  const { values: formData, handleChange, isDirty, reset } = useDirtyForm(info);

  const onInputChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const { name, value } = e.target;
    handleChange(name as keyof typeof formData, value);
  };

  const handleSave = () => {
    setInfo(formData);
    reset(formData);
  };

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
            disabled={!isDirty}
            className={`flex items-center px-6 py-2.5 rounded-xl text-sm font-semibold transition-all shadow-sm ${
              isDirty
                ? 'bg-blue-600 text-white hover:bg-blue-700 shadow-blue-200'
                : 'bg-gray-100 text-gray-400 cursor-not-allowed'
            }`}
          >
            <Save className="w-4 h-4 mr-2" />
            {t('common.action.save')}
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
                {t('settings.store.form.basicInfo')}
              </h3>
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
                 <div className="col-span-2">
                  <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.store.form.establishmentName')}</label>
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
                  <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.store.form.logoUrl')}</label>
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
                      placeholder={t('settings.store.form.logoHelp')}
                      className="block w-full pl-10 rounded-lg border-gray-300 shadow-sm focus:border-blue-500 focus:ring-blue-500 sm:text-sm p-2.5 border transition-colors"
                    />
                  </div>
                  <p className="mt-1.5 text-xs text-gray-500">
                    {t('settings.store.form.logoHelp')}
                  </p>
                </div>
              </div>
            </div>

            <div className="col-span-2 border-t border-gray-100 my-2"></div>

            {/* Contact Info Group */}
            <div className="col-span-2">
              <h3 className="text-sm font-bold text-gray-900 uppercase tracking-wider mb-6 flex items-center gap-2">
                <MapPin className="w-4 h-4 text-gray-400" />
                {t('settings.store.form.contactInfo')}
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
                      {t('settings.store.form.taxId')}
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

          </div>
        </div>
      </div>
    </div>
  );
};
