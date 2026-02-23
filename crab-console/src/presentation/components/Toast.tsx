import React, { useEffect, useState } from 'react';
import { CheckCircle, XCircle, AlertCircle, X } from 'lucide-react';

const TOAST_AUTO_DISMISS_MS = 3000;
const TOAST_EXIT_ANIMATION_MS = 300;

type ToastType = 'success' | 'error' | 'warning';

interface ToastItem {
  id: string;
  message: string;
  type: ToastType;
}

interface ToastProps {
  item: ToastItem;
  onRemove: (id: string) => void;
}

const Toast: React.FC<ToastProps> = ({ item, onRemove }) => {
  const [isExiting, setIsExiting] = useState(false);

  useEffect(() => {
    const timer = setTimeout(() => {
      setIsExiting(true);
      setTimeout(() => onRemove(item.id), TOAST_EXIT_ANIMATION_MS);
    }, TOAST_AUTO_DISMISS_MS);
    return () => clearTimeout(timer);
  }, [item.id, onRemove]);

  const icons = {
    success: <CheckCircle size={20} className="text-green-500" />,
    error: <XCircle size={20} className="text-red-500" />,
    warning: <AlertCircle size={20} className="text-amber-500" />,
  };

  const bgColors = {
    success: 'bg-green-50 border-green-200',
    error: 'bg-red-50 border-red-200',
    warning: 'bg-amber-50 border-amber-200',
  };

  return (
    <div
      className={`flex items-center gap-3 px-4 py-3 rounded-lg border shadow-lg transition-all duration-300 ${
        bgColors[item.type]
      } ${isExiting ? 'opacity-0 translate-x-4' : 'opacity-100 translate-x-0'}`}
    >
      {icons[item.type]}
      <span className="text-gray-700 text-sm flex-1">{item.message}</span>
      <button
        onClick={() => {
          setIsExiting(true);
          setTimeout(() => onRemove(item.id), TOAST_EXIT_ANIMATION_MS);
        }}
        className="text-gray-400 hover:text-gray-600"
      >
        <X size={16} />
      </button>
    </div>
  );
};

let toastListeners = new Set<(toasts: ToastItem[]) => void>();
let toasts: ToastItem[] = [];
let toastCounter = 0;

const generateToastId = () => `toast-${Date.now()}-${++toastCounter}`;

const notifyListeners = () => {
  toastListeners.forEach(listener => listener([...toasts]));
};

export const toast = {
  success: (message: string) => {
    toasts = [...toasts, { id: generateToastId(), message, type: 'success' }];
    notifyListeners();
  },
  error: (message: string) => {
    toasts = [...toasts, { id: generateToastId(), message, type: 'error' }];
    notifyListeners();
  },
  warning: (message: string) => {
    toasts = [...toasts, { id: generateToastId(), message, type: 'warning' }];
    notifyListeners();
  },
};

export const ToastContainer: React.FC = () => {
  const [items, setItems] = useState<ToastItem[]>([]);

  useEffect(() => {
    setItems([...toasts]);
    const listener = (newToasts: ToastItem[]) => setItems(newToasts);
    toastListeners.add(listener);
    return () => { toastListeners.delete(listener); };
  }, []);

  const handleRemove = (id: string) => {
    toasts = toasts.filter(t => t.id !== id);
    notifyListeners();
  };

  if (items.length === 0) return null;

  return (
    <div className="fixed bottom-4 left-4 right-4 md:left-auto z-[10000] flex flex-col gap-2 max-w-sm">
      {items.map(item => (
        <Toast key={item.id} item={item} onRemove={handleRemove} />
      ))}
    </div>
  );
};
