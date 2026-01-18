/**
 * Core Domain Types - Backward Compatibility Layer
 *
 * This file re-exports types from the new core/domain structure.
 * New code should import directly from core/domain/types.
 */

// Re-export everything from new location
export * from './core/domain/types';

// Also export from this location for any legacy imports
// Remove after full migration
