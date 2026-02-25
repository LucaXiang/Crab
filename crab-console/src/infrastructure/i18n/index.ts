export type Locale = 'es' | 'en' | 'zh';

export const SUPPORTED_LOCALES: Locale[] = ['es', 'en', 'zh'];
export const DEFAULT_LOCALE: Locale = 'es';
export const LANG_LABELS: Record<Locale, string> = { es: 'ES', en: 'EN', zh: '中文' };

const STORAGE_KEY = 'redcoral-lang';

let currentLocale: Locale = DEFAULT_LOCALE;
const translations: Record<Locale, Record<string, string>> = { es: {}, en: {}, zh: {} };
const subscribers = new Set<(locale: Locale) => void>();

function flattenObject(obj: Record<string, unknown>, prefix = ''): Record<string, string> {
  const result: Record<string, string> = {};
  for (const key in obj) {
    if (Object.prototype.hasOwnProperty.call(obj, key)) {
      const value = obj[key];
      const newKey = prefix ? `${prefix}.${key}` : key;
      if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
        Object.assign(result, flattenObject(value as Record<string, unknown>, newKey));
      } else if (typeof value === 'string') {
        result[newKey] = value;
      }
    }
  }
  return result;
}

async function loadTranslations(): Promise<void> {
  try {
    const [esModule, enModule, zhModule] = await Promise.all([
      import('./locales/es.json'),
      import('./locales/en.json'),
      import('./locales/zh.json'),
    ]);
    translations.es = flattenObject(esModule.default);
    translations.en = flattenObject(enModule.default);
    translations.zh = flattenObject(zhModule.default);
  } catch (error) {
    console.error('[i18n] Failed to load translations:', error);
  }
}

function detectLocale(): Locale {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored && SUPPORTED_LOCALES.includes(stored as Locale)) return stored as Locale;
  const browserLang = navigator.language.slice(0, 2);
  if (SUPPORTED_LOCALES.includes(browserLang as Locale)) return browserLang as Locale;
  return DEFAULT_LOCALE;
}

export function getLocale(): Locale { return currentLocale; }

export function setLocale(locale: Locale): void {
  if (!SUPPORTED_LOCALES.includes(locale)) return;
  currentLocale = locale;
  localStorage.setItem(STORAGE_KEY, locale);
  document.documentElement.lang = locale;
  subscribers.forEach(cb => cb(currentLocale));
}

export function subscribeLocale(callback: (locale: Locale) => void): () => void {
  subscribers.add(callback);
  return () => subscribers.delete(callback);
}

export function t(key: string, params?: Record<string, string | number>): string {
  let result = translations[currentLocale][key];
  if (result === undefined) result = translations[DEFAULT_LOCALE][key];
  if (result === undefined) {
    console.warn(`[i18n] Missing translation: "${key}"`);
    return key;
  }
  if (params) {
    Object.entries(params).forEach(([k, v]) => {
      result = result.replace(new RegExp(`\\{${k}\\}`, 'g'), String(v));
    });
  }
  return result;
}

export function apiErrorMessage(
  translate: (key: string) => string,
  code: number | null,
  fallback: string,
  httpStatus?: number,
): string {
  if (code !== null) {
    const key = `error.${code}`;
    const msg = translate(key);
    if (msg !== key) return msg;
  }
  if (httpStatus) {
    const key = `httpStatus.${httpStatus}`;
    const msg = translate(key);
    if (msg !== key) return msg;
  }
  return fallback;
}

export const i18n = {
  getLocale,
  setLocale,
  subscribe: subscribeLocale,
  t,
};

// Initialize: load translations, then detect locale and notify subscribers to trigger re-render
loadTranslations().then(() => {
  currentLocale = detectLocale();
  document.documentElement.lang = currentLocale;
  subscribers.forEach(cb => cb(currentLocale));
});
