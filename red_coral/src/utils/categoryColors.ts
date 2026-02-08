// 12 ultra-light pastel backgrounds for category groups
// Stride-7 interleaved order for maximum hue contrast between adjacent indices
// NOTE: Green tones are reserved for comped items (emerald)
export const CATEGORY_BG = [
  '#fef2f2', // Red
  '#eef2ff', // Indigo
  '#fffbeb', // Amber
  '#faf5ff', // Purple
  '#ecfeff', // Cyan
  '#fdf2f8', // Pink
  '#eff6ff', // Blue
  '#fff7ed', // Orange
  '#f5f3ff', // Violet
  '#fefce8', // Yellow
  '#fdf4ff', // Fuchsia
  '#f0f9ff', // Light Blue
];

// Slightly more visible for category headers (same hue order)
export const CATEGORY_HEADER_BG = [
  '#fee2e2', // Red
  '#e0e7ff', // Indigo
  '#fef3c7', // Amber
  '#f3e8ff', // Purple
  '#cffafe', // Cyan
  '#fce7f3', // Pink
  '#dbeafe', // Blue
  '#ffedd5', // Orange
  '#ede9fe', // Violet
  '#fef9c3', // Yellow
  '#fae8ff', // Fuchsia
  '#e0f2fe', // Light Blue
];

// Tailwind 500-level accent colors (for dots, icons, etc.) (same hue order)
export const CATEGORY_ACCENT = [
  '#ef4444', // Red
  '#6366f1', // Indigo
  '#f59e0b', // Amber
  '#a855f7', // Purple
  '#06b6d4', // Cyan
  '#ec4899', // Pink
  '#3b82f6', // Blue
  '#f97316', // Orange
  '#8b5cf6', // Violet
  '#eab308', // Yellow
  '#d946ef', // Fuchsia
  '#0ea5e9', // Light Blue
];

// Build a stable category→color index map based on sort_order position.
// Guarantees unique colors for ≤12 categories; wraps for >12.
export function buildCategoryColorMap(
  categories: Array<{ id: number; sort_order: number }>,
): Map<string, number> {
  const sorted = [...categories].sort((a, b) => a.sort_order - b.sort_order);
  const map = new Map<string, number>();
  sorted.forEach((cat, i) => map.set(String(cat.id), i % CATEGORY_BG.length));
  return map;
}
