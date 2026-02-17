import React from 'react';
import { Download, X, RefreshCw, Loader2 } from 'lucide-react';
import { useUpdateChecker, type UpdateStatus } from '@/core/hooks/useUpdateChecker';
import { t } from '@/infrastructure/i18n';

/**
 * 应用更新通知横幅
 *
 * 固定在屏幕顶部，发现新版本时显示。
 * 提供"立即更新"和"稍后"两个操作。
 */
export const UpdateNotification: React.FC = () => {
  const { status, updateInfo, progress, installUpdate, dismiss } = useUpdateChecker();

  if (status === 'idle' || status === 'checking' || status === 'error') {
    return null;
  }

  return (
    <div className="fixed top-0 left-0 right-0 z-[9999] bg-blue-600 text-white px-4 py-2.5 shadow-lg flex items-center justify-center gap-3 text-sm">
      <StatusContent status={status} version={updateInfo?.version} progress={progress} />
      <div className="flex items-center gap-2 ml-2">
        {status === 'available' && (
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
            >
              <X size={16} />
            </button>
          </>
        )}
        {status === 'ready' && (
          <button
            onClick={() => window.location.reload()}
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

const StatusContent: React.FC<{ status: UpdateStatus; version?: string; progress: number }> = ({
  status,
  version,
  progress,
}) => {
  switch (status) {
    case 'available':
      return (
        <span>
          <Download size={16} className="inline mr-1.5 -mt-0.5" />
          {t('update.available', { version: version ?? '' })}
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
    default:
      return null;
  }
};
