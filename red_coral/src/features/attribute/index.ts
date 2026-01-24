/**
 * Attribute Feature Module
 *
 * 属性管理功能模块，包含属性和选项的 CRUD 操作。
 */

// Store
export {
  useAttributeStore,
  useAttributes,
  useAttributesLoading,
  useAttributeById,
  useAttributeActions,
  useOptionActions,
  attributeHelpers,
  useAttributeHelpers,
} from './store';

// Components
export { AttributeManagement } from './AttributeManagement';
export { AttributeForm } from './AttributeForm';
export { OptionForm } from './OptionForm';
export { ProductAttributesSection } from './ProductAttributesSection';
export { AttributeSelectionModal } from './AttributeSelectionModal';
