import React from 'react';

export interface Notification {
  id: string;
  level: 'info' | 'warning' | 'error' | 'success';
  title: string;
  message: string;
  timestamp: number;
}

interface Props {
  notification: Notification;
  onDismiss: (id: string) => void;
}

export function NotificationToast({ notification, onDismiss }: Props) {
  const levelStyles = {
    info: 'bg-blue-100 border-blue-500 text-blue-700',
    warning: 'bg-yellow-100 border-yellow-500 text-yellow-700',
    error: 'bg-red-100 border-red-500 text-red-700',
    success: 'bg-green-100 border-green-500 text-green-700',
  };

  return (
    <div
      className={`border-l-4 p-4 mb-2 rounded shadow-lg ${levelStyles[notification.level]}`}
      role="alert"
    >
      <div className="flex justify-between items-start">
        <div>
          <p className="font-bold">{notification.title}</p>
          <p className="text-sm">{notification.message}</p>
        </div>
        <button
          onClick={() => onDismiss(notification.id)}
          className="ml-4 text-lg font-semibold"
        >
          ×
        </button>
      </div>
    </div>
  );
}
