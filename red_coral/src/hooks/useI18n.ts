import { useCallback, useEffect, useState } from 'react';
import { i18n, t as translate, Locale } from '@/services/i18n';

export function useI18n() {
  const [locale, setLocaleState] = useState<Locale>(i18n.getLocale());

  useEffect(() => {
    const unsubscribe = i18n.subscribe(() => setLocaleState(i18n.getLocale()));
    return unsubscribe;
  }, []);

  const setLocale = (l: Locale) => i18n.setLocale(l);
  const t = useCallback(
    (key: string, params?: Record<string, string | number>): string => translate(key, params),
    []
  );

  return { t, locale, setLocale };
}
