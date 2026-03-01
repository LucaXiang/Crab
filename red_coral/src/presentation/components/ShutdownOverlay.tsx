/**
 * Shutdown Overlay
 *
 * 应用关闭时显示的全屏遮罩，不可关闭、不可交互。
 * 监听 Tauri 事件 "app-shutting-down"，收到后立即渲染。
 */

import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { useI18n } from '@/hooks/useI18n';

export const ShutdownOverlay: React.FC = () => {
  const [isShuttingDown, setIsShuttingDown] = useState(false);
  const { t } = useI18n();

  useEffect(() => {
    const unlistenPromise = listen('app-shutting-down', () => {
      setIsShuttingDown(true);
    });

    return () => {
      unlistenPromise.then(unlisten => unlisten());
    };
  }, []);

  if (!isShuttingDown) return null;

  return (
    <div className="fixed inset-0 z-[200] flex items-center justify-center bg-black/70 backdrop-blur-sm">
      <div className="flex flex-col items-center gap-4">
        <div className="w-10 h-10 border-4 border-white/30 border-t-white rounded-full animate-spin" />
        <p className="text-white text-lg font-medium">{t('common.message.shutting_down')}</p>
      </div>
    </div>
  );
};
