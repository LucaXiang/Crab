/**
 * Product Feature Module
 *
 * Provides all product-related functionality including:
 * - Product store (state management)
 * - Product components (Card, Image, Form)
 * - Product management UI
 * - Product mutations (CRUD operations)
 */

// Store
export {
  useProductStore,
  useProducts,
  useProductsLoading,
  useProductById,
  useProductActions,
} from './store';

// Components
export { ProductCard, type ProductWithPrice } from './ProductCard';
export { ProductImage } from './ProductImage';
export { ProductForm } from './ProductForm';
export { ProductManagement } from './ProductManagement';
export { ProductModal } from './ProductModal';

// Mutations
export {
  createProduct,
  updateProduct,
  deleteProduct,
  loadProductFullData,
  type ProductFormData,
} from './mutations';
