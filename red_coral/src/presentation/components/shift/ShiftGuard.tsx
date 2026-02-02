/**
 * Shift Guard Component - 班次守卫
 *
 * 登录后自动检测班次状态:
 * - 如果没有打开的班次 → 弹出开班弹窗
 * - 如果班次被异常关闭 → 先显示通知 Dialog，再弹出开班弹窗
 * - 用户必须开班才能继续使用 POS
 */

import React, { useEffect, useState } from 'react';
import { AlertTriangle, X } from 'lucide-react';
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
    fetchCurrentShift,
    setNeedsOpenShift,
    setForceClosedMessage,
  } = useShiftStore();

  const [isChecking, setIsChecking] = useState(true);
  const [showOpenModal, setShowOpenModal] = useState(false);
  const [showForceClosedDialog, setShowForceClosedDialog] = useState(false);

  // 登录后检查班次状态
  useEffect(() => {
    const checkShift = async () => {
      if (!user) {
        setIsChecking(false);
        return;
      }

      setIsChecking(true);
      const shift = await fetchCurrentShift(user.id);
      setIsChecking(false);

      // 如果没有班次且没有异常关闭通知待展示，直接显示开班弹窗
      // 如果有异常关闭通知，由 forceClosedMessage 的 useEffect 处理流程
      if (!shift && !useShiftStore.getState().forceClosedMessage) {
        setShowOpenModal(true);
      }
    };

    checkShift();
  }, [user, fetchCurrentShift]);

  // 监听异常关闭通知
  useEffect(() => {
    if (forceClosedMessage && !isChecking) {
      setShowForceClosedDialog(true);
    }
  }, [forceClosedMessage, isChecking]);

  // 监听 needsOpenShift 标记
  useEffect(() => {
    if (needsOpenShift && !showOpenModal && !showForceClosedDialog && !isChecking) {
      setShowOpenModal(true);
    }
  }, [needsOpenShift, showOpenModal, showForceClosedDialog, isChecking]);

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

  // 用户点击关闭弹窗 (不允许跳过开班)
  const handleCloseModal = () => {
    // 如果还没有班次，不允许关闭
    if (!currentShift) {
      return;
    }
    setShowOpenModal(false);
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

  return (
    <>
      {children}

      {/* 异常关闭通知 Dialog */}
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

      {/* 开班弹窗 - 登录后自动显示 (与异常关闭对话框互斥) */}
      <ShiftActionModal
        open={showOpenModal && !showForceClosedDialog}
        action="open"
        shift={null}
        onClose={handleCloseModal}
        onSuccess={handleOpenSuccess}
      />
    </>
  );
};
