import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { useUIScale, useSetUIScale } from '@/core/stores/ui';
import { useI18n } from '@/hooks/useI18n';
import type { Locale } from '@/infrastructure/i18n';
import { Monitor, Zap, Trash2, ZoomIn, Plus, Minus, Languages, Keyboard } from 'lucide-react';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { useVirtualKeyboardStore, useVirtualKeyboardMode } from '@/core/stores/ui';

const SCALE_STEPS = [0.9, 0.95, 1, 1.05, 1.1, 1.15, 1.2, 1.25, 1.3];

export const SystemSettings: React.FC = () => {
  const { t, locale, setLocale } = useI18n();
  const navigate = useNavigate();
  const performanceMode = useSettingsStore((state) => state.performanceMode);
  const setPerformanceMode = useSettingsStore((state) => state.setPerformanceMode);

  const uiScale = useUIScale();
  const setUIScale = useSetUIScale();

  const vkbMode = useVirtualKeyboardMode();
  const setVkbMode = useVirtualKeyboardStore((s) => s.setMode);

  const [showClearCacheDialog, setShowClearCacheDialog] = useState(false);

  const isDev = import.meta.env.DEV;

  const scalePercent = Math.round(uiScale * 100);

  const handleClearCache = () => {
    window.location.reload();
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="bg-white rounded-xl border border-gray-200 p-6 shadow-sm">
        <div className="flex items-center gap-4">
          <div className="w-12 h-12 bg-blue-50 rounded-xl flex items-center justify-center">
            <Monitor className="w-6 h-6 text-blue-600" />
          </div>
          <div>
            <h2 className="text-xl font-bold text-gray-900">{t('settings.system.title')}</h2>
          </div>
        </div>
      </div>

      {/* Settings Card */}
      <div className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
        <div className="p-6 md:p-8">

          {/* Display Group */}
          <div>
            <h3 className="text-sm font-bold text-gray-900 uppercase tracking-wider mb-6 flex items-center gap-2">
              <ZoomIn className="w-4 h-4 text-gray-400" />
              {t('settings.system.ui_scale.title')}
            </h3>

            <div className="space-y-5">
              {/* UI Scale */}
              <div>
                <div className="flex items-center justify-between mb-3">
                  <p className="text-sm text-gray-500">{t('settings.system.ui_scale.description')}</p>
                  <div className="flex items-center gap-2">
                    <span className="text-lg font-bold text-blue-600 tabular-nums">{scalePercent}%</span>
                    {uiScale !== 1 && (
                      <button
                        onClick={() => setUIScale(1)}
                        className="px-2.5 py-1 text-xs font-medium text-gray-500 bg-gray-100 rounded-md hover:bg-gray-200 transition-colors"
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
                    className="w-9 h-9 flex items-center justify-center rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                  >
                    <Minus size={16} />
                  </button>
                  <div className="flex-1 flex items-center justify-center gap-1.5">
                    {SCALE_STEPS.map((step) => (
                      <button
                        key={step}
                        onClick={() => setUIScale(step)}
                        className={`w-2.5 h-2.5 rounded-full transition-all ${
                          Math.abs(uiScale - step) < 0.01
                            ? 'bg-blue-600 scale-125'
                            : step <= uiScale
                            ? 'bg-blue-300'
                            : 'bg-gray-200'
                        }`}
                      />
                    ))}
                  </div>
                  <button
                    onClick={() => setUIScale(uiScale + 0.05)}
                    disabled={uiScale >= 1.3}
                    className="w-9 h-9 flex items-center justify-center rounded-lg border border-gray-200 text-gray-600 hover:bg-gray-50 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
                  >
                    <Plus size={16} />
                  </button>
                </div>
              </div>

              {/* Language */}
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2.5">
                  <Languages className="w-4 h-4 text-gray-400" />
                  <span className="text-sm font-medium text-gray-700">{t('settings.language.title')}</span>
                </div>
                <select
                  value={locale}
                  onChange={(e) => setLocale(e.target.value as Locale)}
                  className="border border-gray-300 rounded-lg px-3 py-1.5 bg-white focus:border-blue-500 focus:ring-blue-500 text-sm transition-colors"
                >
                  <option value="zh-CN">{t('settings.language.lang.zh')}</option>
                  <option value="es-ES">{t('settings.language.lang.es')}</option>
                </select>
              </div>

              {/* Performance Mode */}
              <div className="flex items-center justify-between">
                <div>
                  <div className="flex items-center gap-2.5">
                    <Zap className={`w-4 h-4 ${performanceMode ? 'text-green-500' : 'text-gray-400'}`} />
                    <span className="text-sm font-medium text-gray-700">{t('settings.system.performance_mode')}</span>
                  </div>
                  <p className="text-xs text-gray-400 mt-0.5 ml-6.5">{t('settings.system.performance_mode_desc')}</p>
                </div>
                <label className="relative inline-flex items-center cursor-pointer">
                  <input
                    type="checkbox"
                    className="sr-only peer"
                    checked={performanceMode}
                    onChange={(e) => setPerformanceMode(e.target.checked)}
                  />
                  <div className="w-11 h-6 bg-gray-200 rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-0.5 after:left-0.5 after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-green-500" />
                </label>
              </div>

              {/* Virtual Keyboard */}
              <div className="flex items-center justify-between">
                <div>
                  <div className="flex items-center gap-2.5">
                    <Keyboard className={`w-4 h-4 ${vkbMode === 'always' ? 'text-blue-500' : vkbMode === 'auto' ? 'text-gray-500' : 'text-gray-400'}`} />
                    <span className="text-sm font-medium text-gray-700">{t('settings.system.virtual_keyboard')}</span>
                  </div>
                  <p className="text-xs text-gray-400 mt-0.5 ml-6.5">{t('settings.system.virtual_keyboard_desc')}</p>
                </div>
                <div className="flex bg-gray-100 rounded-lg p-0.5 gap-0.5">
                  {(['always', 'auto', 'never'] as const).map((mode) => (
                    <button
                      key={mode}
                      onClick={() => setVkbMode(mode)}
                      className={`px-3 py-1.5 text-xs font-medium rounded-md transition-colors ${
                        vkbMode === mode
                          ? 'bg-white text-blue-600 shadow-sm'
                          : 'text-gray-500 hover:text-gray-700'
                      }`}
                    >
                      {t(`settings.system.virtual_keyboard_${mode}`)}
                    </button>
                  ))}
                </div>
              </div>
            </div>
          </div>

          <div className="border-t border-gray-100 my-6" />

          {/* Danger Zone */}
          <div>
            <h3 className="text-sm font-bold text-gray-900 uppercase tracking-wider mb-6 flex items-center gap-2">
              <Trash2 className="w-4 h-4 text-gray-400" />
              {t('settings.system.clear_cache.title')}
            </h3>

            <div className="space-y-4">
              {/* Clear Cache */}
              <div className="flex items-center justify-between p-4 bg-primary-50/50 rounded-lg border border-primary-100">
                <div>
                  <p className="text-sm font-medium text-gray-700">{t('settings.system.clear_cache.title')}</p>
                  <p className="text-xs text-gray-400 mt-0.5">{t('settings.system.clear_cache.description')}</p>
                </div>
                <button
                  onClick={() => setShowClearCacheDialog(true)}
                  className="px-3.5 py-1.5 text-sm font-medium text-primary-600 bg-white border border-primary-200 rounded-lg hover:bg-primary-50 transition-colors"
                >
                  {t('common.action.clear')}
                </button>
              </div>

              {/* Debug - Dev Only */}
              {isDev && (
                <div className="flex items-center justify-between p-4 bg-orange-50/50 rounded-lg border border-orange-100">
                  <div>
                    <p className="text-sm font-medium text-gray-700">
                      {t('settings.system.debug_orders', { fallback: '订单调试' })}
                      <span className="ml-1.5 px-1.5 py-0.5 text-[10px] font-bold bg-orange-200 text-orange-700 rounded">DEV</span>
                    </p>
                    <p className="text-xs text-gray-400 mt-0.5">查看所有订单状态，排查幽灵订单问题</p>
                  </div>
                  <button
                    onClick={() => navigate('/debug/orders')}
                    className="px-3.5 py-1.5 text-sm font-medium text-orange-600 bg-white border border-orange-200 rounded-lg hover:bg-orange-50 transition-colors"
                  >
                    打开调试
                  </button>
                </div>
              )}
            </div>
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
