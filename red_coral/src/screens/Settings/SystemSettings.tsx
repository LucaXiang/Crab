import React, { useState } from 'react';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { useUIScale, useSetUIScale } from '@/core/stores/ui';
import { useI18n } from '@/hooks/useI18n';
import { Monitor, Zap, Trash2, ZoomIn, Plus, Minus } from 'lucide-react';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';


export const SystemSettings: React.FC = () => {
  const { t } = useI18n();
  const performanceMode = useSettingsStore((state) => state.performanceMode);
  const setPerformanceMode = useSettingsStore((state) => state.setPerformanceMode);

  const uiScale = useUIScale();
  const setUIScale = useSetUIScale();

  const [showClearCacheDialog, setShowClearCacheDialog] = useState(false);

  const scalePercent = Math.round(uiScale * 100);

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

          {/* UI Scale Section */}
          <div className="p-6 bg-gray-50 rounded-xl border border-gray-100 transition-all hover:border-blue-100 hover:shadow-sm">
            <div className="flex items-center justify-between mb-4">
              <div className="flex gap-5">
                <div className="w-12 h-12 flex items-center justify-center rounded-xl bg-blue-100">
                  <ZoomIn className="text-blue-600" size={24} />
                </div>
                <div>
                  <div className="font-bold text-gray-900 text-lg mb-1">
                    {t('settings.system.ui_scale.title')}
                  </div>
                  <div className="text-sm text-gray-500 max-w-lg leading-relaxed">
                    {t('settings.system.ui_scale.description')}
                  </div>
                </div>
              </div>

              <div className="flex items-center gap-3">
                <span className="text-2xl font-bold text-blue-600 min-w-[4rem] text-right">
                  {scalePercent}%
                </span>
                {uiScale !== 1 && (
                  <button
                    onClick={() => setUIScale(1)}
                    className="px-3 py-1.5 text-sm font-medium text-gray-600 bg-white border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors"
                  >
                    {t('settings.system.ui_scale.reset')}
                  </button>
                )}
              </div>
            </div>

            <div className="flex items-center gap-3">
              <button
                onClick={() => setUIScale(uiScale - 0.05)}
                disabled={uiScale <= 0.9}
                className="w-10 h-10 flex items-center justify-center rounded-lg bg-white border border-gray-200 text-gray-700 hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                <Minus size={20} />
              </button>
              <div className="flex-1 flex items-center justify-center gap-1">
                {[0.9, 0.95, 1, 1.05, 1.1, 1.15, 1.2, 1.25, 1.3].map((step) => (
                  <div
                    key={step}
                    className={`w-2 h-2 rounded-full transition-colors ${
                      Math.abs(uiScale - step) < 0.01
                        ? 'bg-blue-600 scale-125'
                        : step <= uiScale
                        ? 'bg-blue-300'
                        : 'bg-gray-300'
                    }`}
                  />
                ))}
              </div>
              <button
                onClick={() => setUIScale(uiScale + 0.05)}
                disabled={uiScale >= 1.3}
                className="w-10 h-10 flex items-center justify-center rounded-lg bg-white border border-gray-200 text-gray-700 hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
              >
                <Plus size={20} />
              </button>
            </div>
          </div>

          {/* Performance Mode Section */}
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
