import React from 'react';
import { Navigate, useLocation } from 'react-router-dom';
import { useAppState, AppStateHelpers } from '@/core/stores/bridge';

interface ProtectedRouteProps {
  children: React.ReactNode;
}

/**
 * ProtectedRoute - 路由守卫组件
 *
 * 使用 AppState 状态机确保只有在正确状态下才能访问受保护的路由。
 * 参考设计文档: docs/plans/2026-01-18-application-state-machine.md
 */
export const ProtectedRoute: React.FC<ProtectedRouteProps> = ({ children }) => {
  const appState = useAppState();
  const location = useLocation();

  // 检查是否可以访问 POS
  if (!AppStateHelpers.canAccessPOS(appState)) {
    // 根据当前状态重定向到正确的页面
    const redirectTo = AppStateHelpers.getRouteForState(appState);
    return <Navigate to={redirectTo} state={{ from: location }} replace />;
  }

  return <>{children}</>;
};
