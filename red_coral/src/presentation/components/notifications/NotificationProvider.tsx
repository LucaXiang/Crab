import React, { createContext, useContext, useState, useCallback } from 'react';
import { useServerMessages, ServerMessage } from '@/core/hooks/useServerMessages';
import { NotificationToast, Notification } from './NotificationToast';

interface NotificationContextValue {
  notifications: Notification[];
  addNotification: (notification: Omit<Notification, 'id' | 'timestamp'>) => void;
  dismissNotification: (id: string) => void;
}

const NotificationContext = createContext<NotificationContextValue | null>(null);

export function useNotifications() {
  const ctx = useContext(NotificationContext);
  if (!ctx) throw new Error('useNotifications must be used within NotificationProvider');
  return ctx;
}

export function NotificationProvider({ children }: { children: React.ReactNode }) {
  const [notifications, setNotifications] = useState<Notification[]>([]);

  const addNotification = useCallback((notif: Omit<Notification, 'id' | 'timestamp'>) => {
    const id = crypto.randomUUID();
    setNotifications((prev) => [...prev, { ...notif, id, timestamp: Date.now() }]);

    // Auto dismiss after 5 seconds
    setTimeout(() => {
      setNotifications((prev) => prev.filter((n) => n.id !== id));
    }, 5000);
  }, []);

  const dismissNotification = useCallback((id: string) => {
    setNotifications((prev) => prev.filter((n) => n.id !== id));
  }, []);

  // Handle server messages
  useServerMessages(useCallback((msg: ServerMessage) => {
    if (msg.event_type === 'notification') {
      const payload = msg.payload as { level?: string; title?: string; message?: string };
      addNotification({
        level: (payload.level as Notification['level']) || 'info',
        title: payload.title || 'Notification',
        message: payload.message || '',
      });
    }
  }, [addNotification]));

  return (
    <NotificationContext.Provider value={{ notifications, addNotification, dismissNotification }}>
      {children}
      {/* Notification container */}
      <div className="fixed top-4 right-4 z-50 w-80">
        {notifications.map((n) => (
          <NotificationToast key={n.id} notification={n} onDismiss={dismissNotification} />
        ))}
      </div>
    </NotificationContext.Provider>
  );
}
