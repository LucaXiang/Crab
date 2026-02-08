/**
 * Server Message Toast - 服务端消息推送 Toast
 *
 * 显示来自服务端的通知消息，支持：
 * - 4种级别：info, warning, error, critical
 * - 4种分类：system, printer, network, business
 * - 自动消失 + 进度条
 * - 堆叠显示多条消息
 */

import React, { useEffect, useState, useCallback, useRef } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import {
  Info,
  AlertTriangle,
  XCircle,
  AlertOctagon,
  Monitor,
  Printer,
  Wifi,
  ShoppingBag,
  X,
} from 'lucide-react';

// ============================================================================
// Types
// ============================================================================

type NotificationLevel = 'info' | 'warning' | 'error' | 'critical';
type NotificationCategory = 'system' | 'printer' | 'network' | 'business';

interface NotificationPayload {
  title: string;
  message: string;
  level: NotificationLevel;
  category: NotificationCategory;
  data?: unknown;
}

interface ServerMessage {
  event_type: string;
  payload: unknown;
  correlation_id?: string;
}

interface ToastItem {
  id: string;
  title: string;
  message: string;
  level: NotificationLevel;
  category: NotificationCategory;
  timestamp: number;
}

// ============================================================================
// Config
// ============================================================================

const MAX_VISIBLE_TOASTS = 10;
const TOAST_DISMISS_ANIMATION_MS = 300;

// Level 配置
const levelConfig: Record<
  NotificationLevel,
  {
    icon: React.ElementType;
    bgColor: string;
    borderColor: string;
    iconColor: string;
    accentColor: string;
  }
> = {
  info: {
    icon: Info,
    bgColor: 'bg-slate-800/95',
    borderColor: 'border-blue-500/50',
    iconColor: 'text-blue-400',
    accentColor: 'bg-blue-500',
  },
  warning: {
    icon: AlertTriangle,
    bgColor: 'bg-slate-800/95',
    borderColor: 'border-amber-500/50',
    iconColor: 'text-amber-400',
    accentColor: 'bg-amber-500',
  },
  error: {
    icon: XCircle,
    bgColor: 'bg-slate-800/95',
    borderColor: 'border-red-500/50',
    iconColor: 'text-red-400',
    accentColor: 'bg-red-500',
  },
  critical: {
    icon: AlertOctagon,
    bgColor: 'bg-red-900/95',
    borderColor: 'border-red-500',
    iconColor: 'text-red-300',
    accentColor: 'bg-red-400',
  },
};

// Category 图标
const categoryIcons: Record<NotificationCategory, React.ElementType> = {
  system: Monitor,
  printer: Printer,
  network: Wifi,
  business: ShoppingBag,
};

// ============================================================================
// Single Toast Component
// ============================================================================

interface ToastProps {
  item: ToastItem;
  onDismiss: (id: string) => void;
}

function ServerToast({ item, onDismiss }: ToastProps) {
  const [isExiting, setIsExiting] = useState(false);

  const config = levelConfig[item.level];
  const LevelIcon = config.icon;
  const CategoryIcon = categoryIcons[item.category];

  const handleDismiss = useCallback(() => {
    setIsExiting(true);
    setTimeout(() => onDismiss(item.id), TOAST_DISMISS_ANIMATION_MS);
  }, [item.id, onDismiss]);

  // Format timestamp
  const timeStr = new Date(item.timestamp).toLocaleTimeString('zh-CN', {
    hour: '2-digit',
    minute: '2-digit',
  });

  return (
    <div
      className={`
        relative overflow-hidden
        w-80 rounded-lg border shadow-2xl
        backdrop-blur-sm
        transform transition-all duration-300 ease-out
        ${config.bgColor} ${config.borderColor}
        ${isExiting ? 'opacity-0 translate-x-8 scale-95' : 'opacity-100 translate-x-0 scale-100'}
      `}
      role="alert"
    >
      {/* Left accent bar */}
      <div className={`absolute left-0 top-0 bottom-0 w-1 ${config.accentColor}`} />

      {/* Content */}
      <div className="p-4 pl-5">
        <div className="flex items-start gap-3">
          {/* Level Icon */}
          <div className={`flex-shrink-0 ${config.iconColor}`}>
            <LevelIcon size={22} strokeWidth={2} />
          </div>

          {/* Text Content */}
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2 mb-1">
              <h4 className="font-semibold text-white text-sm truncate">
                {item.title}
              </h4>
              {/* Category Badge */}
              <span className="flex items-center gap-1 px-1.5 py-0.5 rounded text-xs bg-white/10 text-gray-400">
                <CategoryIcon size={10} />
              </span>
            </div>
            <p className="text-gray-300 text-sm leading-relaxed">
              {item.message}
            </p>
            {/* Timestamp */}
            <p className="text-gray-500 text-xs mt-1">{timeStr}</p>
          </div>

          {/* Close Button */}
          <button
            onClick={handleDismiss}
            className="flex-shrink-0 p-1.5 rounded-full hover:bg-white/10 text-gray-400 hover:text-white transition-colors"
            title="关闭"
          >
            <X size={18} />
          </button>
        </div>
      </div>
    </div>
  );
}

