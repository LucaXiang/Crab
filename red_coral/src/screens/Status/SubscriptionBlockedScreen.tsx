import React from 'react';
import { Ban, AlertTriangle, CreditCard, ExternalLink } from 'lucide-react';
import { useAppState } from '@/core/stores/bridge';
import { t } from '@/infrastructure/i18n';
import type { SubscriptionStatus } from '@/core/domain/types/appState';

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
  const appState = useAppState();

  if (appState?.type !== 'ServerSubscriptionBlocked') {
    return null;
  }

  const { info } = appState.data;
  const theme = getTheme(info.status);
  const statusLabel = t(`subscription.status.${statusKey(info.status)}`);

  return (
    <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
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

export default SubscriptionBlockedScreen;
