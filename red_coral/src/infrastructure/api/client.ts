import type {
  LoginRequest,
  LoginResponseData,
  RegisterRequest,
  ChangePasswordRequest,
  CurrentUserData,
  SuccessData,
  Product,
  ProductQuery,
  ProductListData,
  CreateProductRequest,
  UpdateProductRequest,
  Category,
  CategoryData,
  CategoryListData,
  CreateCategoryRequest,
  UpdateCategoryRequest,
  PriceAdjustmentData,
  PriceAdjustmentListData,
  CreatePriceAdjustmentRequest,
  UpdatePriceAdjustmentRequest,
  AttributeTemplateData,
  AttributeTemplateListData,
  CreateAttributeTemplateRequest,
  UpdateAttributeTemplateRequest,
  AttributeOptionData,
  AttributeOptionListData,
  CreateAttributeOptionRequest,
  UpdateAttributeOptionRequest,
  TagData,
  TagListData,
  CreateTagRequest,
  UpdateTagRequest,
  RoleData,
  RoleListData,
  CreateRoleRequest,
  UpdateRoleRequest,
  RolePermissionListData,
  CreateRolePermissionsRequest,
  PrinterData,
  PrinterListData,
  CreatePrinterRequest,
  UpdatePrinterRequest,
  ZoneData,
  ZoneListData,
  CreateZoneRequest,
  UpdateZoneRequest,
  TableData,
  TableListData,
  CreateTableRequest,
  UpdateTableRequest,
  CategoryAttributeData,
  CategoryAttributeListData,
  CreateCategoryAttributeRequest,
  UpdateCategoryAttributeRequest,
  SpecificationTagListData,
  CreateSpecificationTagsRequest,
  ProductSpecification,
  ProductSpecListData,
  CreateSpecificationRequest,
  UpdateSpecificationRequest,
  HealthData,
  ReadinessData,
  LivenessData,
  DeleteResponse,
  ApiResponse,
  ProductAttributeData,
  ProductAttributeListData,
  CreateProductAttributeRequest,
  UpdateProductAttributeRequest,
  ImportResult,
  ExportResponse,
} from '@/core/domain/types/api';

// API Error class
export class ApiError extends Error {
  code: string;
  httpStatus: number;

  constructor(code: string, message: string, httpStatus: number = 500) {
    super(message);
    this.code = code;
    this.httpStatus = httpStatus;
    this.name = 'ApiError';
  }
}

// HTTP client configuration
interface ClientConfig {
  baseUrl: string;
  accessToken?: string;
}

// Default configuration
const defaultConfig: ClientConfig = {
  baseUrl: import.meta.env.VITE_API_BASE_URL || 'http://localhost:9625',
};

// Auth token storage key
const AUTH_TOKEN_KEY = 'auth-token';

// Get token from localStorage (shared between auth store and API client)
function getStoredToken(): string | undefined {
  try {
    return localStorage.getItem(AUTH_TOKEN_KEY) || undefined;
  } catch {
    return undefined;
  }
}

// Set token in localStorage
export function setAuthToken(token: string): void {
  try {
    localStorage.setItem(AUTH_TOKEN_KEY, token);
  } catch (e) {
    console.error('Failed to store auth token:', e);
  }
}

// Clear token from localStorage
export function clearAuthToken(): void {
  try {
    localStorage.removeItem(AUTH_TOKEN_KEY);
  } catch (e) {
    console.error('Failed to clear auth token:', e);
  }
}

// Main API Client class
export class ApiClient {
  private config: ClientConfig;

  constructor(config: Partial<ClientConfig> = {}) {
    this.config = { ...defaultConfig, ...config };
  }

  setAccessToken(token: string): void {
    this.config.accessToken = token;
    setAuthToken(token);
  }

  clearAccessToken(): void {
    this.config.accessToken = undefined;
    clearAuthToken();
  }