// ============================================================================
// Toast Container (manages state)
// ============================================================================

// Global reference for manual toast triggering
let globalAddToast: ((notification: NotificationPayload) => void) | null = null;

export function ServerMessageToastContainer() {
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const addToast = useCallback((notification: NotificationPayload) => {
    const newToast: ToastItem = {
      id: `server-toast-${Date.now()}-${Math.random().toString(36).slice(2)}`,
      title: notification.title,
      message: notification.message,
      level: notification.level,
      category: notification.category,
      timestamp: Date.now(),
    };

    setToasts((prev) => {
      const updated = [newToast, ...prev];
      // Keep only MAX_VISIBLE_TOASTS
      return updated.slice(0, MAX_VISIBLE_TOASTS);
    });
  }, []);

  const removeToast = useCallback((id: string) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  // Register global handler
  useEffect(() => {
    globalAddToast = addToast;
    return () => {
      globalAddToast = null;
    };
  }, [addToast]);

  // Listen for server messages
  useEffect(() => {
    listen<ServerMessage>('server-message', (event) => {
      const msg = event.payload;

      // Only handle notification type messages
      if (msg.event_type === 'notification') {
        const notification = msg.payload as NotificationPayload;
        addToast(notification);
      }
    }).then((unlisten) => {
      unlistenRef.current = unlisten;
    });

    return () => {
      unlistenRef.current?.();
    };
  }, [addToast]);

  const clearAll = useCallback(() => {
    setToasts([]);
  }, []);

  if (toasts.length === 0) return null;

  return (
    <div className="fixed top-4 right-4 z-[10000] flex flex-col gap-3">
      {/* Clear All Button (when multiple toasts) */}
      {toasts.length > 1 && (
        <button
          onClick={clearAll}
          className="self-end px-3 py-1.5 text-xs font-medium text-gray-400 hover:text-white
                     bg-slate-800/90 hover:bg-slate-700 rounded-lg border border-slate-600/50
                     transition-colors shadow-lg"
        >
          全部清除 ({toasts.length})
        </button>
      )}
      {toasts.map((toast) => (
        <ServerToast key={toast.id} item={toast} onDismiss={removeToast} />
      ))}
    </div>
  );
}

// ============================================================================
// Manual trigger (for testing or programmatic use)
// ============================================================================

/**
 * Programmatically show server-style toasts
 *
 * @example
 * ```tsx
 * import { serverToast } from '@/presentation/components/ServerMessageToast';
 *
 * // Show different levels
 * serverToast.info('提示', '操作成功');
 * serverToast.warning('警告', '磁盘空间不足', 'system');
 * serverToast.error('错误', '打印失败', 'printer');
 * serverToast.critical('严重', '网络连接断开', 'network');
 * ```
 */
export const serverToast = {
  info: (title: string, message: string, category: NotificationCategory = 'system') => {
    globalAddToast?.({ title, message, level: 'info', category });
  },
  warning: (title: string, message: string, category: NotificationCategory = 'system') => {
    globalAddToast?.({ title, message, level: 'warning', category });
  },
  error: (title: string, message: string, category: NotificationCategory = 'system') => {
    globalAddToast?.({ title, message, level: 'error', category });
  },
  critical: (title: string, message: string, category: NotificationCategory = 'system') => {
    globalAddToast?.({ title, message, level: 'critical', category });
  },
};

// Re-export types for external use
export type { NotificationLevel, NotificationCategory, NotificationPayload };

