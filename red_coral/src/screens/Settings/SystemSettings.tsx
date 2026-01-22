import React, { useState } from 'react';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { useI18n } from '@/hooks/useI18n';
import { Monitor, Zap, Type, Trash2 } from 'lucide-react';
import { ConfirmDialog } from '@/presentation/components/ui/ConfirmDialog';


export const SystemSettings: React.FC = () => {
  const { t } = useI18n();
  const performanceMode = useSettingsStore((state) => state.performanceMode);
  const setPerformanceMode = useSettingsStore((state) => state.setPerformanceMode);

  const [showClearCacheDialog, setShowClearCacheDialog] = useState(false);

  const handleClearCache = () => {
    localStorage.removeItem('pos-active-orders');
    // We iterate to remove individual order keys
    Object.keys(localStorage).forEach((key) => {
      if (key.startsWith('pos-active-order:') || key.startsWith('pos-active-events:')) {
        localStorage.removeItem(key);
      }
    });
    window.location.reload();
  };

  return (
    <div className="space-y-6">
      {/* Header Section */}
      <div className="bg-white rounded-xl border border-gray-200 p-6 shadow-sm">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 bg-blue-50 rounded-xl flex items-center justify-center">
            <Monitor className="w-6 h-6 text-blue-600" />
          </div>
          <div>
            <h2 className="text-xl font-bold text-gray-900">
              {t('settings.system.title')}
            </h2>
          </div>
        </div>
      </div>

      {/* Content Section */}
      <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
        <div className="p-6 md:p-8 space-y-6">

          <div className="flex items-center justify-between p-6 bg-gray-50 rounded-xl border border-gray-100 transition-all hover:border-blue-100 hover:shadow-sm">
            <div className="flex gap-5">
              <div className={`w-12 h-12 flex items-center justify-center rounded-xl transition-colors ${performanceMode ? 'bg-green-100' : 'bg-gray-200'}`}>
                <Zap className={performanceMode ? 'text-green-600' : 'text-gray-500'} size={24} />
              </div>
              <div>
                <div className="font-bold text-gray-900 text-lg mb-1">
                  {t('settings.system.performance_mode')}
                </div>
                <div className="text-sm text-gray-500 max-w-lg leading-relaxed">
                  {t('settings.system.performance_mode_desc')}
                </div>
              </div>
            </div>

            <label className="relative inline-flex items-center cursor-pointer ml-4">
              <input
                type="checkbox"
                className="sr-only peer"
                checked={performanceMode}
                onChange={(e) => setPerformanceMode(e.target.checked)}
              />
              <div className="w-14 h-7 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-green-300 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:left-1 after:bg-white after:border-gray-300 after:border after:rounded-full after:h-6 after:w-6 after:transition-all peer-checked:bg-green-500"></div>
            </label>
          </div>

          <div className="p-5 rounded-xl bg-blue-50 text-blue-800 text-sm flex items-start gap-4 border border-blue-100">
            <div className="p-2 bg-blue-100 rounded-lg shrink-0">
              <Type size={18} className="text-blue-600" />
            </div>
            <div>
              <div className="font-bold mb-1 text-blue-900">{t('settings.system.font_optimization.title')}</div>
              <p className="leading-relaxed text-blue-800/80">
                {t('settings.system.font_optimization.description')}
              </p>
            </div>
          </div>

          {/* Clear Cache Section */}
          <div className="flex items-center justify-between p-6 bg-red-50 rounded-xl border border-red-100 transition-all hover:border-red-200 hover:shadow-sm">
            <div className="flex gap-5">
              <div className="w-12 h-12 flex items-center justify-center rounded-xl bg-red-100 text-red-600 shrink-0">
                <Trash2 size={24} />
              </div>
              <div>
                <div className="font-bold text-gray-900 text-lg mb-1">
                  {t('settings.system.clear_cache.title')}
                </div>
                <div className="text-sm text-gray-500 max-w-lg leading-relaxed">
                  {t('settings.system.clear_cache.description')}
                </div>
              </div>
            </div>

            <button
              onClick={() => setShowClearCacheDialog(true)}
              className="px-4 py-2 bg-white border border-red-200 text-red-600 rounded-lg font-medium hover:bg-red-50 transition-colors shadow-sm"
            >
              {t('common.action.clear')}
            </button>
          </div>

        </div>
      </div>

      <ConfirmDialog
        isOpen={showClearCacheDialog}
        title={t('settings.system.clear_cache.confirm_title')}
        description={t('settings.system.clear_cache.confirm_description')}
        variant="danger"
        confirmText={t('common.action.confirm')}
        onConfirm={handleClearCache}
        onCancel={() => setShowClearCacheDialog(false)}
      />
    </div>
  );
};
