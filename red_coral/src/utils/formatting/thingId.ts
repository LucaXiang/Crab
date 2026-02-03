/**
 * Thing ID Utilities
 *
 * SurrealDB Thing ID format: "table:id" (e.g., "product:abc123", "dining_table:t1")
 * These utilities handle display and extraction of Thing IDs.
 */

/**
 * Extract the raw ID from a Thing ID string.
 * "product:abc123" -> "abc123"
 * "abc123" -> "abc123" (fallback for non-Thing ID)
 */
export function extractThingId(thingId: string | null | undefined): string {
  if (!thingId) return '';
  const colonIndex = thingId.indexOf(':');
  return colonIndex !== -1 ? thingId.slice(colonIndex + 1) : thingId;
}

/**
 * Format a Thing ID for display (truncated).
 * "product:abc123def456" -> "abc123de" (first 8 chars of ID)
 * @param thingId - The full Thing ID
 * @param length - Number of characters to show (default: 8)
 */
export function displayThingId(
  thingId: string | null | undefined,
  length: number = 8
): string {
  const rawId = extractThingId(thingId);
  return rawId.slice(0, length);
}

