import { Currency } from './currency';

export interface FormatCurrencyOptions {
  currency?: string;
  locale?: string;
}

const formatterCache = new Map<string, Intl.NumberFormat>();

function getFormatter(locale: string, currency: string): Intl.NumberFormat {
  const key = `${locale}:${currency}`;
  if (!formatterCache.has(key)) {
    formatterCache.set(
      key,
      new Intl.NumberFormat(locale, {
        style: 'currency',
        currency,
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      })
    );
  }
  return formatterCache.get(key)!;
}

export function formatCurrency(value: number | undefined | null, options: FormatCurrencyOptions = {}): string {
  const { currency = 'EUR', locale = 'es-ES' } = options;
  // Handle undefined/null values
  if (value === undefined || value === null) {
    return getFormatter(locale, currency).format(0);
  }
  // Always use ROUND_HALF_UP (四舍五入) for currency formatting
  const decimal = Currency.round2(value);
  let amount = decimal.toNumber();
  if (Object.is(amount, -0)) {
    amount = 0;
  }

  return getFormatter(locale, currency).format(amount);
}
