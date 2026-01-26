import React from 'react';
import { useNavigate } from 'react-router-dom';
import { AlertTriangle, RefreshCw, Clock, Shield, Wifi } from 'lucide-react';
import { useBridgeStore, useAppState } from '@/core/stores/bridge';
import { t } from '@/infrastructure/i18n';
import type { ActivationRequiredReason } from '@/core/domain/types/appState';

/** 获取激活原因的 i18n 消息 */
function getReasonMessage(reason: ActivationRequiredReason): string {
  switch (reason.code) {
    case 'CertificateExpired':
      return t('activation.reason.CertificateExpired', { days_overdue: reason.details.days_overdue });
    case 'CertificateExpiringSoon':
      return t('activation.reason.CertificateExpiringSoon', { days_remaining: reason.details.days_remaining });
    case 'ClockTampering':
      if (reason.details.direction === 'backward') {
        return t('activation.reason.ClockTampering_backward', { hours: Math.floor(reason.details.drift_seconds / 3600) });
      }
      return t('activation.reason.ClockTampering_forward', { days: Math.floor(reason.details.drift_seconds / 86400) });
    default:
      return t(`activation.reason.${reason.code}`);
  }
}

/** 获取恢复建议的 i18n 消息 */
function getHintMessage(hintCode: string): string {
  // hintCode 格式: "hint.xxx" -> 转换为 "activation.hint.xxx"
  const key = hintCode.replace('hint.', 'activation.hint.');
  return t(key);
}

export const ActivationRequiredScreen: React.FC = () => {
  const navigate = useNavigate();
  const appState = useAppState();
  const isLoading = useBridgeStore((state) => state.isLoading);

  if (appState?.type !== 'ServerNeedActivation') {
    return null;
  }

  const { reason, can_auto_recover, recovery_hint } = appState.data;
  const message = getReasonMessage(reason);
  const hint = getHintMessage(recovery_hint);

  const getIcon = () => {
    switch (reason.code) {
      case 'ClockTampering':
        return <Clock className="text-yellow-500" size={48} />;
      case 'DeviceMismatch':
      case 'SignatureInvalid':
        return <Shield className="text-red-500" size={48} />;
      case 'NetworkError':
        return <Wifi className="text-orange-500" size={48} />;
      default:
        return <AlertTriangle className="text-yellow-500" size={48} />;
    }
  };

  const handleReactivate = () => {
    navigate('/setup', { replace: true });
  };

  return (
    <div className="min-h-screen w-full flex items-center justify-center p-8 bg-gray-50">
      <div className="max-w-md w-full bg-white rounded-2xl shadow-lg p-8">
        <div className="text-center mb-6">
          <div className="inline-flex items-center justify-center w-20 h-20 bg-gray-100 rounded-full mb-4">
            {getIcon()}
          </div>
          <h1 className="text-2xl font-bold text-gray-900 mb-2">{t('activation.title')}</h1>
          <p className="text-lg text-gray-600">{message}</p>
        </div>

        <div className="bg-gray-50 rounded-xl p-4 mb-6">
          <p className="text-sm text-gray-600">
            <strong>{t('common.label.suggestion')}：</strong> {hint}
          </p>
        </div>

        {reason.code === 'CertificateExpired' && (
          <div className="bg-red-50 border border-red-100 rounded-xl p-4 mb-6">
            <p className="text-sm text-red-600">
              {t('activation.detail.certificate_expired_at', { expired_at: reason.details.expired_at })}
            </p>
          </div>
        )}

        {reason.code === 'ClockTampering' && (
          <div className="bg-yellow-50 border border-yellow-100 rounded-xl p-4 mb-6">
            <p className="text-sm text-yellow-700">
              {reason.details.direction === 'backward'
                ? t('activation.detail.clock_backward')
                : t('activation.detail.clock_forward')}
            </p>
          </div>
        )}

        <div className="space-y-3">
          <button
            onClick={handleReactivate}
            disabled={isLoading}
            className="w-full py-3 bg-[#FF5E5E] text-white font-bold rounded-xl hover:bg-[#E54545] active:scale-[0.98] transition-all flex items-center justify-center gap-2 disabled:opacity-50"
          >
            <RefreshCw size={20} />
            {t('activation.button_reactivate')}
          </button>

          {can_auto_recover && (
            <button
              onClick={() => window.location.reload()}
              disabled={isLoading}
              className="w-full py-3 bg-gray-100 text-gray-700 font-medium rounded-xl hover:bg-gray-200 transition-all disabled:opacity-50"
            >
              {t('activation.button_retry_later')}
            </button>
          )}
        </div>
      </div>
    </div>
  );
};

export default ActivationRequiredScreen;
