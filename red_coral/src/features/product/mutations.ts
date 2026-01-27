import { createTauriClient } from '@/infrastructure/api';
import { useProductStore } from './store';
import type { Product, Category, EmbeddedSpec, PrintState } from '@/core/domain/types';
import { syncAttributeBindings } from '@/screens/Settings/utils';

const api = createTauriClient();

export interface ProductFormData {
  id?: string;
  name: string;
  receipt_name?: string;
  category?: string;
  image?: string;
  tax_rate?: number;
  sort_order?: number;
  kitchen_print_name?: string;
  print_destinations?: string[];
  label_print_destinations?: string[];
  is_kitchen_print_enabled?: PrintState;
  is_label_print_enabled?: PrintState;
  is_active?: boolean;
  tags?: string[];
  specs?: EmbeddedSpec[];
  selected_attribute_ids?: string[];
  attribute_default_options?: Record<string, string | string[]>;
}

/**
 * Create a new product
 */
export async function createProduct(
  formData: ProductFormData,
  categories: Category[]
): Promise<{ productId: string; success: boolean }> {
  // Get price and externalId from root spec
  const rootSpec = formData.specs?.find(s => s.is_root);
  const price = rootSpec?.price ?? 0;
  const externalId = rootSpec?.external_id;

  const productPayload = {
    name: formData.name.trim(),
    category: String(formData.category),
    image: formData.image?.trim() ?? '',
    sort_order: formData.sort_order ?? 0,
    tax_rate: formData.tax_rate ?? 0,
    receipt_name: formData.receipt_name?.trim() ?? undefined,
    kitchen_print_name: formData.kitchen_print_name?.trim() ?? undefined,
    kitchen_print_destinations: formData.print_destinations ?? [],
    label_print_destinations: formData.label_print_destinations ?? [],
    is_kitchen_print_enabled: formData.is_kitchen_print_enabled ?? -1,
    is_label_print_enabled: formData.is_label_print_enabled ?? -1,
    tags: formData.tags ?? [],
    specs: [{
      name: formData.name.trim(),
      price: Math.max(0.01, price),
      display_order: 0,
      is_default: true,
      is_active: true,
      is_root: true,
      external_id: externalId ?? null,
      receipt_name: null,
    }],
  };

  const created = await api.createProduct(productPayload);
  const productId = created?.id || '';

  // Optimistic update: add to ProductStore
  if (created?.id) {
    useProductStore.getState().optimisticAdd(created as Product & { id: string });
  }

  // Handle attribute bindings
  const selectedAttributeIds = formData.selected_attribute_ids || [];
  for (let i = 0; i < selectedAttributeIds.length; i++) {
    const attributeId = selectedAttributeIds[i];
    const rawDefaults = formData.attribute_default_options?.[attributeId];
    const defaultOptionIds = Array.isArray(rawDefaults)
      ? rawDefaults
      : (rawDefaults ? [rawDefaults] : []);
    const defaultOptionIdx = defaultOptionIds.length > 0 ? parseInt(defaultOptionIds[0], 10) : undefined;

    try {
      await api.bindProductAttribute({
        product_id: productId,
        attribute_id: attributeId,
        is_required: false,
        display_order: i,
        default_option_idx: !isNaN(defaultOptionIdx as number) ? defaultOptionIdx : undefined,
      });
    } catch (error) {
      console.error('Failed to bind attribute:', attributeId, error);
    }
  }

  // Handle multi-spec for CREATE mode
  if (formData.specs && formData.specs.length > 1) {
    try {
      const embeddedSpecs = formData.specs.map((spec, idx: number) => ({
        name: spec.name,
        price: spec.price,
        display_order: idx,
        is_default: spec.is_default ?? false,
        is_active: true,
        is_root: spec.is_root,
        external_id: spec.external_id ?? null,
      }));

      await api.updateProduct(productId, {
        specs: embeddedSpecs,
      });
    } catch (error) {
      console.error('Failed to update specifications:', error);
      throw error;
    }
  }

  return { productId, success: true };
}

/**
 * Update an existing product
 */
