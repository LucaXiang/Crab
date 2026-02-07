import { createTauriClient } from '@/infrastructure/api';
import { useProductStore } from './store';
import type { Product, Category, ProductSpec, PrintState } from '@/core/domain/types';
import { syncAttributeBindings } from '@/screens/Settings/utils';

const getApi = () => createTauriClient();

export interface ProductFormData {
  id?: number;
  name: string;
  receipt_name?: string;
  category_id?: number;
  image?: string;
  tax_rate?: number;
  sort_order?: number;
  kitchen_print_name?: string;
  is_kitchen_print_enabled?: PrintState;
  is_label_print_enabled?: PrintState;
  is_active?: boolean;
  /** 菜品编号 (POS 集成，全局唯一) */
  externalId?: number;
  tags?: number[];
  specs?: ProductSpec[];
  selected_attribute_ids?: number[];
  attribute_default_options?: Record<number, number | number[]>;
}

/**
 * Create a new product
 */
export async function createProduct(
  formData: ProductFormData,
  categories: Category[]
): Promise<{ productId: number | string; success: boolean }> {
  // Get price from root spec
  const rootSpec = formData.specs?.find(s => s.is_root);
  const price = rootSpec?.price ?? 0;

  const productPayload = {
    name: formData.name.trim(),
    category_id: formData.category_id!,
    image: formData.image?.trim() ?? '',
    sort_order: formData.sort_order ?? 0,
    tax_rate: formData.tax_rate ?? 0,
    receipt_name: formData.receipt_name?.trim() ?? undefined,
    kitchen_print_name: formData.kitchen_print_name?.trim() ?? undefined,
    is_kitchen_print_enabled: formData.is_kitchen_print_enabled ?? -1,
    is_label_print_enabled: formData.is_label_print_enabled ?? -1,
    external_id: formData.externalId ?? null,
    tags: formData.tags ?? [],
    specs: [{
      name: formData.name.trim(),
      price: Math.max(0.01, price),
      display_order: 0,
      is_default: true,
      is_active: true,
      is_root: true,
      receipt_name: undefined,
    }],
  };

  const created = await getApi().createProduct(productPayload);
  const productId = created?.id ?? 0;

  // Optimistic update: add to ProductStore
  if (created?.id) {
    useProductStore.getState().optimisticAdd(created as Product & { id: number });
  }

  // Handle attribute bindings
  const selectedAttributeIds = formData.selected_attribute_ids || [];
  for (let i = 0; i < selectedAttributeIds.length; i++) {
    const attributeId = selectedAttributeIds[i];
    const rawDefaults = formData.attribute_default_options?.[attributeId];
    const defaultIndices = Array.isArray(rawDefaults)
      ? rawDefaults
      : (rawDefaults != null ? [rawDefaults] : []);

    try {
      await getApi().bindProductAttribute({
        product_id: Number(productId),
        attribute_id: attributeId,
        is_required: false,
        display_order: i,
        default_option_indices: defaultIndices.length > 0 ? defaultIndices : undefined,
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
      }));

      await getApi().updateProduct(Number(productId), {
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
  id: number | string,
  formData: ProductFormData
): Promise<{ success: boolean }> {
  // Get price from root spec
  const rootSpec = formData.specs?.find(s => s.is_root);
  const price = rootSpec?.price ?? 0;

  const existingSpecs = formData.specs ?? [];
  const updatedSpecs = existingSpecs.length > 0
    ? existingSpecs
    : [{
        name: formData.name.trim(),
        price: Math.max(0.01, price),
        display_order: 0,
        is_default: true,
        is_active: true,
        is_root: true,
        receipt_name: undefined,
      }];

  const updatePayload = {
    name: formData.name.trim(),
    category_id: formData.category_id,
    image: formData.image?.trim() ?? '',
    tax_rate: formData.tax_rate ?? 0,
    sort_order: formData.sort_order ?? 0,
    receipt_name: formData.receipt_name?.trim() ?? undefined,
    kitchen_print_name: formData.kitchen_print_name?.trim() ?? undefined,
    is_kitchen_print_enabled: formData.is_kitchen_print_enabled ?? -1,
    is_label_print_enabled: formData.is_label_print_enabled ?? -1,
    is_active: formData.is_active ?? true,
    external_id: formData.externalId ?? null,
    tags: formData.tags ?? [],
    specs: updatedSpecs,
  };

  const updated = await getApi().updateProduct(Number(id), updatePayload);

  // Update ProductStore cache with API response data
  if (updated?.id) {
    useProductStore.getState().optimisticUpdate(Number(id), () => updated as Product & { id: number });
  }

  // Handle attribute bindings
  const selectedAttributeIds = formData.selected_attribute_ids || [];

  // Get existing bindings (exclude inherited ones — they are managed at category level)
  let existingBindings: { attributeId: string; id: string; defaultOptionIds?: string[] }[] = [];
  try {
    const productAttrs = await getApi().fetchProductAttributes(Number(id));
    existingBindings = (productAttrs ?? [])
      .filter((pa) => !pa.is_inherited)
      .map((pa) => ({
        attributeId: String(pa.attribute.id),
        id: String(pa.id),
        defaultOptionIds: pa.default_option_indices?.map(String) ?? [],
      }));
  } catch (error) {
    console.error('Failed to fetch existing attributes:', error);
  }

  // Convert number IDs/options to string for syncAttributeBindings compatibility
  const strAttrIds = selectedAttributeIds.map(String);
  const strDefaults: Record<string, string | string[]> = {};
  const opts = formData.attribute_default_options;
  if (opts) {
    for (const [k, v] of Object.entries(opts)) {
      strDefaults[k] = Array.isArray(v) ? v.map(String) : String(v);
    }
  }

  // Handle attribute bindings using helper
  await syncAttributeBindings(
    strAttrIds,
    strDefaults,
    existingBindings,
    async (attrId) => getApi().unbindProductAttribute(Number(attrId)),
    async (attrId, defaultOptionIds, index) => {
      const defaultIndices = defaultOptionIds
        .map((id: string) => parseInt(id, 10))
        .filter((n: number) => !isNaN(n));
      await getApi().bindProductAttribute({
        product_id: Number(id),
        attribute_id: Number(attrId),
        is_required: false,
        display_order: index,
        default_option_indices: defaultIndices.length > 0 ? defaultIndices : undefined,
      });
    }
  );

  return { success: true };
}

/**
 * Delete a product
 */
export async function deleteProduct(id: number | string): Promise<{ success: boolean }> {
  await getApi().deleteProduct(Number(id));
  // Optimistic update: remove from ProductStore
  useProductStore.getState().optimisticRemove(Number(id));
  return { success: true };
}

/**
 * Load full product data for editing (from API)
 *
 * list_products 返回的是 Product（不含 attributes），
 * 编辑时需要通过 get_product_full 获取完整的 ProductFull 数据。
 */
export async function loadProductFullData(productId: number | string) {
  const productFull = await getApi().getProductFull(String(productId));
  if (!productFull) {
    throw new Error('Failed to load product full data');
  }

  const attributes = productFull.attributes ?? [];
  const tags = productFull.tags ?? [];
  const specs = productFull.specs ?? [];

  // Extract only direct (non-inherited) attribute bindings as editable selections
  const directAttributes = attributes.filter((binding) => !binding.is_inherited);
  const attributeIds = directAttributes.map((binding) => binding.attribute.id).filter((id): id is number => id != null);

  // Load default options: binding-level override > attribute-level default
  // Only include direct (non-inherited) attributes for editable defaults
  const defaultOptions: Record<number, number[]> = {};
  directAttributes.forEach((binding) => {
    const attrId = binding.attribute.id;
    const indices = binding.default_option_indices ?? binding.attribute.default_option_indices;
    if (attrId && indices && indices.length > 0) {
      defaultOptions[attrId] = indices;
    }
  });

  // Extract tag IDs
  const tagIds = tags.map((tag) => tag.id).filter((id): id is number => id != null);

  // Get price from default spec, externalId from product level
  const defaultSpec = specs.find((s) => s.is_default === true) ?? specs[0];
  const price = defaultSpec?.price ?? 0;
  const externalId = productFull.external_id ?? undefined;

  // Extract inherited attribute IDs for UI (shown as locked/read-only)
  const inheritedAttributes = attributes.filter((binding) => binding.is_inherited);
  const inheritedAttributeIds = inheritedAttributes.map((binding) => binding.attribute.id).filter((id): id is number => id != null);

  return {
    // 必须返回所有 computeIsDirty 跟踪的字段，
    // 确保 setAsyncFormData 完整覆盖 formData 和 formInitialData，
    // 避免 openModal 与 setAsyncFormData 之间的中间状态导致误判 dirty
    name: productFull.name,
    category_id: productFull.category_id,
    image: productFull.image,
    tax_rate: productFull.tax_rate,
    receipt_name: productFull.receipt_name ?? '',
    sort_order: productFull.sort_order,
    is_active: productFull.is_active,
    selected_attribute_ids: attributeIds,
    attribute_default_options: defaultOptions,
    inherited_attribute_ids: inheritedAttributeIds,
    specs,
    tags: tagIds,
    has_multi_spec: specs.length > 1,
    is_kitchen_print_enabled: productFull.is_kitchen_print_enabled,
    is_label_print_enabled: productFull.is_label_print_enabled,
    kitchen_print_name: productFull.kitchen_print_name ?? '',
    price,
    externalId,
  };
}
