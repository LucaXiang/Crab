export * from './models';

// API Response types - aligned with Rust server (src-tauri/src/core/response.rs)
export interface ApiResponse<T> {
  /** Error code: 0 = success, >0 = error code from shared::error::ErrorCode */
  code: number | null;
  message: string;
  data?: T;
  details?: Record<string, unknown>;
}

// Auth types
export interface LoginRequest {
  username: string;
  password: string;
}

/**
 * Login response - aligned with shared::client::LoginResponse
 */
export interface LoginResponseData {
  token: string;
  user: CurrentUser;
}

/**
 * Current user info - aligned with shared::client::UserInfo
 */
export interface CurrentUser {
  id: number;
  username: string;
  name: string;
  role_id: number;
  role_name: string;
  permissions: string[];
  is_system: boolean;
  is_active: boolean;
  created_at: number;
}

// Category Attribute binding request
export interface CreateCategoryAttributeRequest {
  category_id: number;
  attribute_id: number;
  is_required?: boolean;
  display_order?: number;
  default_option_ids?: number[];
}

// Product Attribute binding request
export interface CreateProductAttributeRequest {
  product_id: number;
  attribute_id: number;
  is_required?: boolean;
  display_order?: number;
  default_option_ids?: number[];
}
