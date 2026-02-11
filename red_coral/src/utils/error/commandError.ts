import { t } from '@/infrastructure/i18n';
import type { CommandErrorCode } from '@/core/domain/types/orderEvent';

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
