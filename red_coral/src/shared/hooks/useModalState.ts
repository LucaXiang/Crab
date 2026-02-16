import { useState, useCallback } from 'react';

export function useModalState<T>() {
  const [isOpen, setIsOpen] = useState(false);
  const [editing, setEditing] = useState<T | null>(null);

  const open = useCallback((item?: T) => {
    setEditing(item ?? null);
    setIsOpen(true);
  }, []);

  const close = useCallback(() => {
    setIsOpen(false);
    setEditing(null);
  }, []);

  return { isOpen, editing, open, close };
}