export async function updateProduct(
  id: string,
  formData: ProductFormData
): Promise<{ success: boolean }> {
  // Get price and externalId from root spec
  const rootSpec = formData.specs?.find(s => s.is_root);
  const price = rootSpec?.price ?? 0;
  const externalId = rootSpec?.external_id;

  const existingSpecs = formData.specs ?? [];
  const updatedSpecs = existingSpecs.length > 0
    ? existingSpecs.map(spec => spec.is_root ? {
        ...spec,
        name: spec.name,
        price: spec.price,
        external_id: spec.external_id,
      } : spec)
    : [{
        name: formData.name.trim(),
        price: Math.max(0.01, price),
        display_order: 0,
        is_default: true,
        is_active: true,
        is_root: true,
        external_id: externalId ?? null,
        receipt_name: null,
      }];

  const updatePayload = {
    name: formData.name.trim(),
    category: String(formData.category),
    image: formData.image?.trim() ?? '',
    tax_rate: formData.tax_rate ?? 0,
    sort_order: formData.sort_order ?? 0,
    receipt_name: formData.receipt_name?.trim() ?? undefined,
    kitchen_print_name: formData.kitchen_print_name?.trim() ?? undefined,
    kitchen_print_destinations: formData.print_destinations ?? [],
    label_print_destinations: formData.label_print_destinations ?? [],
    is_kitchen_print_enabled: formData.is_kitchen_print_enabled ?? -1,
    is_label_print_enabled: formData.is_label_print_enabled ?? -1,
    is_active: formData.is_active ?? true,
    tags: formData.tags ?? [],
    specs: updatedSpecs,
  };

  const updated = await api.updateProduct(id, updatePayload);

  // Update ProductStore cache with API response data
  if (updated) {
    useProductStore.getState().optimisticUpdate(id, () => updated as Product);
  }

  // Handle attribute bindings
  const selectedAttributeIds = formData.selected_attribute_ids || [];

  // Get existing bindings
  let existingBindings: { attributeId: string; id: string }[] = [];
  try {
    const productAttrs = await api.fetchProductAttributes(id);
    // API returns relation records with 'to' pointing to attribute
    existingBindings = (productAttrs ?? []).map((pa) => ({
      attributeId: (pa as unknown as { to: string }).to,
      id: pa.id as string
    }));
  } catch (error) {
    console.error('Failed to fetch existing attributes:', error);
  }

  // Handle attribute bindings using helper
  await syncAttributeBindings(
    selectedAttributeIds,
    formData.attribute_default_options || {},
    existingBindings,
    async (attrId) => api.unbindProductAttribute(String(attrId)),
    async (attrId, defaultOptionIds, index) => {
      const defaultOptionIdx = defaultOptionIds.length > 0 ? parseInt(defaultOptionIds[0], 10) : undefined;
      await api.bindProductAttribute({
        product_id: id,
        attribute_id: attrId,
        is_required: false,
        display_order: index,
        default_option_idx: !isNaN(defaultOptionIdx as number) ? defaultOptionIdx : undefined,
      });
    }
  );

  return { success: true };
}

/**
 * Delete a product
 */
export async function deleteProduct(id: string): Promise<{ success: boolean }> {
  await api.deleteProduct(id);
  // Optimistic update: remove from ProductStore
  useProductStore.getState().optimisticRemove(id);
  return { success: true };
}

/**
 * Load full product data for editing (from store)
 */
export function loadProductFullData(productId: string) {
  const productFull = useProductStore.getState().getById(productId);
  if (!productFull) {
    throw new Error('Failed to load product full data');
  }

  // Extract attribute bindings
  const attributeIds = productFull.attributes.map((binding) => binding.attribute.id).filter(Boolean) as string[];

  // Load default options from attributes
  const defaultOptions: Record<string, string[]> = {};
  productFull.attributes.forEach((binding) => {
    const attrId = binding.attribute.id;
    const defaultIdx = binding.attribute.default_option_idx;
    if (attrId && defaultIdx !== null && defaultIdx !== undefined) {
      defaultOptions[attrId] = [String(defaultIdx)];
    }
  });

  // Extract tag IDs
  const tagIds = productFull.tags.map((tag) => tag.id).filter(Boolean) as string[];

  // Get price and externalId from default spec
  const defaultSpec = productFull.specs.find((s) => s.is_default === true) ?? productFull.specs[0];
  const price = defaultSpec?.price ?? 0;
  const externalId = defaultSpec?.external_id ?? undefined;

  return {
    selected_attribute_ids: attributeIds,
    attribute_default_options: defaultOptions,
    specs: productFull.specs,
    tags: tagIds,
    has_multi_spec: productFull.specs.length > 1,
    is_kitchen_print_enabled: productFull.is_kitchen_print_enabled,
    is_label_print_enabled: productFull.is_label_print_enabled,
    print_destinations: productFull.kitchen_print_destinations,
    label_print_destinations: productFull.label_print_destinations,
    kitchen_print_name: productFull.kitchen_print_name,
    price,
    externalId,
  };
}
