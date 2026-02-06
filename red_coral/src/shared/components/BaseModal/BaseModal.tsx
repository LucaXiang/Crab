import React, { useEffect, useCallback } from 'react';
import { X } from 'lucide-react';
import { Z_INDEX, type ZIndexValue } from '@/shared/constants/zIndex';

/**
 * Header 背景色变体
 */
export type HeaderVariant = 'default' | 'danger' | 'warning' | 'auth' | 'primary';

/**
 * Modal 最大宽度预设
 */
export type ModalMaxWidth = 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '4xl';

export interface BaseModalProps {
  /** 是否显示 Modal */
  isOpen: boolean;
  /** 关闭回调 */
  onClose: () => void;
  /** Modal 标题 */
  title: string;
  /** Header 背景色变体 */
  headerVariant?: HeaderVariant;
  /** z-index 层级 */
  zIndex?: ZIndexValue;
  /** 最大宽度 */
  maxWidth?: ModalMaxWidth;
  /** 内容区域 */
  children: React.ReactNode;
  /** Footer 区域 (可选) */
  footer?: React.ReactNode;
  /** 是否显示关闭按钮 */
  showCloseButton?: boolean;
  /** 点击背景是否关闭 */
  closeOnBackdropClick?: boolean;
  /** 是否使用强调遮罩 (60% 黑色,用于支付/危险操作) */
  emphasizedOverlay?: boolean;
  /** 自定义 className (应用于 Modal 容器) */
  className?: string;
}

/**
 * BaseModal - 统一的 Modal 基础组件
 *
 * 提供标准的三段式布局 (Header + Content + Footer) 和统一的样式规范。
 *
 * @example
 * ```tsx
 * <BaseModal
 *   isOpen={isOpen}
 *   onClose={handleClose}
 *   title="编辑商品"
 *   headerVariant="primary"
 *   zIndex={Z_INDEX.MODAL_MANAGEMENT}
 *   footer={
 *     <>
 *       <button onClick={handleClose}>取消</button>
 *       <button onClick={handleSave}>保存</button>
 *     </>
 *   }
 * >
 *   <ProductForm />
 * </BaseModal>
 * ```
 */
export const BaseModal: React.FC<BaseModalProps> = ({
  isOpen,
  onClose,
  title,
  headerVariant = 'default',
  zIndex = Z_INDEX.MODAL_BASE,
  maxWidth = '2xl',
  children,
  footer,
  showCloseButton = true,
  closeOnBackdropClick = true,
  emphasizedOverlay = false,
  className = '',
}) => {
  // ESC 键关闭
  useEffect(() => {
    if (!isOpen) return;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onClose();
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  // 背景点击关闭
  const handleBackdropClick = useCallback(
    (e: React.MouseEvent<HTMLDivElement>) => {
      if (closeOnBackdropClick && e.target === e.currentTarget) {
        onClose();
      }
    },
    [closeOnBackdropClick, onClose]
  );

  if (!isOpen) return null;

  // Header 背景色映射
  const headerBgMap: Record<HeaderVariant, string> = {
    default: 'bg-white',
    danger: 'bg-red-50',
    warning: 'bg-orange-50',
    auth: 'bg-teal-50',
    primary: 'bg-primary-50',
  };

  // 最大宽度映射
  const maxWidthMap: Record<ModalMaxWidth, string> = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg',
    xl: 'max-w-xl',
    '2xl': 'max-w-2xl',
    '4xl': 'max-w-4xl',
  };

  // z-index 类名
  const zIndexClass = zIndex <= 50 && zIndex % 10 === 0 ? `z-${zIndex}` : `z-[${zIndex}]`;

  // 遮罩透明度
  const overlayOpacity = emphasizedOverlay ? 'bg-black/60' : 'bg-black/50';

  return (
    <div
      className={`fixed inset-0 ${zIndexClass} ${overlayOpacity} backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200`}
      onClick={handleBackdropClick}
    >
      <div
        className={`bg-white rounded-2xl shadow-2xl w-full ${maxWidthMap[maxWidth]} flex flex-col max-h-[90vh] overflow-hidden animate-in zoom-in-95 duration-200 ${className}`}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className={`shrink-0 px-6 py-4 border-b border-gray-100 ${headerBgMap[headerVariant]}`}>
          <div className="flex items-center justify-between">
            <h2 className="text-xl font-bold text-gray-900">{title}</h2>
            {showCloseButton && (
              <button
                onClick={onClose}
                className="p-2 hover:bg-gray-100 rounded-full transition-colors"
                aria-label="关闭"
              >
                <X size={20} className="text-gray-500" />
              </button>
            )}
          </div>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-6">{children}</div>

        {/* Footer */}
        {footer && (
          <div className="shrink-0 px-6 py-4 border-t border-gray-100 bg-gray-50 flex justify-end gap-3">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
};
