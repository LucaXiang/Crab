import { useMemo } from 'react';
import { Product, ProductSpec } from '@/core/domain/types';
import { useProducts, useProductsLoading, useCategories } from '@/core/stores/resources';
import { useSelectedCategory } from '@/core/stores/ui';

const getDefaultSpec = (p: Product): ProductSpec | undefined =>
  p.specs?.find(s => s.is_default) ?? p.specs?.[0];

const mapProductWithSpec = (p: Product) => ({
  ...p,
  price: getDefaultSpec(p)?.price ?? 0,
});

export function useProductFiltering() {
  const products = useProducts();
  const isProductLoading = useProductsLoading();
  const allCategories = useCategories();
  const selectedCategory = useSelectedCategory();

  const categories = useMemo(
    () => allCategories.filter(c => c.is_display !== false),
    [allCategories],
  );

  const filteredProducts = useMemo(() => {
    // "all" category: show all active products sorted by external_id
    if (selectedCategory === 'all') {
      return [...products]
        .filter((p) => p.is_active)
        .sort((a, b) => {
          const aId = a.external_id ?? Number.MAX_SAFE_INTEGER;
          const bId = b.external_id ?? Number.MAX_SAFE_INTEGER;
          return aId - bId;
        })
        .map(mapProductWithSpec);
    }

    // Find the selected category
    const category = categories.find((c) => c.name === selectedCategory);
    if (!category) {
      return [];
    }

    // Virtual category: filter by tags based on match_mode
    if (category.is_virtual) {
      const tagIds = category.tag_ids || [];
      if (tagIds.length === 0) {
        return [];
      }

      return products
        .filter((p) => {
          if (!p.is_active) return false;
          const productTagIds = (p.tags || []).map((t) => t.id);

          if (category.match_mode === 'all') {
            return tagIds.every((tagId) => productTagIds.includes(tagId));
          } else {
            return tagIds.some((tagId) => productTagIds.includes(tagId));
          }
        })
        .sort((a, b) => a.sort_order - b.sort_order)
        .map(mapProductWithSpec);
    }

    // Regular category: filter by category id
    return products
      .filter((p) => p.is_active && p.category_id === category.id)
      .sort((a, b) => a.sort_order - b.sort_order)
      .map(mapProductWithSpec);
  }, [products, categories, selectedCategory]);

  return {
    filteredProducts,
    isProductLoading,
    categories,
    selectedCategory,
  };
}
