import { createContext, useContext } from 'react';
import type { StoreInfo } from '../types/store';

interface StoreInfoContextValue {
  storeInfo: StoreInfo | null;
  currencySymbol: string;
  currencyCode: string;
}

const StoreInfoContext = createContext<StoreInfoContextValue>({
  storeInfo: null,
  currencySymbol: '\u20ac',
  currencyCode: 'EUR',
});

export function useStoreInfo(): StoreInfoContextValue {
  return useContext(StoreInfoContext);
}

export { StoreInfoContext };
