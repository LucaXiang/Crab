import { useEffect, useRef, useState } from 'react';

export function useDirtyForm<T extends Record<string, any>>(initial: T) {
  const initialRef = useRef<T>(initial);
  const [values, setValues] = useState<T>(initial);
  const [isDirty, setIsDirty] = useState(false);

  useEffect(() => {
    initialRef.current = initial;
    setValues(initial);
    setIsDirty(false);
  }, [initial]);

  const handleChange = (name: keyof T, value: T[keyof T]) => {
    setValues((prev) => {
      const next = { ...prev, [name]: value };
      setIsDirty(JSON.stringify(next) !== JSON.stringify(initialRef.current));
      return next;
    });
  };

  const reset = (nextInitial?: T) => {
    if (nextInitial) {
      initialRef.current = nextInitial;
      setValues(nextInitial);
    } else {
      setValues(initialRef.current);
    }
    setIsDirty(false);
  };

  return { values, setValues, handleChange, isDirty, reset };
}
