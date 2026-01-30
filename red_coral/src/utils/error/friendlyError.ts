import { t } from '@/infrastructure/i18n';

/**
 * Convert raw error messages into user-friendly localized messages.
 * Classifies errors by keyword matching and returns appropriate i18n text.
 */
export function friendlyError(raw: string): string {
  const lower = raw.toLowerCase();

  // Network errors
  if (
    lower.includes('connection failed') ||
    lower.includes('tcp') ||
    lower.includes('lookup') ||
    lower.includes('timeout') ||
    lower.includes('nodename') ||
    lower.includes('network')
  ) {
    return t('error.friendly.network');
  }

  // Authentication errors
  if (lower.includes('auth') || lower.includes('credential') || lower.includes('password')) {
    return t('error.friendly.auth');
  }

  // Certificate / TLS errors
  if (lower.includes('certificate') || lower.includes('tls') || lower.includes('ssl')) {
    return t('error.friendly.certificate');
  }

  // Port / bind errors
  if (lower.includes('port') || lower.includes('address already in use') || lower.includes('bind')) {
    return t('error.friendly.port');
  }

  // Activation errors
  if (lower.includes('activation') || lower.includes('not activated')) {
    return t('error.friendly.activation');
  }

  // Fallback
  return t('error.friendly.unknown');
}
