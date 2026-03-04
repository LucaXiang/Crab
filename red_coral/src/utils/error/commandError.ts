import { t } from '@/infrastructure/i18n';
import type { CommandErrorCode } from '@/core/domain/types/orderEvent';
import { CommandFailedError } from '@/core/stores/order/commands/sendCommand';
import { getErrorMessage } from '@/utils/error';

/**
 * Get a user-friendly localized message for a CommandErrorCode.
 *
 * Looks up `commandError.<CODE>` in i18n. Falls back to `commandError._fallback`
 * if the code has no translation (e.g. future codes not yet translated).
 */
export function commandErrorMessage(code: CommandErrorCode): string {
  const key = `commandError.${code}`;
  const msg = t(key);
  // t() returns the key itself when no translation is found
  return msg !== key ? msg : t('commandError._fallback');
}

/**
 * Get a user-friendly localized message from any error.
 *
 * - CommandFailedError → commandErrorMessage (i18n by code)
 * - ApiResponse-shaped → getErrorMessage (i18n by numeric code)
 * - Fallback → commandError._fallback
 */
export function localizedErrorMessage(err: unknown): string {
  if (err instanceof CommandFailedError) {
    return commandErrorMessage(err.code);
  }
  if (err && typeof err === 'object' && 'code' in err) {
    return getErrorMessage(err);
  }
  if (err instanceof Error) {
    return getErrorMessage(err.message);
  }
  return t('commandError._fallback');
}
