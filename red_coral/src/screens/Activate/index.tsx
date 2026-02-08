import React, { useState } from 'react';
import { useNavigate } from 'react-router-dom';
import { AlertCircle, ChevronRight, Shield, Power } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useBridgeStore } from '@/core/stores/bridge';
import { logger } from '@/utils/logger';
import { useI18n } from '@/hooks/useI18n';

// 订阅被阻止的状态
const BLOCKED_STATUSES = ['inactive', 'expired', 'canceled', 'unpaid'];

export const ActivateScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const {
    activateTenant,
    isLoading,
    error,
  } = useBridgeStore();

  // 激活表单状态
  const authUrl = 'http://127.0.0.1:3001';
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [activationError, setActivationError] = useState('');

  const handleActivate = async (e: React.FormEvent) => {
    e.preventDefault();
    setActivationError('');

    if (!username.trim() || !password.trim()) {
      setActivationError(t('auth.activate.error.empty_fields'));
      return;
    }

    try {
      const result = await activateTenant(authUrl, username, password);

      // 激活成功后立即检查订阅状态，blocked 直接跳转
      if (result.subscription_status && BLOCKED_STATUSES.includes(result.subscription_status)) {
        navigate('/status/subscription-blocked', { replace: true });
        return;
      }

      // 订阅正常，进入模式选择（Setup 页面）
      navigate('/setup', { replace: true });
    } catch (err: unknown) {
      // ApiError 已经在 invokeApi 中做过 friendlyError 处理，直接使用 message
      const msg = err instanceof Error ? err.message : String(err);
      setActivationError(msg);
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

