import { useEffect, useRef, useState } from 'react';

export function useDirtyForm<T extends Record<string, unknown>>(initial: T) {
  const initialRef = useRef<T>(initial);
  const [values, setValues] = useState<T>(initial);
  const [isDirty, setIsDirty] = useState(false);

  // 使用 JSON 深比较，避免对象引用变化导致表单被无限重置
  const initialJson = JSON.stringify(initial);
  useEffect(() => {
    initialRef.current = initial;
    setValues(initial);
    setIsDirty(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [initialJson]);

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
