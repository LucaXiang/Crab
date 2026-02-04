/**
 * Shift Guard Component - 班次守卫
 *
 * 登录后自动检测班次状态:
 * - 如果有跨营业日过期班次 → 先弹出**提示对话框**告知用户，确认后进入结班流程，结班后再开新班
 * - 如果没有打开的班次 → 弹出开班弹窗
 * - 用户必须开班才能继续使用 POS
 *
 * 事件驱动: edge-server 在营业日 cutoff 时间点检测过期班次，
 * 通过 settlement_required 事件通知前端，前端无需主动轮询。
 */

import React, { useEffect, useState } from 'react';
import { AlertTriangle } from 'lucide-react';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useShiftStore } from '@/core/stores/shift';
import { useShiftCloseGuard } from '@/core/hooks';
import { ShiftActionModal } from '@/features/shift';
import { useI18n } from '@/hooks/useI18n';

interface ShiftGuardProps {
  children: React.ReactNode;
}

export const ShiftGuard: React.FC<ShiftGuardProps> = ({ children }) => {
  const { t } = useI18n();
  const user = useAuthStore((state) => state.user);

  // 只在 POS 页面拦截窗口关闭（检查未关闭班次）
  useShiftCloseGuard();
  const {
    currentShift,
    needsOpenShift,
    forceClosedMessage,
    staleShift,
    fetchCurrentShift,
    setNeedsOpenShift,
    setForceClosedMessage,
    setStaleShift,
  } = useShiftStore();

  const [isChecking, setIsChecking] = useState(true);
  const [showOpenModal, setShowOpenModal] = useState(false);
  const [showCloseModal, setShowCloseModal] = useState(false);  // 跨日班次结班弹窗
  const [showForceClosedDialog, setShowForceClosedDialog] = useState(false);
  const [showSettlementNotice, setShowSettlementNotice] = useState(false);  // 收盘时间到达提示

  const [checkError, setCheckError] = useState<string | null>(null);

  const checkShift = async () => {
    if (!user) {
      setIsChecking(false);
      return;
    }

    setCheckError(null);
    setIsChecking(true);

    try {
      // 直接检查当前班次状态
      // edge-server 启动时已自动检测过期班次并通过 settlement_required 事件通知
      const shift = await fetchCurrentShift(user.id);
      setIsChecking(false);

      // 检查是否有过期班次需要先结班
      const { staleShift: currentStaleShift, forceClosedMessage: msg } = useShiftStore.getState();
      if (currentStaleShift) {
        // 有过期班次，先显示提示对话框
        setShowSettlementNotice(true);
      } else if (!shift && !msg) {
        // 没有班次且没有异常关闭通知，显示开班弹窗
        setShowOpenModal(true);
      }
    } catch (err) {
      console.error('[ShiftGuard] 班次检查失败:', err);
      setCheckError(err instanceof Error ? err.message : t('settings.shift.check_failed'));
      setIsChecking(false);
    }
  };

  // 登录后检查班次状态
  useEffect(() => {
    checkShift();
  }, [user, fetchCurrentShift]);

  // 监听 staleShift 变化（可能在检查完成后才收到 settlement_required 事件）
  useEffect(() => {
    if (staleShift && !isChecking && !showCloseModal && !showSettlementNotice) {
      // 先显示收盘提示对话框，用户确认后再进入结班流程
      setShowSettlementNotice(true);
    }
  }, [staleShift, isChecking, showCloseModal, showSettlementNotice]);

  // 监听异常关闭通知（班次已被系统关闭的情况）
  useEffect(() => {
    if (forceClosedMessage && !isChecking && !showCloseModal) {
      setShowForceClosedDialog(true);
    }
  }, [forceClosedMessage, isChecking, showCloseModal]);

  // 监听 needsOpenShift 标记
  useEffect(() => {
    if (needsOpenShift && !showOpenModal && !showForceClosedDialog && !showCloseModal && !showSettlementNotice && !isChecking) {
      setShowOpenModal(true);
    }
  }, [needsOpenShift, showOpenModal, showForceClosedDialog, showCloseModal, showSettlementNotice, isChecking]);

  // 收盘提示确认后 → 进入结班流程
  const handleSettlementNoticeConfirm = () => {
    setShowSettlementNotice(false);
    setShowCloseModal(true);
  };

  // 结班成功后 → 清除 staleShift，弹出开班弹窗
  const handleCloseSuccess = () => {
    setShowCloseModal(false);
    setStaleShift(null);
    setShowOpenModal(true);
  };

  // 异常关闭通知确认后 → 弹出开班弹窗
  const handleForceClosedConfirm = () => {
    setShowForceClosedDialog(false);
    setForceClosedMessage(null);
    setShowOpenModal(true);
  };

  // 开班成功后关闭弹窗
  const handleOpenSuccess = () => {
    setShowOpenModal(false);
    setNeedsOpenShift(false);
  };

  // 用户点击关闭弹窗 (不允许跳过)
  const handleCloseModal = () => {
    // 如果还没有班次，不允许关闭
    if (!currentShift) {
      return;
    }
    setShowOpenModal(false);
  };

  // 不允许跳过结班
  const handleCloseCancel = () => {
    // 有过期班次必须结班，不允许取消
  };

  // 检查中显示 loading
  if (isChecking) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="w-8 h-8 border-4 border-blue-500/30 border-t-blue-500 rounded-full animate-spin mx-auto mb-3" />
          <p className="text-gray-600">{t('settings.shift.checking')}</p>
        </div>
      </div>
    );
  }

  // 检查失败显示重试
  if (checkError) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center max-w-md px-6">
          <div className="w-12 h-12 rounded-full bg-red-100 flex items-center justify-center mx-auto mb-4">
            <AlertTriangle className="text-red-500" size={24} />
          </div>
          <h3 className="text-lg font-bold text-gray-800 mb-2">
            {t('settings.shift.checking')}
          </h3>
          <p className="text-sm text-gray-600 mb-6">{checkError}</p>
          <button
            onClick={() => checkShift()}
            className="px-6 py-2.5 bg-primary-500 text-white font-semibold rounded-xl hover:bg-primary-600 transition-colors"
          >
            {t('common.action.retry')}
          </button>
        </div>
      </div>
    );
  }

  return (
    <>
      {children}

      {/* 跨日班次结班弹窗 */}
      <ShiftActionModal
        open={showCloseModal}
        action="close"
        shift={staleShift}
        onClose={handleCloseCancel}
        onSuccess={handleCloseSuccess}
      />

      {/* 收盘时间到达提示 Dialog */}
      {showSettlementNotice && (
        <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200">
          <div className="bg-white rounded-2xl shadow-2xl w-full max-w-md overflow-hidden">
            {/* Header */}
            <div className="p-6 border-b border-gray-100">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-orange-100 flex items-center justify-center">
                  <AlertTriangle className="text-orange-500" size={20} />
                </div>
                <h3 className="text-xl font-bold text-gray-800">
                  {t('settings.shift.settlement_notice_title')}
                </h3>
              </div>
            </div>

            {/* Content */}
            <div className="p-6">
              <p className="text-sm text-gray-600">
                {t('settings.shift.settlement_notice_message')}
              </p>
            </div>

            {/* Action */}
            <div className="p-6 pt-0">
              <button
                onClick={handleSettlementNoticeConfirm}
                className="w-full py-3 bg-orange-500 text-white font-bold rounded-xl hover:bg-orange-600 transition-colors"
              >
                {t('settings.shift.go_to_settlement')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 异常关闭通知 Dialog（班次已被系统自动关闭的情况） */}
      {showForceClosedDialog && (
        <div className="fixed inset-0 z-50 bg-black/60 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200">
          <div className="bg-white rounded-2xl shadow-2xl w-full max-w-md overflow-hidden">
            {/* Header */}
            <div className="p-6 border-b border-gray-100">
              <div className="flex items-center gap-3">
                <div className="w-10 h-10 rounded-full bg-orange-100 flex items-center justify-center">
                  <AlertTriangle className="text-orange-500" size={20} />
                </div>
                <h3 className="text-xl font-bold text-gray-800">
                  {t('settings.shift.force_closed_notice_title')}
                </h3>
              </div>
            </div>

            {/* Content */}
            <div className="p-6">
              <div className="bg-orange-50 border border-orange-200 rounded-xl p-4">
                <p className="text-sm text-orange-800">
                  {forceClosedMessage}
                </p>
              </div>
              <p className="mt-4 text-sm text-gray-600">
                {t('settings.shift.force_closed_notice_hint')}
              </p>
            </div>

            {/* Action */}
            <div className="p-6 pt-0">
              <button
                onClick={handleForceClosedConfirm}
                className="w-full py-3 bg-emerald-600 text-white font-bold rounded-xl hover:bg-emerald-700 transition-colors"
              >
                {t('settings.shift.open_new_shift')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 开班弹窗 - 登录后自动显示 */}
      <ShiftActionModal
        open={showOpenModal && !showForceClosedDialog && !showCloseModal && !showSettlementNotice}
        action="open"
        shift={null}
        onClose={handleCloseModal}
        onSuccess={handleOpenSuccess}
      />
    </>
  );
};
