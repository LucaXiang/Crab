import React from 'react';
import { Download, X, RefreshCw, Loader2, AlertTriangle } from 'lucide-react';
import { useUpdateChecker, type UpdateStatus } from '@/core/hooks/useUpdateChecker';
import { t } from '@/infrastructure/i18n';

/**
 * 应用更新通知横幅
 *
 * 固定在屏幕顶部，发现新版本时显示。
 * 强制更新时自动下载，禁止关闭。
 */
export const UpdateNotification: React.FC = () => {
  const { status, updateInfo, progress, errorMessage, installUpdate, restartApp, dismiss } =
    useUpdateChecker();

  if (status === 'idle' || status === 'checking') {
    return null;
  }

  const isMandatory = updateInfo?.mandatory ?? false;

  // 强制更新使用红色背景
  const bgColor =
    status === 'error' ? 'bg-red-600' : isMandatory ? 'bg-red-600' : 'bg-blue-600';

  return (
    <div
      className={`fixed top-0 left-0 right-0 z-[9999] ${bgColor} text-white px-4 py-2.5 shadow-lg flex items-center justify-center gap-3 text-sm`}
    >
      <StatusContent
        status={status}
        version={updateInfo?.version}
        progress={progress}
        mandatory={isMandatory}
        errorMessage={errorMessage}
      />
      <div className="flex items-center gap-2 ml-2">
        {status === 'available' && !isMandatory && (
          <>
            <button
              onClick={installUpdate}
              className="flex items-center gap-1.5 px-3 py-1 bg-white text-blue-600 font-semibold rounded-lg hover:bg-blue-50 transition-colors"
            >
              <Download size={14} />
              {t('update.install')}
            </button>
            <button
              onClick={dismiss}
              className="p-1 hover:bg-blue-500 rounded transition-colors"
              title={t('update.later')}
            >
              <X size={16} />
            </button>
          </>
        )}
        {status === 'ready' && (
          <button
            onClick={restartApp}
            className="flex items-center gap-1.5 px-3 py-1 bg-white text-blue-600 font-semibold rounded-lg hover:bg-blue-50 transition-colors"
          >
            <RefreshCw size={14} />
            {t('update.restart')}
          </button>
        )}
      </div>
    </div>
  );
};

const StatusContent: React.FC<{
  status: UpdateStatus;
  version?: string;
  progress: number;
  mandatory: boolean;
  errorMessage: string | null;
}> = ({ status, version, progress, mandatory, errorMessage }) => {
  switch (status) {
    case 'available':
      return (
        <span>
          {mandatory ? (
            <AlertTriangle size={16} className="inline mr-1.5 -mt-0.5" />
          ) : (
            <Download size={16} className="inline mr-1.5 -mt-0.5" />
          )}
          {mandatory
            ? t('update.mandatory_available', { version: version ?? '' })
            : t('update.available', { version: version ?? '' })}
        </span>
      );
    case 'downloading':
      return (
        <span className="flex items-center gap-2">
          <Loader2 size={16} className="animate-spin" />
          {t('update.downloading', { progress: String(progress) })}
        </span>
      );
    case 'ready':
      return (
        <span>
          <RefreshCw size={16} className="inline mr-1.5 -mt-0.5" />
          {t('update.ready')}
        </span>
      );
    case 'error':
      return (
        <span className="flex items-center gap-2">
          <AlertTriangle size={16} />
          {t('update.error')}
          {errorMessage && <span className="opacity-75 text-xs">({errorMessage})</span>}
        </span>
      );
    default:
      return null;
  }
};
