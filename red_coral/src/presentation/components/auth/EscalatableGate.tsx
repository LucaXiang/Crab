import React from 'react';
import { Lock } from 'lucide-react';
import { Permission, User } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';
import { usePermissionEscalationStore } from '@/core/stores/auth/usePermissionEscalationStore';
import { useI18n } from '@/hooks/useI18n';

interface EscalatableGateProps {
  permission: Permission;
  children: React.ReactNode;
  description?: string;
  onAuthorized?: (user: User) => void;
  /**
   * 展现模式
   * - 'block': 显示锁屏界面 (默认)
   * - 'intercept': 渲染子元素但拦截点击事件
   */
  mode?: 'block' | 'intercept';
}

/**
 * 权限控制组件 - 可提权
 * 如果用户没有权限，显示锁屏/提权界面
 * 点击后弹出授权弹窗，授权成功后可临时访问
 */
export const EscalatableGate: React.FC<EscalatableGateProps> = ({
  permission,
  children,
  description,
  onAuthorized,
  mode = 'block',
}) => {
  const { hasPermission } = usePermission();
  const { t } = useI18n();
  const openEscalation = usePermissionEscalationStore((state) => state.openEscalation);

  // 如果已经有权限，直接渲染内容
  if (hasPermission(permission)) {
    return <>{children}</>;
  }

  const handleEscalate = () => {
    openEscalation({
      requiredPermission: permission,
      description: description || t('common.auth.required'),
      onSuccess: (user) => {
        if (onAuthorized) {
          onAuthorized(user);
        }
      },
    });
  };

  if (mode === 'intercept') {
    // 拦截模式：渲染子元素，但拦截点击
    // 确保只有一个子元素
    const child = React.Children.only(children) as React.ReactElement<any>;

    const handleClick = (e: React.MouseEvent) => {
      e.preventDefault();
      e.stopPropagation();
      handleEscalate();
    };

    return React.cloneElement(child, {
      onClick: handleClick,
      // 可选：添加视觉提示
      title: description || t('common.auth.required'),
    });
  }

  // 默认 block 模式：显示锁屏界面
  return (
    <div className="flex flex-col items-center justify-center h-full min-h-[400px] p-8 text-center bg-gray-50/50 rounded-2xl border-2 border-dashed border-gray-200">
      <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mb-4">
        <Lock className="w-8 h-8 text-gray-400" />
      </div>
      <h3 className="text-xl font-semibold text-gray-900 mb-2">
        {t('common.auth.denied')}
      </h3>
      <p className="max-w-md mx-auto text-gray-500 mb-6">
        {description || t('common.auth.deniedMessage')}
      </p>
      <button
        onClick={handleEscalate}
        className="px-6 py-2.5 bg-blue-600 text-white font-semibold rounded-xl hover:bg-blue-700 transition-colors shadow-sm hover:shadow-md"
      >
        {t('common.auth.clickToAuthorize')}
      </button>
    </div>
  );
};
