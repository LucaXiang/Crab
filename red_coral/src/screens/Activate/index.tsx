import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { AlertCircle, ChevronRight, Shield, Power, Monitor, RefreshCw } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useBridgeStore } from '@/core/stores/bridge';
import type { QuotaInfo, ActiveDevice } from '@/core/stores/bridge';
import { ApiError } from '@/infrastructure/api/tauri-client';
import { ErrorCode } from '@/generated/error-codes';
import { logger } from '@/utils/logger';
import { useI18n } from '@/hooks/useI18n';

// 订阅被阻止的状态
const BLOCKED_STATUSES = ['inactive', 'expired', 'canceled', 'unpaid'];

function formatTimestamp(ts: number): string {
  if (!ts) return '-';
  return new Date(ts).toLocaleDateString(undefined, {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
    hour: '2-digit',
    minute: '2-digit',
  });
}

export const ActivateScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const {
    activateTenant,
    isLoading,
    error,
  } = useBridgeStore();

  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [activationError, setActivationError] = useState('');
  const [quotaInfo, setQuotaInfo] = useState<QuotaInfo | null>(null);
  const [replacingId, setReplacingId] = useState<string | null>(null);

  const handleActivate = async (e: React.FormEvent) => {
    e.preventDefault();
    setActivationError('');
    setQuotaInfo(null);

    if (!username.trim() || !password.trim()) {
      setActivationError(t('auth.activate.error.empty_fields'));
      return;
    }

    try {
      const result = await activateTenant(username, password);

      // 激活成功后立即检查订阅状态，blocked 直接跳转
      if (result.subscription_status && BLOCKED_STATUSES.includes(result.subscription_status)) {
        navigate('/status/subscription-blocked', { replace: true });
        return;
      }

      // 订阅正常，进入模式选择（Setup 页面）
      navigate('/setup', { replace: true });
    } catch (err: unknown) {
      if (err instanceof ApiError && err.code === ErrorCode.DeviceLimitReached) {
        // 设备数量已满，显示设备列表
        const qi = err.details?.quota_info as QuotaInfo | undefined;
        if (qi) {
          setQuotaInfo(qi);
          return;
        }
      }
      const msg = err instanceof Error ? err.message : String(err);
      setActivationError(msg);
    }
  };

  const handleReplace = async (device: ActiveDevice) => {
    setReplacingId(device.entity_id);
    setActivationError('');

    try {
      const result = await activateTenant(username, password, device.entity_id);
      setQuotaInfo(null);

      if (result.subscription_status && BLOCKED_STATUSES.includes(result.subscription_status)) {
        navigate('/status/subscription-blocked', { replace: true });
        return;
      }

      navigate('/setup', { replace: true });
    } catch (err: unknown) {
      const msg = err instanceof Error ? err.message : String(err);
      setActivationError(msg);
    } finally {
      setReplacingId(null);
    }
  };

  const handleCloseApp = async () => {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.destroy();
    } catch (err) {
      logger.error('Failed to close app', err);
    }
  };

  // 设备替换选择界面
  if (quotaInfo) {
    return (
      <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
        <button
          onClick={handleCloseApp}
          className="absolute top-6 right-6 p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors z-20"
          title={t('common.dialog.close_app')}
        >
          <Power size={24} />
        </button>

        <div className="w-full max-w-lg mx-auto space-y-6">
          <div className="text-center">
            <div className="inline-flex items-center justify-center w-16 h-16 bg-amber-500/10 rounded-2xl mb-4">
              <Monitor className="text-amber-500" size={32} />
            </div>
            <h1 className="text-2xl font-bold text-gray-900 mb-2">
              {t('auth.activate.device_limit.title')}
            </h1>
            <p className="text-gray-500">
              {t('auth.activate.device_limit.description', {
                max: String(quotaInfo.max_edge_servers),
                count: String(quotaInfo.active_count),
              })}
            </p>
          </div>

          {activationError && (
            <div className="flex items-center gap-3 text-red-600 bg-red-50 p-4 rounded-xl border border-red-100">
              <AlertCircle size={20} className="shrink-0" />
              <span className="text-sm font-medium">{activationError}</span>
            </div>
          )}

          <div className="space-y-3">
            {quotaInfo.active_devices.map((device) => (
              <div
                key={device.entity_id}
                className="bg-white rounded-xl border border-gray-200 p-4 flex items-center justify-between gap-4"
              >
                <div className="min-w-0 flex-1">
                  <div className="font-medium text-gray-900 truncate text-sm">
                    {device.entity_id}
                  </div>
                  <div className="text-xs text-gray-500 mt-1 space-y-0.5">
                    <div>
                      {t('auth.activate.device_limit.device_id')}: {device.device_id.slice(0, 12)}...
                    </div>
                    <div>
                      {t('auth.activate.device_limit.activated_at')}: {formatTimestamp(device.activated_at)}
                    </div>
                    {device.last_refreshed_at && (
                      <div>
                        {t('auth.activate.device_limit.last_refreshed')}: {formatTimestamp(device.last_refreshed_at)}
                      </div>
                    )}
                  </div>
                </div>
                <button
                  onClick={() => handleReplace(device)}
                  disabled={replacingId !== null}
                  className="shrink-0 px-4 py-2 text-sm font-medium text-white bg-amber-500 rounded-lg hover:bg-amber-600 active:scale-[0.98] transition-all disabled:opacity-50 flex items-center gap-2"
                >
                  {replacingId === device.entity_id ? (
                    <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  ) : (
                    <RefreshCw size={14} />
                  )}
                  <span>
                    {replacingId === device.entity_id
                      ? t('auth.activate.device_limit.replacing')
                      : t('auth.activate.device_limit.button_replace')}
                  </span>
                </button>
              </div>
            ))}
          </div>

          <button
            onClick={() => setQuotaInfo(null)}
            disabled={replacingId !== null}
            className="w-full py-3 text-gray-600 font-medium rounded-xl border border-gray-200 hover:bg-gray-50 transition-colors disabled:opacity-50"
          >
            {t('auth.activate.device_limit.button_cancel')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
      {/* 关闭按钮 */}
      <button
        onClick={handleCloseApp}
        className="absolute top-6 right-6 p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors z-20"
        title={t('common.dialog.close_app')}
      >
        <Power size={24} />
      </button>

      <div className="w-full max-w-md mx-auto space-y-8">
        <div className="text-center">
          <div className="inline-flex items-center justify-center w-16 h-16 bg-primary-500/10 rounded-2xl mb-4">
            <Shield className="text-primary-500" size={32} />
          </div>
          <h1 className="text-2xl font-bold text-gray-900 mb-2">{t('auth.activate.title')}</h1>
          <p className="text-gray-500">{t('auth.activate.description')}</p>
        </div>

        <form onSubmit={handleActivate} className="space-y-6">
          {/* Username */}
          <div className="space-y-1">
            <label className="text-sm font-medium text-gray-700">{t('auth.activate.username_label')}</label>
            <input
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder={t('auth.activate.username_placeholder')}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
              disabled={isLoading}
            />
          </div>

          {/* Password */}
          <div className="space-y-1">
            <label className="text-sm font-medium text-gray-700">{t('auth.activate.password_label')}</label>
            <input
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder={t('auth.activate.password_placeholder')}
              className="w-full px-4 py-3 border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
              disabled={isLoading}
            />
          </div>

          {/* 错误信息 */}
          {(activationError || error) && (
            <div className="flex items-center gap-3 text-red-600 bg-red-50 p-4 rounded-xl border border-red-100">
              <AlertCircle size={20} className="shrink-0" />
              <span className="text-sm font-medium">{activationError || error}</span>
            </div>
          )}

          {/* 提交按钮 */}
          <button
            type="submit"
            disabled={isLoading}
            className="w-full py-3 bg-primary-500 text-white font-bold rounded-xl hover:bg-primary-600 active:scale-[0.98] transition-all shadow-lg shadow-primary-500/25 flex items-center justify-center gap-2 disabled:opacity-70"
          >
            {isLoading ? (
              <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
            ) : (
              <>
                <span>{t('auth.activate.button_submit')}</span>
                <ChevronRight size={20} />
              </>
            )}
          </button>
        </form>
      </div>
    </div>
  );
};
