/**
 * i18n Service
 * Provides internationalization support for the POS system
 * Loads translations from JSON files with nested structure support
 */

export type Locale = 'zh-CN' | 'es-ES';

// Supported locales
export const SUPPORTED_LOCALES = ['zh-CN', 'es-ES'] as const;
export type SupportedLocale = typeof SUPPORTED_LOCALES[number];

// Default locale
export const DEFAULT_LOCALE: Locale = 'zh-CN';

// Current locale state
let currentLocale: Locale = DEFAULT_LOCALE;

// Flattened translations for quick lookup (loaded from JSON)
const flattenedTranslations: Record<Locale, Record<string, string>> = {
  'zh-CN': {},
  'es-ES': {},
};

// Subscribers for locale changes
const subscribers: Set<(locale: Locale) => void> = new Set();

/**
 * Flatten a nested object into dot-notation keys
 */
function flattenObject(obj: Record<string, unknown>, prefix: string = ''): Record<string, string> {
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

/**
 * Load translations from JSON files
 */
async function loadTranslations(): Promise<void> {
  try {
    const [zhModule, esModule] = await Promise.all([
      import('./locales/zh-CN.json'),
      import('./locales/es-ES.json'),
    ]);

    flattenedTranslations['zh-CN'] = flattenObject(zhModule.default);
    flattenedTranslations['es-ES'] = flattenObject(esModule.default);
  } catch (error) {
    // Cannot use logger here as i18n initializes before logger
    console.error('[i18n] Failed to load translations:', error);
    flattenedTranslations['zh-CN'] = {};
    flattenedTranslations['es-ES'] = {};
  }
}

/**
 * Get current locale
 */
export function getLocale(): Locale {
  return currentLocale;
}

/**
 * Set current locale
 */
export function setLocale(locale: Locale): void {
  if (SUPPORTED_LOCALES.includes(locale as SupportedLocale)) {
    currentLocale = locale;
    localStorage.setItem('pos-locale', locale);
    subscribers.forEach(cb => cb(currentLocale));
  }
}

/**
 * Subscribe to locale changes
 */
export function subscribeLocale(callback: (locale: Locale) => void): () => void {
  subscribers.add(callback);
  return () => subscribers.delete(callback);
}

/**
 * Initialize locale from localStorage or browser settings
 */
export function initLocale(): void {
  const savedLocale = localStorage.getItem('pos-locale') as Locale | null;
  if (savedLocale && SUPPORTED_LOCALES.includes(savedLocale as SupportedLocale)) {
    currentLocale = savedLocale;
    return;
  }

  // Only zh-CN is supported for now
  currentLocale = 'zh-CN';
}

/**
 * Translate a key to the current locale
 * Supports both nested dot-notation (settings.user.form.name) and flat keys
 */
export function t(key: string, params?: Record<string, string | number>): string {
  let result = flattenedTranslations[currentLocale][key];

  // Try with common prefix if not found
  if (result === undefined && !key.startsWith('common.')) {
    result = flattenedTranslations[currentLocale][`common.${key}`];
  }

  // Return the key itself as fallback and warn about missing translation
  if (result === undefined) {
    // Cannot use logger here as i18n initializes before logger
    console.warn(`[i18n] Missing translation for key: "${key}"`);
    result = key;
  }

  // Replace placeholders with params
  if (params) {
    Object.entries(params).forEach(([paramKey, value]) => {
      result = result.replace(new RegExp(`\\{${paramKey}\\}`, 'g'), String(value));
    });
  }

  return result;
}

/**
 * Get all translations for the current locale
 */
export function getAllTranslations(): Record<string, string> {
  return { ...flattenedTranslations[currentLocale] };
}

// i18n 单例对象
export const i18n = {
  getLocale: () => currentLocale,
  setLocale,
  subscribe: subscribeLocale,
  t,
  getAllTranslations,
};

// Initialize translations on import
loadTranslations().then(() => {
  initLocale();
});

