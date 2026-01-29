/**
 * Shift Guard Component - 班次守卫
 *
 * 登录后自动检测班次状态:
 * - 如果没有打开的班次 → 弹出开班弹窗
 * - 用户必须开班才能继续使用 POS
 */

import React, { useEffect, useState } from 'react';
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
    fetchCurrentShift,
    setNeedsOpenShift,
  } = useShiftStore();

  const [isChecking, setIsChecking] = useState(true);
  const [showOpenModal, setShowOpenModal] = useState(false);

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

      // 如果没有班次，显示开班弹窗
      if (!shift) {
        setShowOpenModal(true);
      }
    };

    checkShift();
  }, [user, fetchCurrentShift]);

  // 监听 needsOpenShift 标记
  useEffect(() => {
    if (needsOpenShift && !showOpenModal && !isChecking) {
      setShowOpenModal(true);
    }
  }, [needsOpenShift, showOpenModal, isChecking]);

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

      {/* 开班弹窗 - 登录后自动显示 */}
      <ShiftActionModal
        open={showOpenModal}
        action="open"
        shift={null}
        onClose={handleCloseModal}
        onSuccess={handleOpenSuccess}
      />
    </>
  );
};
