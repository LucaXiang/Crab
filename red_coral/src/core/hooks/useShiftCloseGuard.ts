/**
 * Shift Close Guard Hook - 退出时班次关闭拦截
 *
 * 当用户尝试关闭应用时:
 * 1. 检查是否有打开的班次
 * 2. 如果有，显示确认对话框
 * 3. 用户可以选择: 收班(盘点现金) / 强制关闭 / 取消
 */

import { useEffect, useCallback, useRef } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { confirm, message } from '@tauri-apps/plugin-dialog';
import { createTauriClient } from '@/infrastructure/api/tauri-client';
import { logger } from '@/utils/logger';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { formatCurrency } from '@/utils/currency';
import type { Shift } from '@/core/domain/types/api';

const client = createTauriClient();

/**
 * 班次关闭守卫 Hook
 *
 * 在应用关闭时检查是否有打开的班次，并提示用户处理
 */
export function useShiftCloseGuard() {
  const user = useAuthStore(state => state.user);
  const unlistenRef = useRef<(() => void) | null>(null);

  const checkAndHandleOpenShift = useCallback(async (): Promise<boolean> => {
    // 如果用户未登录，直接允许关闭
    if (!user) {
      return true;
    }

    try {
      // 检查当前用户是否有打开的班次
      const openShift = await client.getCurrentShift();

      if (!openShift) {
        // 没有打开的班次，允许关闭
        return true;
      }

      // 有打开的班次，显示确认对话框
      const shouldClose = await confirm(
        `您有一个未关闭的班次 (开始于 ${formatTime(openShift.start_time)})。\n\n` +
        `预期现金: ${formatCurrency(openShift.expected_cash)}\n\n` +
        `是否强制关闭班次并退出？\n` +
        `(建议先进行收班操作以核对现金)`,
        {
          title: '班次未关闭',
          kind: 'warning',
          okLabel: '强制关闭并退出',
          cancelLabel: '取消退出',
        }
      );

      if (shouldClose) {
        // 用户选择强制关闭
        try {
          await client.forceCloseShift(openShift.id, {
            note: '应用退出时自动强制关闭',
          });
          await message('班次已强制关闭', { title: '提示', kind: 'info' });
          return true;
        } catch (err) {
          logger.error('Failed to force close shift', err);
          await message(`强制关闭班次失败: ${err}`, { title: '错误', kind: 'error' });
          return false;
        }
      }

      // 用户选择取消
      return false;
    } catch (err) {
      logger.error('Failed to check open shift', err);
      // 出错时允许关闭，避免用户无法退出
      return true;
    }
  }, [user]);

  useEffect(() => {
    let isSubscribed = true;

    const setupCloseHandler = async () => {
      try {
        const currentWindow = getCurrentWindow();

        // 监听窗口关闭请求
        const unlisten = await currentWindow.onCloseRequested(async (event) => {
          if (!isSubscribed) return;

          // 检查并处理打开的班次
          const canClose = await checkAndHandleOpenShift();

          if (!canClose) {
            // 阻止关闭
            event.preventDefault();
          }
        });

        unlistenRef.current = unlisten;
      } catch (err) {
        logger.error('Failed to setup close handler', err);
      }
    };

    setupCloseHandler();

    return () => {
      isSubscribed = false;
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, [checkAndHandleOpenShift]);
}

/**
 * 格式化时间显示
 */
function formatTime(millis: number): string {
  try {
    const date = new Date(millis);
    return date.toLocaleString('zh-CN', {
      month: '2-digit',
      day: '2-digit',
      hour: '2-digit',
      minute: '2-digit',
    });
  } catch {
    return String(millis);
  }
}
