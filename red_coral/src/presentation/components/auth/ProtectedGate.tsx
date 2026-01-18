import React from 'react';
import { Permission } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';

interface ProtectedGateProps {
  permission: Permission;
  children: React.ReactNode;
  fallback?: React.ReactNode;
}

/**
 * 权限控制组件 - 直接隐藏入口
 * 如果用户没有权限，则不渲染 children (渲染 null 或 fallback)
 */
export const ProtectedGate: React.FC<ProtectedGateProps> = ({
  permission,
  children,
  fallback = null,
}) => {
  const { hasPermission } = usePermission();

  if (!hasPermission(permission)) {
    return <>{fallback}</>;
  }

  return <>{children}</>;
};
