/**
 * 应用自动更新检查 hook
 *
 * 启动后延迟检查，发现新版本时通过回调通知。
 * 使用 @tauri-apps/plugin-updater 与 crab-cloud 通信。
 */

import { useEffect, useRef, useState, useCallback } from 'react';
import { check, type Update } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';
import { logger } from '@/utils/logger';

const CHECK_DELAY_MS = 10_000; // 启动后 10 秒检查
const CHECK_INTERVAL_MS = 4 * 60 * 60 * 1000; // 每 4 小时检查一次

export interface UpdateInfo {
  version: string;
  body: string;
}

export type UpdateStatus = 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'error';

export function useUpdateChecker() {
  const [status, setStatus] = useState<UpdateStatus>('idle');
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [progress, setProgress] = useState(0);
  const updateRef = useRef<Update | null>(null);

  const checkForUpdate = useCallback(async () => {
    // 开发模式跳过
    if (!('__TAURI__' in window)) return;

    try {
      setStatus('checking');
      const update = await check();

      if (update) {
        updateRef.current = update;
        setUpdateInfo({
          version: update.version,
          body: update.body ?? '',
        });
        setStatus('available');
        logger.info(`Update available: v${update.version}`);
      } else {
        setStatus('idle');
      }
    } catch (err) {
      logger.warn('Update check failed', { error: String(err) });
      setStatus('error');
    }
  }, []);

  const installUpdate = useCallback(async () => {
    const update = updateRef.current;
    if (!update) return;

    try {
      setStatus('downloading');
      let downloaded = 0;
      let contentLength = 0;

      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case 'Started':
            contentLength = event.data.contentLength ?? 0;
            break;
          case 'Progress':
            downloaded += event.data.chunkLength;
            if (contentLength > 0) {
              setProgress(Math.round((downloaded / contentLength) * 100));
            }
            break;
          case 'Finished':
            setStatus('ready');
            break;
        }
      });

      // 重启应用
      await relaunch();
    } catch (err) {
      logger.error('Update install failed', err);
      setStatus('error');
    }
  }, []);

  const dismiss = useCallback(() => {
    setStatus('idle');
    setUpdateInfo(null);
    updateRef.current = null;
  }, []);

  // 启动后延迟检查 + 定时检查
  useEffect(() => {
    const timeout = setTimeout(checkForUpdate, CHECK_DELAY_MS);
    const interval = setInterval(checkForUpdate, CHECK_INTERVAL_MS);
    return () => {
      clearTimeout(timeout);
      clearInterval(interval);
    };
  }, [checkForUpdate]);

  return { status, updateInfo, progress, checkForUpdate, installUpdate, dismiss };
}
