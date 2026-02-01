import { createTauriClient } from '@/infrastructure/api';
import { useProductStore } from '@/core/stores/resources';
import { useCategoryStore } from './store';
import type { Attribute } from '@/core/domain/types/api';
import type { PrintState } from '@/core/domain/types';

const getApi = () => createTauriClient();

// Helper for attribute binding synchronization
async function syncAttributeBindings(
  categoryId: string,
  selectedAttributeIds: string[],
  attributeDefaultOptions: Record<string, string | string[]>,
  existingBindings: { attributeId: string; id: string }[]
) {
  // Unbind removed attributes - use binding ID, not attribute ID
  const toUnbind = existingBindings.filter(b => !selectedAttributeIds.includes(b.attributeId));
  for (const binding of toUnbind) {
    try {
      await getApi().unbindCategoryAttribute(categoryId, String(binding.id));
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
  print_destinations?: string[];
  is_kitchen_print_enabled?: PrintState;
  is_label_print_enabled?: PrintState;
  is_active?: boolean;
  selected_attribute_ids?: string[];
  attribute_default_options?: Record<string, string | string[]>;
  is_virtual?: boolean;
  tag_ids?: string[];
  match_mode?: 'any' | 'all';
  label_print_destinations?: string[];
  sort_order?: number;
}

/**
 * Create a new category
 */
export async function createCategory(formData: CategoryFormData): Promise<string> {
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

  const categoryId = created?.id || '';

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
export async function updateCategory(id: string, formData: CategoryFormData): Promise<void> {
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
    is_active: formData.is_active ?? true,
    is_virtual: formData.is_virtual ?? false,
    tag_ids: formData.tag_ids ?? [],
    match_mode: formData.match_mode ?? 'any',
  });

  // Handle attribute bindings
  const selectedAttributeIds = formData.selected_attribute_ids || [];

  // Get existing bindings
  let existingBindings: { attributeId: string; id: string }[] = [];
  try {
    const catAttrs = await getApi().listCategoryAttributes(id);
    // Transform to expected format for syncAttributeBindings
    // API returns relation records with 'to' pointing to attribute
    existingBindings = catAttrs.map((ca) => ({
      attributeId: (ca as unknown as { out: string }).out,
      id: ca.id as string
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
export async function deleteCategory(id: string): Promise<void> {
  await getApi().deleteCategory(id);

  // Refresh products and categories from resources stores
  useProductStore.getState().fetchAll();
  useCategoryStore.getState().fetchAll();
}

/**
 * Load category attributes (for editing)
 */
export async function loadCategoryAttributes(categoryId: string): Promise<{
  attributeIds: string[];
  defaultOptions: Record<string, string[]>;
}> {
  const catAttrs = await getApi().listCategoryAttributes(categoryId);
  // API returns Attribute[] (with id field), not bindings (with attribute_id)
  const safeAttrs: Attribute[] = catAttrs ?? [];
  const attributeIds = safeAttrs.map((ca) => ca.id).filter(Boolean) as string[];

  // Load default options from category attributes
  const defaultOptions: Record<string, string[]> = {};
  safeAttrs.forEach((ca) => {
    const defaults = ca.default_option_indices?.map(String) ?? [];
    if (defaults.length > 0 && ca.id) {
      defaultOptions[ca.id] = defaults;
    }
  });

  return { attributeIds, defaultOptions };
}