  async request<T>(
    method: string,
    endpoint: string,
    options: {
      body?: unknown;
      query?: Record<string, string | number | boolean | undefined>;
    } = {}
  ): Promise<T> {
    let url = `${this.config.baseUrl}${endpoint}`;

    // Add query parameters
    if (options.query) {
      const params = new URLSearchParams();
      Object.entries(options.query).forEach(([key, value]) => {
        if (value !== undefined) {
          params.append(key, String(value));
        }
      });
      const queryString = params.toString();
      if (queryString) {
        url += `?${queryString}`;
      }
    }

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    // Get token from instance config first, then from localStorage
    const token = this.config.accessToken || getStoredToken();
    if (token) {
      headers['Authorization'] = `Bearer ${token}`;
    }

    // Filter out null/undefined values from body
    let bodyData: unknown = options.body;
    if (options.body && typeof options.body === 'object') {
      bodyData = Object.fromEntries(
        Object.entries(options.body as Record<string, unknown>)
          .filter(([_, v]) => v !== null && v !== undefined)
      );
    }

    const response = await fetch(url, {
      method,
      headers,
      body: bodyData ? JSON.stringify(bodyData) : undefined,
    });

    // Handle empty responses (e.g., 204 No Content)
    const text = await response.text();
    if (!text) {
      if (!response.ok) {
        throw new ApiError('E0001', `HTTP ${response.status}: Empty error response`, response.status);
      }
      return {} as T;
    }

    // Try to parse as JSON
    let json: unknown;
    try {
      json = JSON.parse(text);
    } catch {
      // Non-JSON response (e.g., HTML error page)
      if (!response.ok) {
        throw new ApiError('E0001', `HTTP ${response.status}: ${text.substring(0, 200)}`, response.status);
      }
      throw new ApiError('E0001', 'Invalid JSON response', response.status);
    }

    if (!response.ok) {
      // Server returns { error_code: string, message: string, data: null } for errors
      const errorData = json as { error_code?: string | null; message?: string };
      const errorCode = errorData.error_code || 'E0001';
      const errorMessage = errorData.message || 'Unknown error';
      throw new ApiError(errorCode, errorMessage, response.status);
    }

    return json as T;
  }

  // Health endpoints
  async getHealth(): Promise<ApiResponse<HealthData>> {
    return this.request('GET', '/health');
  }

  async getReadiness(): Promise<ApiResponse<ReadinessData>> {
    return this.request('GET', '/health/ready');
  }

  async getLiveness(): Promise<ApiResponse<LivenessData>> {
    return this.request('GET', '/health/live');
  }

  // Auth endpoints
  async login(data: LoginRequest): Promise<ApiResponse<LoginResponseData>> {
    return this.request('POST', '/api/auth/login', { body: data });
  }

  async register(data: RegisterRequest): Promise<ApiResponse<LoginResponseData>> {
    return this.request('POST', '/api/auth/register', { body: data });
  }

  async getCurrentUser(): Promise<ApiResponse<CurrentUserData>> {
    return this.request('GET', '/api/auth/me');
  }

  async refreshToken(): Promise<ApiResponse<LoginResponseData>> {
    return this.request('POST', '/api/auth/refresh');
  }

  async changePassword(data: ChangePasswordRequest): Promise<ApiResponse<SuccessData>> {
    return this.request('POST', '/api/auth/change-password', { body: data });
  }

  // Product endpoints
  async listProducts(query?: ProductQuery): Promise<ApiResponse<ProductListData>> {
    return this.request('GET', '/api/products', { query: query as Record<string, string | number | boolean | undefined> });
  }

  async getProduct(id: number): Promise<ApiResponse<ProductListData['products'][0]>> {
    return this.request('GET', `/api/products/${id}`);
  }

  async createProduct(data: CreateProductRequest): Promise<ApiResponse<{ product: Product }>> {
    return this.request('POST', '/api/products', { body: data });
  }

  async updateProduct(id: string | number, data: UpdateProductRequest): Promise<ApiResponse<ProductListData['products'][0]>> {
    return this.request('PUT', `/api/products/${id}`, { body: data });
  }

