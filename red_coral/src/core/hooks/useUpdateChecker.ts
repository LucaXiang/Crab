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
  mandatory: boolean;
}

export type UpdateStatus = 'idle' | 'checking' | 'available' | 'downloading' | 'ready' | 'error';

const ERROR_AUTO_DISMISS_MS = 8_000;

export function useUpdateChecker() {
  const [status, setStatus] = useState<UpdateStatus>('idle');
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [progress, setProgress] = useState(0);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const updateRef = useRef<Update | null>(null);

  const checkForUpdate = useCallback(async () => {
    if (!('__TAURI_INTERNALS__' in window)) return;

    try {
      setStatus('checking');
      const update = await check();

      if (update) {
        updateRef.current = update;
        const rawBody = update.body ?? '';
        const mandatory = rawBody.startsWith('[MANDATORY]');
        const body = mandatory ? rawBody.replace(/^\[MANDATORY\]\n?/, '') : rawBody;
        setUpdateInfo({ version: update.version, body, mandatory });
        setStatus('available');
        logger.info(`Update available: v${update.version}${mandatory ? ' (mandatory)' : ''}`);
      } else {
        setStatus('idle');
      }
    } catch (err) {
      logger.warn('Update check failed', { error: String(err) });
      setStatus('error');
      setErrorMessage(String(err));
    }
  }, []);

  const restartApp = useCallback(async () => {
    try {
      await relaunch();
    } catch (err) {
      logger.error('Relaunch failed', err);
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
            break;
        }
      });

      setStatus('ready');
    } catch (err) {
      logger.error('Update install failed', err);
      setStatus('error');
      setErrorMessage(String(err));
    }
  }, []);

  const dismiss = useCallback(() => {
    // 强制更新不可关闭
    if (updateInfo?.mandatory) return;
    setStatus('idle');
    setUpdateInfo(null);
    updateRef.current = null;
  }, [updateInfo?.mandatory]);

  // 错误状态自动消失
  useEffect(() => {
    if (status !== 'error') return;
    const timer = setTimeout(() => {
      setStatus('idle');
      setErrorMessage(null);
    }, ERROR_AUTO_DISMISS_MS);
    return () => clearTimeout(timer);
  }, [status]);

  // 启动后延迟检查 + 定时检查
  useEffect(() => {
    const timeout = setTimeout(checkForUpdate, CHECK_DELAY_MS);
    const interval = setInterval(checkForUpdate, CHECK_INTERVAL_MS);
    return () => {
      clearTimeout(timeout);
      clearInterval(interval);
    };
  }, [checkForUpdate]);

  // 强制更新发现后自动开始下载
  useEffect(() => {
    if (status === 'available' && updateInfo?.mandatory) {
      installUpdate();
    }
  }, [status, updateInfo?.mandatory, installUpdate]);

  return { status, updateInfo, progress, errorMessage, checkForUpdate, installUpdate, restartApp, dismiss };
}
