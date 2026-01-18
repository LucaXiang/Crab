import { useState, useCallback } from 'react';

interface ConfirmConfig {
  title: string;
  description: string;
  confirmText?: string;
  cancelText?: string;
  variant?: 'danger' | 'warning' | 'info';
  onConfirm: () => void;
  onCancel?: () => void;
}

export function useConfirm() {
  const [dialogProps, setDialogProps] = useState({
    isOpen: false,
    title: '',
    description: '',
    confirmText: undefined as string | undefined,
    cancelText: undefined as string | undefined,
    variant: 'danger' as 'danger' | 'warning' | 'info',
    onConfirm: () => {},
    onCancel: () => setDialogProps((p) => ({ ...p, isOpen: false }))
  });

  const confirm = useCallback((cfg: ConfirmConfig) => {
    setDialogProps({
      isOpen: true,
      title: cfg.title,
      description: cfg.description,
      confirmText: cfg.confirmText,
      cancelText: cfg.cancelText,
      variant: cfg.variant || 'danger',
      onConfirm: () => {
        setDialogProps((p) => ({ ...p, isOpen: false }));
        cfg.onConfirm();
      },
      onCancel: () => {
        setDialogProps((p) => ({ ...p, isOpen: false }));
        cfg.onCancel && cfg.onCancel();
      }
    });
  }, []);

  return { confirm, dialogProps };
}
