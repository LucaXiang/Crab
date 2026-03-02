import { useOutletContext } from 'react-router-dom';

interface StoreContext {
  storeName: string;
}

export function useStoreName(): string {
  const ctx = useOutletContext<StoreContext>();
  return ctx?.storeName ?? '';
}
