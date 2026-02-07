/**
 * Shift Store - 班次状态管理
 *
 * 管理当前用户的班次状态:
 * - 当前班次信息
 * - 开班/收班/强制关闭操作
 * - 自动检测未关闭班次
 */

import { create } from 'zustand';
import { createTauriClient } from '@/infrastructure/api';
import type { Shift, ShiftCreate, ShiftClose, ShiftForceClose } from '@/core/domain/types/api';

const getApi = () => createTauriClient();

interface ShiftStore {
  // State
  currentShift: Shift | null;
  isLoading: boolean;
  error: string | null;

  // 需要开班的标记 (登录后检测到没有班次)
  needsOpenShift: boolean;

  // 异常关闭通知消息 (显示后自动清除)
  forceClosedMessage: string | null;

  // 过期班次 (跨营业日未结班次，需要先强制关闭)
  staleShift: Shift | null;

  // Actions
  fetchCurrentShift: (operatorId: number) => Promise<Shift | null>;
  openShift: (data: ShiftCreate) => Promise<Shift>;
  closeShift: (shiftId: number, data: ShiftClose) => Promise<Shift>;
  forceCloseShift: (shiftId: number, data: ShiftForceClose) => Promise<Shift>;
  clearShift: () => void;
  setNeedsOpenShift: (needs: boolean) => void;
  setForceClosedMessage: (message: string | null) => void;
  setStaleShift: (shift: Shift | null) => void;
}

export const useShiftStore = create<ShiftStore>((set, get) => ({
  // Initial State
  currentShift: null,
  isLoading: false,
  error: null,
  needsOpenShift: false,
  forceClosedMessage: null,
  staleShift: null,

  /**
   * Fetch current open shift for operator
   */
  fetchCurrentShift: async (operatorId: number) => {
    set({ isLoading: true, error: null });
    try {
      const shift = await getApi().getCurrentShift(operatorId);
      set({ currentShift: shift, isLoading: false });

      // 如果没有班次，标记需要开班
      if (!shift) {
        set({ needsOpenShift: true });
      }

      return shift;
    } catch (error) {
      console.error('Failed to fetch current shift:', error);
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to fetch shift',
      });
      return null;
    }
  },

  /**
   * Open a new shift
   */
  openShift: async (data: ShiftCreate) => {
    set({ isLoading: true, error: null });
    try {
      const shift = await getApi().openShift(data);
      set({ currentShift: shift, isLoading: false, needsOpenShift: false });
      return shift;
    } catch (error) {
      console.error('Failed to open shift:', error);
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to open shift',
      });
      throw error;
    }
  },

  /**
   * Close current shift (正常收班)
   */
  closeShift: async (shiftId: number, data: ShiftClose) => {
    set({ isLoading: true, error: null });
    try {
      const shift = await getApi().closeShift(shiftId, data);
      set({ currentShift: null, isLoading: false });
      return shift;
    } catch (error) {
      console.error('Failed to close shift:', error);
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to close shift',
      });
      throw error;
    }
  },

  /**
   * Force close shift (强制关闭)
   */
  forceCloseShift: async (shiftId: number, data: ShiftForceClose) => {
    set({ isLoading: true, error: null });
    try {
      const shift = await getApi().forceCloseShift(shiftId, data);
      set({
        currentShift: null,
        isLoading: false,
        needsOpenShift: true,
        forceClosedMessage: data.note || '班次已被强制关闭',
      });
      return shift;
    } catch (error) {
      console.error('Failed to force close shift:', error);
      set({
        isLoading: false,
        error: error instanceof Error ? error.message : 'Failed to force close shift',
      });
      throw error;
    }
  },

  /**
   * Clear shift state (on logout)
   */
  clearShift: () => {
    set({ currentShift: null, needsOpenShift: false, forceClosedMessage: null, staleShift: null, error: null });
  },

  /**
   * Set needs open shift flag
   */
  setNeedsOpenShift: (needs: boolean) => {
    set({ needsOpenShift: needs });
  },

  /**
   * Set force closed message (异常关闭通知)
   */
  setForceClosedMessage: (message: string | null) => {
    set({ forceClosedMessage: message });
  },

  /**
   * Set stale shift (跨营业日未结班次)
   */
  setStaleShift: (shift: Shift | null) => {
    set({ staleShift: shift });
  },
}));

// Selectors
export const useCurrentShift = () => useShiftStore((state) => state.currentShift);
export const useNeedsOpenShift = () => useShiftStore((state) => state.needsOpenShift);
