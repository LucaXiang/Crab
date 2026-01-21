import type { Product, ProductFull, EmbeddedSpec, ProductAttributeBinding } from '@/core/domain/types/api';

/** 判断是否单规格产品 */
export const isSingleSpec = (product: Product | ProductFull): boolean =>
  product.specs.filter((s) => s.is_active).length === 1;

/** 获取默认规格 */
export const getDefaultSpec = (product: Product | ProductFull): EmbeddedSpec | undefined =>
  product.specs.find((s) => s.is_default && s.is_active) ??
  product.specs.find((s) => s.is_active);

/** 获取所有激活规格 */
export const getActiveSpecs = (product: Product | ProductFull): EmbeddedSpec[] =>
  product.specs.filter((s) => s.is_active);

/** 判断是否可快速添加 */
export const canQuickAdd = (product: ProductFull): boolean => {
  const hasDefaultSpec =
    product.specs.some((s) => s.is_default && s.is_active) ||
    product.specs.filter((s) => s.is_active).length === 1;

  const requiredAttrs = product.attributes.filter((a) => a.is_required);
  const allAttrsHaveDefault = requiredAttrs.every((a) => a.default_option_idx != null);

  return hasDefaultSpec && allAttrsHaveDefault;
};

/** 获取默认属性选项 */
export const getDefaultAttributeOptions = (
  attributes: ProductAttributeBinding[]
): Array<{ attribute_id: string; option_idx: number }> =>
  attributes
    .filter((a) => a.is_required && a.default_option_idx != null)
    .map((a) => ({
      attribute_id: a.attribute.id!,
      option_idx: a.default_option_idx!,
    }));
