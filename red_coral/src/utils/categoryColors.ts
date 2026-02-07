// 12 ultra-light pastel backgrounds for category groups
// NOTE: Green tones are reserved for comped items (emerald)
export const CATEGORY_BG = [
  '#fef2f2', '#fff7ed', '#fffbeb', '#fefce8',
  '#ecfeff', '#f0f9ff', '#eff6ff', '#eef2ff',
  '#f5f3ff', '#faf5ff', '#fdf4ff', '#fdf2f8',
];

// Slightly more visible for category headers
export const CATEGORY_HEADER_BG = [
  '#fee2e2', '#ffedd5', '#fef3c7', '#fef9c3',
  '#cffafe', '#e0f2fe', '#dbeafe', '#e0e7ff',
  '#ede9fe', '#f3e8ff', '#fae8ff', '#fce7f3',
];

// Tailwind 500-level accent colors (for dots, icons, etc.)
export const CATEGORY_ACCENT = [
  '#ef4444', '#f97316', '#f59e0b', '#eab308',
  '#06b6d4', '#0ea5e9', '#3b82f6', '#6366f1',
  '#8b5cf6', '#a855f7', '#d946ef', '#ec4899',
];

// Deterministic hash: same category ID always maps to the same color
export function hashToColorIndex(id: string): number {
  let hash = 0;
  for (let i = 0; i < id.length; i++) {
    hash = ((hash << 5) - hash + id.charCodeAt(i)) | 0;
  }
  return Math.abs(hash) % CATEGORY_BG.length;
}
