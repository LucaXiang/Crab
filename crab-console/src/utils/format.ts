export function formatDate(ms: number): string {
  return new Date(ms).toLocaleDateString();
}

export function formatDateTime(ms: number): string {
  return new Date(ms).toLocaleString();
}

// Formatter cache keyed by locale:currency
const currencyFormatters = new Map<string, Intl.NumberFormat>();

export function formatCurrency(
  amount: number,
  options?: { currency?: string; locale?: string },
): string {
  const currency = options?.currency ?? 'EUR';
  const locale = options?.locale ?? navigator.language ?? 'es-ES';
  const key = `${locale}:${currency}`;
  let fmt = currencyFormatters.get(key);
  if (!fmt) {
    fmt = new Intl.NumberFormat(locale, { style: 'currency', currency });
    currencyFormatters.set(key, fmt);
  }
  return fmt.format(amount);
}

export function timeAgo(ms: number): string {
  const diff = Date.now() - ms;
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return '< 1 min';
  if (minutes < 60) return `${minutes} min`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h`;
  const days = Math.floor(hours / 24);
  return `${days}d`;
}
