/**
 * Stores index - Compatibility layer
 *
 * DEPRECATED: Use @/core/stores directly
 *
 * All stores have been migrated to the new architecture:
 * - @/core/stores/resources - Server-authoritative data stores
 * - @/core/stores/order - Order management
 * - @/core/stores/cart - Cart management
 * - @/core/stores/ui - UI state
 * - @/core/stores/auth - Authentication
 * - @/core/stores/settings - Settings UI state
 */

// Re-export everything from core stores for backward compatibility
export * from '@/core/stores';
