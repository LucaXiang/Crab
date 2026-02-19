import React, { useState } from 'react';
import { Ban, AlertTriangle, CreditCard, ExternalLink, Power, RefreshCw, LogOut } from 'lucide-react';
import { useNavigate } from 'react-router-dom';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAppState, useBridgeStore, AppStateHelpers } from '@/core/stores/bridge';
import { logger } from '@/utils/logger';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { t } from '@/infrastructure/i18n';
import type { SubscriptionStatus } from '@/core/domain/types/appState';
import type { AppState } from '@/core/stores/bridge/useBridgeStore';

/**
 * 订阅阻止状态的权限说明
 *
 * | 状态       | 系统权限     |
 * |-----------|-------------|
 * | Inactive  | 完全不可用   |
 * | Expired   | 只读         |
 * | Canceled  | 只读         |
 * | Unpaid    | 完全不可用   |
 */

/** 状态 → 图标 */
function getIcon(status: SubscriptionStatus) {
  switch (status) {
    case 'inactive':
    case 'unpaid':
      return <Ban className="text-red-500" size={48} />;
    case 'expired':
    case 'canceled':
      return <AlertTriangle className="text-yellow-500" size={48} />;
    default:
      return <CreditCard className="text-gray-500" size={48} />;
  }
}

/** 状态 → 颜色主题 */
function getTheme(status: SubscriptionStatus) {
  switch (status) {
    case 'inactive':
    case 'unpaid':
      return { bg: 'bg-red-50', border: 'border-red-100', text: 'text-red-600', badge: 'bg-red-100 text-red-700' };
    case 'expired':
    case 'canceled':
      return { bg: 'bg-yellow-50', border: 'border-yellow-100', text: 'text-yellow-700', badge: 'bg-yellow-100 text-yellow-700' };
    default:
      return { bg: 'bg-gray-50', border: 'border-gray-200', text: 'text-gray-600', badge: 'bg-gray-100 text-gray-700' };
  }
}

export const SubscriptionBlockedScreen: React.FC = () => {
  const navigate = useNavigate();
  const appState = useAppState();
  const exitTenant = useBridgeStore((s) => s.exitTenant);
  const [isChecking, setIsChecking] = useState(false);
  const [checkMessage, setCheckMessage] = useState<string | null>(null);
  const [showExitConfirm, setShowExitConfirm] = useState(false);

  if (appState?.type !== 'ServerSubscriptionBlocked') {
    const target = AppStateHelpers.getRouteForState(appState);
    navigate(target, { replace: true });
    return null;
  }

  const { info } = appState.data;
  const theme = getTheme(info.status);
  const statusLabel = t(`subscription.status.${statusKey(info.status)}`);

  const handleCloseApp = async () => {
    const appWindow = getCurrentWindow();
    await appWindow.close();
  };

  const handleExitTenant = async () => {
    try {
      await exitTenant();
      const newState = useBridgeStore.getState().appState;
      const route = AppStateHelpers.getRouteForState(newState);
      navigate(route, { replace: true });
    } catch (error) {
      logger.error('Exit tenant failed', error);
    }
  };

  const handleCheckSubscription = async () => {
    setIsChecking(true);
    setCheckMessage(null);
    try {
      const newState = await invokeApi<AppState>('check_subscription');
      // 更新全局 appState
      useBridgeStore.setState({ appState: newState });

      if (newState.type !== 'ServerSubscriptionBlocked') {
        // 订阅恢复正常，appState 已更新，路由守卫会自动导航
        // 无需手动跳转
      } else {
        // 仍然被阻止
        setCheckMessage(t('subscriptionBlocked.still_blocked'));
      }
    } catch {
      setCheckMessage(t('subscriptionBlocked.still_blocked'));
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
          <div className="inline-flex items-center justify-center w-20 h-20 bg-gray-100 rounded-full mb-4">
            {getIcon(info.status)}
          </div>
          <h1 className="text-2xl font-bold text-gray-900 mb-2">
            {t('subscriptionBlocked.title')}
          </h1>
          <p className="text-lg text-gray-600">
            {t(`subscriptionBlocked.message.${statusKey(info.status)}`)}
          </p>
        </div>

        {/* 状态标签 */}
        <div className="flex items-center justify-center mb-6">
          <span className={`px-3 py-1 rounded-full text-sm font-medium ${theme.badge}`}>
            {statusLabel}
          </span>
        </div>

        {/* 详情卡片 */}
        <div className={`${theme.bg} border ${theme.border} rounded-xl p-4 mb-6 space-y-2`}>
          <p className={`text-sm ${theme.text}`}>
            <strong>{t('subscriptionBlocked.plan')}:</strong>{' '}
            {t(`subscriptionBlocked.planType.${info.plan}`)}
          </p>
          {info.expired_at && (
            <p className={`text-sm ${theme.text}`}>
              <strong>{t('subscriptionBlocked.expired_at')}:</strong>{' '}
              {new Date(info.expired_at).toLocaleDateString()}
            </p>
          )}
          {info.in_grace_period && info.grace_period_ends_at && (
            <p className={`text-sm ${theme.text}`}>
              <strong>{t('subscriptionBlocked.grace_period_ends')}:</strong>{' '}
              {new Date(info.grace_period_ends_at).toLocaleDateString()}
            </p>
          )}
        </div>

        {/* 操作按钮 */}
        <div className="space-y-3">
          {/* 重新检查订阅状态 */}
          <button
            onClick={handleCheckSubscription}
            disabled={isChecking}
            className="w-full py-3 bg-blue-500 text-white font-bold rounded-xl hover:bg-blue-600 active:scale-[0.98] transition-all flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <RefreshCw size={20} className={isChecking ? 'animate-spin' : ''} />
            {isChecking
              ? t('subscriptionBlocked.rechecking')
              : t('subscriptionBlocked.button_recheck')}
          </button>

          {/* 检查结果提示 */}
          {checkMessage && (
            <p className="text-sm text-center text-orange-600">{checkMessage}</p>
          )}

          {info.renewal_url && (
            <a
              href={info.renewal_url}
              target="_blank"
              rel="noopener noreferrer"
              className="w-full py-3 bg-primary-500 text-white font-bold rounded-xl hover:bg-primary-600 active:scale-[0.98] transition-all flex items-center justify-center gap-2"
            >
              <CreditCard size={20} />
              {t('subscriptionBlocked.button_renew')}
            </a>
          )}
          {info.support_url && (
            <a
              href={info.support_url}
              target="_blank"
              rel="noopener noreferrer"
              className="w-full py-3 bg-gray-100 text-gray-700 font-medium rounded-xl hover:bg-gray-200 transition-all flex items-center justify-center gap-2"
            >
              <ExternalLink size={20} />
              {t('subscriptionBlocked.button_contact_support')}
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
                {t('subscriptionBlocked.button_exit_tenant')}
              </button>
            ) : (
              <div className="space-y-2">
                <p className="text-sm text-center text-gray-500">
                  {t('subscriptionBlocked.confirm_exit_tenant')}
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

/** SubscriptionStatus snake_case → i18n PascalCase key */
function statusKey(status: SubscriptionStatus): string {
  const map: Record<SubscriptionStatus, string> = {
    inactive: 'Inactive',
    active: 'Active',
    past_due: 'PastDue',
    expired: 'Expired',
    canceled: 'Canceled',
    unpaid: 'Unpaid',
  };
  return map[status];
}