  async deleteProduct(id: string | number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/products/${id}`);
  }

  async bulkDeleteProducts(ids: number[]): Promise<ApiResponse<DeleteResponse>> {
    return this.request('POST', '/api/products/bulk-delete', { body: { ids } });
  }

  // Product Spec endpoints
  async listProductSpecs(productId: string | number): Promise<ApiResponse<ProductSpecListData>> {
    return this.request('GET', `/api/products/${productId}/specs`);
  }

  async createProductSpec(productId: string | number, data: CreateSpecificationRequest): Promise<ApiResponse<ProductSpecification>> {
    return this.request('POST', `/api/products/${productId}/specs`, { body: data });
  }

  async updateProductSpec(productId: string | number, specId: string | number, data: UpdateSpecificationRequest): Promise<ApiResponse<ProductSpecification>> {
    return this.request('PUT', `/api/products/${productId}/specs/${specId}`, { body: data });
  }

  async deleteProductSpec(productId: string | number, specId: string | number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/products/${productId}/specs/${specId}`);
  }

  // Category endpoints
  async listCategories(): Promise<ApiResponse<CategoryListData>> {
    return this.request('GET', '/api/categories');
  }

  async getCategory(id: string | number): Promise<ApiResponse<CategoryData>> {
    return this.request('GET', `/api/categories/${id}`);
  }

  async createCategory(data: CreateCategoryRequest): Promise<ApiResponse<{ category: Category }>> {
    return this.request('POST', '/api/categories', { body: data });
  }

  async updateCategory(id: string | number, data: UpdateCategoryRequest): Promise<ApiResponse<CategoryData>> {
    return this.request('PUT', `/api/categories/${id}`, { body: data });
  }

  async deleteCategory(id: string | number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/categories/${id}`);
  }

  async bulkDeleteCategories(ids: number[]): Promise<ApiResponse<DeleteResponse>> {
    return this.request('POST', '/api/categories/bulk-delete', { body: { ids } });
  }

  async batchUpdateCategorySortOrder(updates: { id: string | number; sort_order: number }[]): Promise<ApiResponse<DeleteResponse>> {
    return this.request('POST', '/api/categories/batch-sort-order', { body: { updates } });
  }

  async getProductsByCategory(categoryId: number): Promise<ApiResponse<ProductListData>> {
    return this.request('GET', `/api/categories/${categoryId}/products`);
  }

  // Pricing endpoints
  async listPriceAdjustments(): Promise<ApiResponse<PriceAdjustmentListData>> {
    return this.request('GET', '/api/pricing/rules');
  }

  async listActivePriceAdjustments(): Promise<ApiResponse<PriceAdjustmentListData>> {
    return this.request('GET', '/api/pricing/rules/active');
  }

  async getPriceAdjustment(id: number): Promise<ApiResponse<PriceAdjustmentData>> {
    return this.request('GET', `/api/pricing/rules/${id}`);
  }

  async createPriceAdjustment(data: CreatePriceAdjustmentRequest): Promise<ApiResponse<PriceAdjustmentData>> {
    return this.request('POST', '/api/pricing/rules', { body: data });
  }

  async updatePriceAdjustment(id: number, data: UpdatePriceAdjustmentRequest): Promise<ApiResponse<PriceAdjustmentData>> {
    return this.request('PUT', `/api/pricing/rules/${id}`, { body: data });
  }

  async deletePriceAdjustment(id: number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/pricing/rules/${id}`);
  }

  // Attribute Template endpoints
  async listAttributeTemplates(): Promise<ApiResponse<AttributeTemplateListData>> {
    return this.request('GET', '/api/attributes');
  }

  async getAttributeTemplate(id: number): Promise<ApiResponse<AttributeTemplateData>> {
    return this.request('GET', `/api/attributes/${id}`);
  }

  async createAttributeTemplate(data: CreateAttributeTemplateRequest): Promise<ApiResponse<AttributeTemplateData>> {
    return this.request('POST', '/api/attributes', { body: data });
  }

  async updateAttributeTemplate(id: number, data: UpdateAttributeTemplateRequest): Promise<ApiResponse<AttributeTemplateData>> {
    return this.request('PUT', `/api/attributes/${id}`, { body: data });
  }

  async deleteAttributeTemplate(id: number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/attributes/${id}`);
  }

  async listAttributeTemplateOptions(templateId: number): Promise<ApiResponse<AttributeOptionListData>> {
    return this.request('GET', `/api/attributes/${templateId}/options`);
  }

  // Attribute Option endpoints (独立路由)
  async listAttributeOptions(): Promise<ApiResponse<AttributeOptionListData>> {
    return this.request('GET', '/api/attribute-options');
  }

  async getAttributeOption(id: number): Promise<ApiResponse<AttributeOptionData>> {
    return this.request('GET', `/api/attribute-options/${id}`);
  }

  async createAttributeOption(data: CreateAttributeOptionRequest): Promise<ApiResponse<AttributeOptionData>> {
    return this.request('POST', '/api/attribute-options', { body: data });
  }

  async updateAttributeOption(id: number, data: UpdateAttributeOptionRequest): Promise<ApiResponse<AttributeOptionData>> {
    return this.request('PUT', `/api/attribute-options/${id}`, { body: data });
  }

  async deleteAttributeOption(id: number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/attribute-options/${id}`);
  }

  // Tags endpoints
  async listTags(): Promise<ApiResponse<TagListData>> {
    return this.request('GET', '/api/tags');
  }

  async getTag(id: number): Promise<ApiResponse<TagData>> {
    return this.request('GET', `/api/tags/${id}`);
  }

  async createTag(data: CreateTagRequest): Promise<ApiResponse<TagData>> {
    return this.request('POST', '/api/tags', { body: data });
  }

  async updateTag(id: number, data: UpdateTagRequest): Promise<ApiResponse<TagData>> {
    return this.request('PUT', `/api/tags/${id}`, { body: data });
  }

  async deleteTag(id: number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/tags/${id}`);
  }

  // Roles endpoints
  async listRoles(): Promise<ApiResponse<RoleListData>> {
    return this.request('GET', '/api/roles');
  }

  async getRole(id: number): Promise<ApiResponse<RoleData>> {
    return this.request('GET', `/api/roles/${id}`);
  }

  async createRole(data: CreateRoleRequest): Promise<ApiResponse<RoleData>> {
    return this.request('POST', '/api/roles', { body: data });
  }

  async updateRole(id: number, data: UpdateRoleRequest): Promise<ApiResponse<RoleData>> {
    return this.request('PUT', `/api/roles/${id}`, { body: data });
  }

  async deleteRole(id: number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/roles/${id}`);
  }

  async getRolePermissions(roleId: string | number): Promise<ApiResponse<RolePermissionListData>> {
    return this.request('GET', `/api/roles/${roleId}/permissions`);
  }

  async createRolePermissions(data: CreateRolePermissionsRequest): Promise<ApiResponse<RolePermissionListData>> {
    return this.request('POST', '/api/roles/permissions', { body: data });
  }

  async deleteRolePermission(roleId: number, permission: string): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/roles/${roleId}/permissions/${permission}`);
  }

  // Kitchen endpoints
  async listPrinters(): Promise<ApiResponse<PrinterListData>> {
    return this.request('GET', '/api/kitchen/printers');
  }

  async getPrinter(id: number): Promise<ApiResponse<PrinterData>> {
    return this.request('GET', `/api/kitchen/printers/${id}`);
  }

  async createPrinter(data: CreatePrinterRequest): Promise<ApiResponse<PrinterData>> {
    return this.request('POST', '/api/kitchen/printers', { body: data });
  }

  async updatePrinter(id: number, data: UpdatePrinterRequest): Promise<ApiResponse<PrinterData>> {
    return this.request('PUT', `/api/kitchen/printers/${id}`, { body: data });
  }

  async deletePrinter(id: number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/kitchen/printers/${id}`);
  }

  // Cashier endpoints (Zones)
  async listZones(): Promise<ApiResponse<ZoneListData>> {
    return this.request('GET', '/api/zones');
  }

  async getZone(id: string | number): Promise<ApiResponse<ZoneData>> {
    return this.request('GET', `/api/zones/${id}`);
  }

  async createZone(data: CreateZoneRequest): Promise<ApiResponse<ZoneData>> {
    return this.request('POST', '/api/zones', { body: data });
  }

  async updateZone(id: string | number, data: UpdateZoneRequest): Promise<ApiResponse<ZoneData>> {
    return this.request('PUT', `/api/zones/${id}`, { body: data });
  }

  async deleteZone(id: string | number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/zones/${id}`);
  }

  // Tables
  async listTables(): Promise<ApiResponse<TableListData>> {
    return this.request('GET', '/api/tables');
  }

  async getTable(id: string | number): Promise<ApiResponse<TableData>> {
    return this.request('GET', `/api/tables/${id}`);
  }

  async getTablesByZone(zoneId: string | number): Promise<ApiResponse<TableListData>> {
    return this.request('GET', `/api/zones/${zoneId}/tables`);
  }

  async createTable(data: CreateTableRequest): Promise<ApiResponse<TableData>> {
    return this.request('POST', '/api/tables', { body: data });
  }

  async updateTable(id: string | number, data: UpdateTableRequest): Promise<ApiResponse<TableData>> {
    return this.request('PUT', `/api/tables/${id}`, { body: data });
  }

  async deleteTable(id: string | number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/tables/${id}`);
  }

  // Associations endpoints
  async listCategoryAttributes(categoryId?: string | number): Promise<ApiResponse<CategoryAttributeListData>> {
    const query = categoryId ? { category_id: categoryId } : undefined;
    return this.request('GET', '/api/associations/category-attributes', { query });
  }

  async createCategoryAttribute(data: CreateCategoryAttributeRequest): Promise<ApiResponse<CategoryAttributeData>> {
    return this.request('POST', '/api/associations/category-attributes', { body: data });
  }

  async updateCategoryAttribute(id: number, data: UpdateCategoryAttributeRequest): Promise<ApiResponse<CategoryAttributeData>> {
    return this.request('PUT', `/api/associations/category-attributes/${id}`, { body: data });
  }

  async deleteCategoryAttribute(id: number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/associations/category-attributes/${id}`);
  }

  // Alias methods for category attributes (compatible with TauriApiClient naming)
  async bindCategoryAttribute(data: CreateCategoryAttributeRequest): Promise<ApiResponse<CategoryAttributeData>> {
    return this.createCategoryAttribute(data);
  }

  async unbindCategoryAttribute(categoryId: string | number, attributeId: string | number): Promise<ApiResponse<DeleteResponse>> {
    // Need to find the binding ID first, or use a different approach
    // For now, use the DELETE endpoint with category_id and attribute_id
    return this.request('DELETE', `/api/associations/category-attributes/by-pair?category_id=${categoryId}&attribute_id=${attributeId}`);
  }

  // Product Attribute endpoints
  async listProductAttributes(productId: string | number): Promise<ApiResponse<ProductAttributeListData>> {
    return this.request('GET', `/api/products/${productId}/attributes`);
  }

  async bindProductAttribute(data: { product_id: string | number; attribute_id: string | number; is_required?: boolean; display_order?: number; default_option_id?: string }): Promise<ApiResponse<ProductAttributeData>> {
    return this.request('POST', '/api/associations/product-attributes', { body: data });
  }

  async unbindProductAttribute(id: string | number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/associations/product-attributes/${id}`);
  }

  async updateProductAttribute(id: number, data: { is_required?: boolean; display_order?: number; default_option_id?: string }): Promise<ApiResponse<ProductAttributeData>> {
    return this.request('PUT', `/api/associations/product-attributes/${id}`, { body: data });
  }

  async listSpecificationTags(specificationId?: number): Promise<ApiResponse<SpecificationTagListData>> {
    const query = specificationId ? { specification_id: specificationId } : undefined;
    return this.request('GET', '/api/associations/specification-tags', { query });
  }

  async createSpecificationTags(data: CreateSpecificationTagsRequest): Promise<ApiResponse<SpecificationTagListData>> {
    return this.request('POST', '/api/associations/specification-tags', { body: data });
  }

  async getSpecificationTags(specificationId: number): Promise<ApiResponse<SpecificationTagListData>> {
    return this.request('GET', `/api/specifications/${specificationId}/tags`);
  }

  async deleteSpecificationTag(specificationId: number, tagId: number): Promise<ApiResponse<DeleteResponse>> {
    return this.request('DELETE', `/api/associations/specification-tags/${specificationId}/${tagId}`);
  }

  // Legacy compatibility methods
  async isAvailable(): Promise<boolean> {
    try {
      await this.getHealth();
      return true;
    } catch {
      return false;
    }
  }

  async getProductAttributes(productId: number): Promise<ApiResponse<{ attributes: unknown[] }>> {
    return this.request('GET', `/api/products/${productId}/attributes`);
  }

  async fetchProductAttributes(productId: string | number): Promise<ApiResponse<ProductAttributeListData>> {
    return this.listProductAttributes(Number(productId));
  }

  // Data Import/Export endpoints
  async importData(data: { data_type: string; format: string; data: Record<string, unknown>; options?: { update_existing?: boolean; skip_errors?: boolean } }): Promise<ApiResponse<ImportResult>> {
    return this.request('POST', '/api/data/import', { body: data });
  }

  async exportData(query: { data_type: string; format: string; include_deleted?: boolean }): Promise<ApiResponse<ExportResponse>> {
    return this.request('POST', '/api/data/export', { body: query });
  }
}

// Factory function to create client
export function createClient(config?: Partial<ClientConfig>): ApiClient {
  return new ApiClient(config);
}
