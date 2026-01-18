/**
 * Tags API
 */

import { invoke } from '@tauri-apps/api/core';
import type { Tag, CreateTagParams, UpdateTagParams } from '@/core/domain/types';

/**
 * Fetch all tags
 */
export async function fetchTags(): Promise<Tag[]> {
  return invoke('fetch_tags');
}

/**
 * Get a single tag
 */
export async function getTag(id: string): Promise<Tag> {
  return invoke('get_tag', { id });
}

/**
 * Create a new tag
 */
export async function createTag(params: CreateTagParams): Promise<Tag> {
  return invoke('create_tag', { ...params } as Record<string, unknown>);
}

/**
 * Update a tag
 */
export async function updateTag(params: UpdateTagParams): Promise<Tag> {
  return invoke('update_tag', { ...params } as Record<string, unknown>);
}

/**
 * Delete a tag
 */
export async function deleteTag(id: string): Promise<void> {
  return invoke('delete_tag', { id });
}

/**
 * Get tags for a product
 */
export async function getProductTags(productId: string): Promise<Tag[]> {
  return invoke('get_product_tags', { productId });
}

/**
 * Set tags for a product
 */
export async function setProductTags(productId: string, tagIds: string[]): Promise<void> {
  return invoke('set_product_tags', { productId, tagIds });
}

/**
 * Get tags for a category
 */
export async function getCategoryTags(categoryId: string): Promise<Tag[]> {
  return invoke('get_category_tags', { categoryId });
}

/**
 * Set tags for a category
 */
export async function setCategoryTags(categoryId: string, tagIds: string[]): Promise<void> {
  return invoke('set_category_tags', { categoryId, tagIds });
}

/**
 * Get tags for a specification
 */
export async function getSpecificationTags(specificationId: string): Promise<Tag[]> {
  return invoke('get_specification_tags', { specificationId });
}

/**
 * Set tags for a specification
 */
export async function setSpecificationTags(specificationId: string, tagIds: string[]): Promise<void> {
  return invoke('set_specification_tags', { specificationId, tagIds });
}
