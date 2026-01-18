import React from 'react';
import { Permission } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';

interface ProtectedActionProps {
  permission: Permission;
  children: React.ReactNode;
  fallback?: React.ReactNode;
  mode?: 'hide' | 'disable';
}

/**
 * 权限保护组件 - 根据权限显示/隐藏/禁用子元素
 *
 * @example
 * <ProtectedAction permission={Permission.VOID_ORDER}>
 *   <button onClick={handleVoid}>作废订单</button>
 * </ProtectedAction>
 *
 * @example
 * <ProtectedAction permission={Permission.DELETE_PRODUCT} mode="disable">
 *   <button onClick={handleDelete}>删除</button>
 * </ProtectedAction>
 */
export const ProtectedAction: React.FC<ProtectedActionProps> = ({
  permission,
  children,
  fallback = null,
  mode = 'hide',
}) => {
  const { hasPermission } = usePermission();

  if (!hasPermission(permission)) {
    if (mode === 'disable' && React.isValidElement(children)) {
      // 禁用模式：克隆子元素并添加 disabled 属性和样式
      return React.cloneElement(children as React.ReactElement<any>, {
        disabled: true,
        style: { ...(children as React.ReactElement<any>).props.style, opacity: 0.5, cursor: 'not-allowed' },
        onClick: (e: React.MouseEvent) => {
            e.preventDefault();
            e.stopPropagation();
        }
      });
    }
    // 隐藏模式（默认）：显示 fallback 或 null
    return <>{fallback}</>;
  }

  return <>{children}</>;
};
