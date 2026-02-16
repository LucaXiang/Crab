import { useState, useCallback } from 'react';

interface ConfirmDialogState {
  isOpen: boolean;
  title: string;
  description: string;
  onConfirm: () => void;
}

export function useConfirmDialog() {
  const [state, setState] = useState<ConfirmDialogState>({
    isOpen: false,
    title: '',
    description: '',
    onConfirm: () => {},
  });

  const show = useCallback((title: string, description: string, onConfirm: () => void | Promise<void>) => {
    setState({ isOpen: true, title, description, onConfirm });
  }, []);

  const close = useCallback(() => {
    setState((prev) => ({ ...prev, isOpen: false }));
  }, []);

  return { ...state, show, close };
}
