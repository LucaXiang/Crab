import { createTauriClient } from '@/infrastructure/api';
import { useProductStore } from '@/core/stores/resources';
import { useCategoryStore } from './store';
import type { Attribute } from '@/core/domain/types/api';
import type { PrintState } from '@/core/domain/types';

const getApi = () => createTauriClient();

// Helper for attribute binding synchronization
async function syncAttributeBindings(
  categoryId: number,
  selectedAttributeIds: number[],
  attributeDefaultOptions: Record<number, number | number[]>,
  existingBindings: { attributeId: number; id: number }[]
) {
  // Unbind removed attributes - use binding ID, not attribute ID
  const toUnbind = existingBindings.filter(b => !selectedAttributeIds.includes(b.attributeId));
  for (const binding of toUnbind) {
    try {
      await getApi().unbindCategoryAttribute(categoryId, binding.id);
    } catch (error) {
      console.error('Failed to unbind attribute:', binding.attributeId, error);
    }
  }

  // Bind new or updated attributes
  for (let i = 0; i < selectedAttributeIds.length; i++) {
    const attributeId = selectedAttributeIds[i];
    const existingBinding = existingBindings.find(b => b.attributeId === attributeId);

    // Normalize default options to array
    const rawNewDefaults = attributeDefaultOptions?.[attributeId];
    const newDefaultOptionIds = Array.isArray(rawNewDefaults)
      ? rawNewDefaults
      : (rawNewDefaults ? [rawNewDefaults] : []);

    let shouldBind = false;

    if (!existingBinding) {
      // New binding
      shouldBind = true;
    } else {
      // Existing binding, check if default option changed - for simplicity, always rebind
      // This matches the original logic which unbinds and rebinds on any change
      try {
        await getApi().unbindCategoryAttribute(categoryId, existingBinding.id);
      } catch (e) {
        const msg = String(e);
        if (!msg.includes('BINDING_NOT_FOUND')) {
          console.error('Failed to unbind for update:', attributeId, e);
        }
      }
      shouldBind = true;
    }

    if (shouldBind) {
      try {
        await getApi().bindCategoryAttribute({
          category_id: categoryId,
          attribute_id: attributeId,
          is_required: false,
          display_order: i,
          default_option_indices: newDefaultOptionIds?.length
            ? newDefaultOptionIds.map(Number).filter((n: number) => !isNaN(n))
            : undefined
        });
      } catch (error) {
        console.error('Failed to bind attribute:', attributeId, error);
      }
    }
  }
}

export interface CategoryFormData {
  name: string;
  print_destinations?: number[];
  is_kitchen_print_enabled?: PrintState;
  is_label_print_enabled?: PrintState;
  selected_attribute_ids?: number[];
  attribute_default_options?: Record<number, number | number[]>;
  is_virtual?: boolean;
  tag_ids?: number[];
  match_mode?: 'any' | 'all';
  label_print_destinations?: number[];
  sort_order?: number;
}

/**
 * Create a new category
 */
export async function createCategory(formData: CategoryFormData): Promise<number> {
  // PrintState to boolean: 1=true, 0=false (Category API uses boolean)
  const kitchenEnabled = formData.is_kitchen_print_enabled === 0 ? false : true;
  const labelEnabled = formData.is_label_print_enabled === 0 ? false : true;

  const created = await getApi().createCategory({
    name: formData.name.trim(),
    sort_order: formData.sort_order ?? 0,
    kitchen_print_destinations: formData.print_destinations ?? [],
    label_print_destinations: formData.label_print_destinations ?? [],
    is_kitchen_print_enabled: kitchenEnabled,
    is_label_print_enabled: labelEnabled,
    is_virtual: formData.is_virtual ?? false,
    tag_ids: formData.tag_ids ?? [],
    match_mode: formData.match_mode ?? 'any',
  });

  const categoryId = created?.id ?? 0;

  // Handle attribute bindings for new category
  const selectedAttributeIds = formData.selected_attribute_ids || [];
  if (selectedAttributeIds.length > 0 && categoryId) {
    await syncAttributeBindings(
      categoryId,
      selectedAttributeIds,
      formData.attribute_default_options || {},
      [] // No existing bindings for new category
    );
  }

  // Trigger refresh of products store
  useProductStore.getState().fetchAll();

  return categoryId;
}

/**
 * Update an existing category
 */
export async function updateCategory(id: number, formData: CategoryFormData): Promise<void> {
  // PrintState to boolean: 1=true, 0=false (Category API uses boolean)
  const kitchenEnabled = formData.is_kitchen_print_enabled === 0 ? false : true;
  const labelEnabled = formData.is_label_print_enabled === 0 ? false : true;

  await getApi().updateCategory(id, {
    name: formData.name.trim(),
    sort_order: formData.sort_order ?? 0,
    kitchen_print_destinations: formData.print_destinations ?? [],
    label_print_destinations: formData.label_print_destinations ?? [],
    is_kitchen_print_enabled: kitchenEnabled,
    is_label_print_enabled: labelEnabled,
    is_virtual: formData.is_virtual ?? false,
    tag_ids: formData.tag_ids ?? [],
    match_mode: formData.match_mode ?? 'any',
  });

  // Handle attribute bindings
  const selectedAttributeIds = formData.selected_attribute_ids || [];

  // Get existing bindings
  let existingBindings: { attributeId: number; id: number }[] = [];
  try {
    const catAttrs = await getApi().listCategoryAttributes(id);
    // Transform to expected format for syncAttributeBindings
    // API returns Attribute[] - we use attribute id as both attributeId and binding id
    existingBindings = catAttrs.map((ca) => ({
      attributeId: ca.id,
      id: ca.id
    }));
  } catch (error) {
    console.error('Failed to fetch existing category attributes:', error);
  }

  await syncAttributeBindings(
    id,
    selectedAttributeIds,
    formData.attribute_default_options || {},
    existingBindings
  );

  // Trigger refresh of products store
  useProductStore.getState().fetchAll();
}

/**
 * Delete a category
 */
export async function deleteCategory(id: number): Promise<void> {
  await getApi().deleteCategory(id);

  // Refresh products and categories from resources stores
  useProductStore.getState().fetchAll();
  useCategoryStore.getState().fetchAll();
}

/**
 * Load category attributes (for editing)
 */
export async function loadCategoryAttributes(categoryId: number): Promise<{
  attributeIds: number[];
  defaultOptions: Record<number, number[]>;
}> {
  const catAttrs = await getApi().listCategoryAttributes(categoryId);
  // API returns Attribute[] (with id field), not bindings (with attribute_id)
  const safeAttrs: Attribute[] = catAttrs ?? [];
  const attributeIds = safeAttrs.map((ca) => ca.id).filter((id): id is number => id != null);

  // Load default options from category attributes
  const defaultOptions: Record<number, number[]> = {};
  safeAttrs.forEach((ca) => {
    const defaults = ca.default_option_indices ?? [];
    if (defaults.length > 0 && ca.id != null) {
      defaultOptions[ca.id] = defaults;
    }
  });

  return { attributeIds, defaultOptions };
}
