// Tag feature module
// Re-exports all Tag-related components, stores, and mutations

// Components
export { TagManagement } from './TagManagement';
export { TagForm } from './TagForm';
export { TagModal } from './TagModal';

// Store
export {
  useTagStore,
  useTags,
  useTagsLoading,
  useTagById,
} from './store';

// Mutations
export {
  createTag,
  updateTag,
  deleteTag,
  type CreateTagInput,
  type UpdateTagInput,
} from './mutations';
