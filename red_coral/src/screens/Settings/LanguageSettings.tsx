import React, { useState } from 'react';
import { Languages } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { Locale } from '@/infrastructure/i18n';
import { toast } from '@/presentation/components/Toast';

export const LanguageSettings: React.FC = React.memo(() => {
  const { t, setLocale } = useI18n();
  const [selectedLocale, setSelectedLocaleState] = useState<Locale>('zh-CN');

  const handleSave = () => {
    setLocale(selectedLocale);
    toast.success(t('settings.status.saved'));
  };

  return (
    <div className="bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden">
      <div className="p-4 border-b border-gray-100 bg-gray-50 flex items-center gap-2 font-bold text-gray-700">
        <Languages size={18} />
        <span>{t('settings.language.title')}</span>
      </div>
      <div className="p-4 space-y-3">
        <div className="space-y-1">
          <label className="text-sm text-gray-700">{t('settings.language.title')}</label>
          <select
            value={selectedLocale}
            onChange={(e) => setSelectedLocaleState(e.target.value as Locale)}
            className="w-full border border-gray-200 rounded-lg p-2 bg-white focus:outline-none focus:ring-2 focus:ring-blue-200"
          >
            <option value="zh-CN">{t('settings.language.lang.zh')}</option>
            <option value="en-US">{t('settings.language.lang.en')}</option>
          </select>
        </div>
        <div className="flex items-center gap-2 pt-1">
          <button
            onClick={handleSave}
            className="ml-auto px-4 py-2 bg-blue-600 text-white rounded-xl text-sm font-bold shadow-md shadow-blue-100 hover:bg-blue-700"
          >
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
});
