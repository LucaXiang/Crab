import React, { useState } from 'react';
import { ShieldAlert, ExternalLink, Power, RefreshCw, LogOut, Upload } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAppState, useBridgeStore } from '@/core/stores/bridge';
import { logger } from '@/utils/logger';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { t } from '@/infrastructure/i18n';
import type { AppState } from '@/core/stores/bridge/useBridgeStore';

export const P12BlockedScreen: React.FC = () => {
  const appState = useAppState();
  const fetchAppState = useBridgeStore((s) => s.fetchAppState);
  const [isChecking, setIsChecking] = useState(false);
  const [checkMessage, setCheckMessage] = useState<string | null>(null);
  const [showExitConfirm, setShowExitConfirm] = useState(false);

  if (appState?.type !== 'ServerP12Blocked') {
    return null;
  }

  const { info } = appState.data;
  const isMissing = info.reason.code === 'Missing';

  const handleCloseApp = async () => {
    const appWindow = getCurrentWindow();
    await appWindow.close();
  };

  const handleExitTenant = async () => {
    try {
      await invokeApi('exit_tenant');
      await fetchAppState();
    } catch (error) {
      logger.error('Exit tenant failed', error);
    }
  };

  const handleCheckP12 = async () => {
    setIsChecking(true);
    setCheckMessage(null);
    try {
      const newState = await invokeApi<AppState>('check_subscription');
      useBridgeStore.setState({ appState: newState });

      if (newState.type === 'ServerP12Blocked') {
        setCheckMessage(t('p12Blocked.still_blocked'));
      }
    } catch {
      setCheckMessage(t('p12Blocked.still_blocked'));
    } finally {
      setIsChecking(false);
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

      <div className="max-w-md w-full bg-white rounded-2xl shadow-lg p-8">
        {/* 图标 + 标题 */}
        <div className="text-center mb-6">
          <div className="inline-flex items-center justify-center w-20 h-20 bg-orange-100 rounded-full mb-4">
            <ShieldAlert className="text-orange-500" size={48} />
          </div>
          <h1 className="text-2xl font-bold text-gray-900 mb-2">
            {t('p12Blocked.title')}
          </h1>
          <p className="text-lg text-gray-600">
            {isMissing
              ? t('p12Blocked.message.missing')
              : t('p12Blocked.message.expired')}
          </p>
        </div>

        {/* 状态标签 */}
        <div className="flex items-center justify-center mb-6">
          <span className={`px-3 py-1 rounded-full text-sm font-medium ${
            isMissing ? 'bg-orange-100 text-orange-700' : 'bg-red-100 text-red-700'
          }`}>
            {isMissing ? t('p12Blocked.status.missing') : t('p12Blocked.status.expired')}
          </span>
        </div>

        {/* 详情卡片 */}
        <div className="bg-orange-50 border border-orange-100 rounded-xl p-4 mb-6 space-y-2">
          <p className="text-sm text-orange-700">
            <strong>{t('p12Blocked.tenant')}:</strong> {info.tenant_id}
          </p>
          {!isMissing && info.reason.code === 'Expired' && (
            <p className="text-sm text-orange-700">
              <strong>{t('p12Blocked.expired_at')}:</strong>{' '}
              {new Date(info.reason.details.expired_at).toLocaleDateString()}
            </p>
          )}
        </div>

        {/* 操作按钮 */}
        <div className="space-y-3">
          {/* 重新检查 */}
          <button
            onClick={handleCheckP12}
            disabled={isChecking}
            className="w-full py-3 bg-blue-500 text-white font-bold rounded-xl hover:bg-blue-600 active:scale-[0.98] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <RefreshCw size={20} className={isChecking ? 'animate-spin' : ''} />
            {isChecking
              ? t('p12Blocked.rechecking')
              : t('p12Blocked.button_recheck')}
          </button>

          {/* 检查结果提示 */}
          {checkMessage && (
            <p className="text-sm text-center text-orange-600">{checkMessage}</p>
          )}

          {/* 上传 P12 链接 */}
          {info.upload_url && (
            <a
              href={info.upload_url}
              target="_blank"
              rel="noopener noreferrer"
              className="w-full py-3 bg-primary-500 text-white font-bold rounded-xl hover:bg-primary-600 active:scale-[0.98] transition-all flex items-center justify-center gap-2"
            >
              <Upload size={20} />
              {t('p12Blocked.button_upload')}
            </a>
          )}

          {/* 退出租户 */}
          <div className="pt-3 border-t border-gray-100">
            {!showExitConfirm ? (
              <button
                onClick={() => setShowExitConfirm(true)}
                className="w-full py-3 text-gray-400 hover:text-red-500 font-medium rounded-xl hover:bg-red-50 transition-all flex items-center justify-center gap-2"
              >
                <LogOut size={18} />
                {t('p12Blocked.button_exit_tenant')}
              </button>
            ) : (
              <div className="space-y-2">
                <p className="text-sm text-center text-gray-500">
                  {t('p12Blocked.confirm_exit_tenant')}
                </p>
                <div className="flex gap-2">
                  <button
                    onClick={() => setShowExitConfirm(false)}
                    className="flex-1 py-2 bg-gray-100 text-gray-600 font-medium rounded-xl hover:bg-gray-200 transition-all"
                  >
                    {t('common.cancel')}
                  </button>
                  <button
                    onClick={handleExitTenant}
                    className="flex-1 py-2 bg-red-500 text-white font-medium rounded-xl hover:bg-red-600 transition-all"
                  >
                    {t('common.confirm')}
                  </button>
                </div>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
