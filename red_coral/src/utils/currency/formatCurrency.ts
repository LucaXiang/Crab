import { Currency } from './currency';
import { useStoreInfoStore } from '@/core/stores/settings/useStoreInfoStore';
import { getLocale } from '@/infrastructure/i18n';

export interface FormatCurrencyOptions {
  locale?: string;
  decimalPlaces?: number;
}

const formatterCache = new Map<string, Intl.NumberFormat>();

function getFormatter(locale: string, decimalPlaces: number): Intl.NumberFormat {
  const key = `${locale}:${decimalPlaces}`;
  if (!formatterCache.has(key)) {
    formatterCache.set(
      key,
      new Intl.NumberFormat(locale, {
        style: 'decimal',
        minimumFractionDigits: decimalPlaces,
        maximumFractionDigits: decimalPlaces,
      })
    );
  }
  return formatterCache.get(key)!;
}

export function formatCurrency(value: number | undefined | null, options: FormatCurrencyOptions = {}): string {
  const info = useStoreInfoStore.getState().info;
  const symbol = info.currency_symbol ?? '€';
  const locale = options.locale ?? getLocale();
  const decimalPlaces = options.decimalPlaces ?? info.currency_decimal_places ?? 2;

  if (value === undefined || value === null) {
    return `${getFormatter(locale, decimalPlaces).format(0)}${symbol}`;
  }
  // Always use ROUND_HALF_UP (四舍五入) for currency formatting
  const decimal = Currency.round2(value);
  let amount = decimal.toNumber();
  if (Object.is(amount, -0)) {
    amount = 0;
  }

  return `${getFormatter(locale, decimalPlaces).format(amount)}${symbol}`;
}

/**
 * Get currency symbol from StoreInfo (for UI display, e.g. "€", "$", "¥")
 * Non-hook version for use outside React components.
 */
export function getCurrencySymbol(): string {
  return useStoreInfoStore.getState().info.currency_symbol ?? '€';
}
