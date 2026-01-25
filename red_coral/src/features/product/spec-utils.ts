/**
 * Specification utility functions
 */

import type { EmbeddedSpec } from '@/core/domain/types';

/**
 * Validate spec data for creation/update
 * @returns error message or null if valid
 */
export function validateSpecData(
  spec: Partial<EmbeddedSpec>,
  isRoot: boolean,
  t: (key: string) => string
): string | null {
  if (!spec.name?.trim()) {
    return t('specification.form.name_required');
  }

  // Non-root specs require external_id and valid price
  if (!isRoot) {
    if (spec.external_id === null || spec.external_id === undefined) {
      return t('settings.external_id_required');
    }

    if (spec.price === undefined || spec.price === null || spec.price < 0) {
      return t('specification.form.price_required');
    }
  }

  return null;
}

/**
 * Set a spec as the default (ensuring only one default at a time)
 */
export function setDefaultSpec(
  specs: EmbeddedSpec[],
  index: number | null
): EmbeddedSpec[] {
  return specs.map((spec, i) => ({
    ...spec,
    is_default: index === null ? false : i === index,
  }));
}

/**
 * Create a new spec with defaults
 */
export function createEmptySpec(): Partial<EmbeddedSpec> {
  return {
    name: '',
    receipt_name: null,
    price: 0,
    display_order: 0,
    is_default: false,
    is_root: false,
    is_active: true,
    external_id: null,
  };
}

/**
 * Check if a spec can be deleted
 */
export function canDeleteSpec(spec: EmbeddedSpec): boolean {
  return !spec.is_root;
}
