/**
 * useShiftRecovery - 启动时自动恢复跨营业日的僵尸班次
 *
 * 在 App 初始化时调用后端 recover_stale_shifts 接口，
 * 根据 business_day_cutoff 自动关闭跨营业日的未结班次。
 */

import { useEffect, useRef } from 'react';
import { useBridgeStore } from '@/core/stores/bridge';
import { useShiftStore } from '@/core/stores/shift';
import { createTauriClient } from '@/infrastructure/api';
import { toast } from '@/presentation/components/Toast';

export function useShiftRecovery() {
  const recovered = useRef(false);
  const appState = useBridgeStore((s) => s.appState);

  useEffect(() => {
    // 只在已认证状态下执行，且只执行一次
    if (recovered.current) return;
    if (appState?.type !== 'ServerAuthenticated' && appState?.type !== 'ClientAuthenticated') return;

    recovered.current = true;

    const run = async () => {
      try {
        const api = createTauriClient();
        const shifts = await api.recoverStaleShifts();
        if (shifts.length > 0) {
          console.log(`[ShiftRecovery] 自动关闭了 ${shifts.length} 个跨营业日僵尸班次`);
          // 通知 ShiftGuard 显示提示并引导开班
          useShiftStore.getState().setForceClosedMessage(
            `检测到 ${shifts.length} 个跨营业日未结班次，已自动关闭`
          );
          useShiftStore.getState().setNeedsOpenShift(true);
        }
      } catch (err) {
        console.warn('[ShiftRecovery] 恢复僵尸班次失败:', err);
        toast.warning('僵尸班次恢复失败，请手动检查班次状态');
      }
    };

    run();
  }, [appState?.type]);
}
