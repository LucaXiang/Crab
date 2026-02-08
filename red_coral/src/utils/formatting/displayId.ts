/**
 * ID Display Utilities
 */

/**
 * Format a numeric ID for display.
 */
export function displayId(id: number | string | null | undefined): string {
  if (id == null) return '';
  return String(id);
}
