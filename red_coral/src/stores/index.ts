/**
 * Stores index
 * Re-export all stores for convenience
 * Now redirects to core architecture
 */

// Re-export from core stores (new architecture)
export * from '@/core/stores';

// Legacy re-exports for backward compatibility
export { useCategoryData } from './useCategoryData';
export { useProducts } from './useProducts';
export { useHeldOrdersStore, useHeldOrdersCount } from './useHeldOrders';
export { useDraftOrdersStore, useDraftOrdersCount } from './useDraftOrders';
