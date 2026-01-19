/**
 * Tauri API Client - 通过 Tauri Commands 调用 API
 *
 * 替代直接 HTTP 调用，所有请求通过:
 * invoke() → Tauri Command → ClientBridge → CrabClient → EdgeServer
 *
 * 这样可以正确处理 mTLS 认证（自签名证书）
 */

import { invoke } from '@tauri-apps/api/core';
import type {
  ApiResponse,
  LoginResponseData,
  CurrentUserData,
  TagListData,
  Tag,
  CategoryListData,
  Category,
  ProductListData,
  Product,
  ProductSpecification,
  ProductSpecListData,
  Zone,
  ZoneListData,
  Table,
  TableListData,
  KitchenPrinter,
  PrinterListData,
  Attribute,
  AttributeTemplateListData,
  RoleListData,
  Role,
  RolePermissionListData,
  ProductAttribute,
  ProductAttributeListData,
} from '@/core/domain/types/api';

// API Error class (与原 client.ts 保持一致)
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

/**
 * 包装 Tauri invoke 调用，统一错误处理
 */
async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    throw new ApiError('INVOKE_ERROR', message, 500);
  }
}

/**
 * Tauri API Client
 *
 * 与原 ApiClient 接口保持一致，但使用 Tauri commands
 */
export class TauriApiClient {
  // ============ Health ============

  async getHealth() {
    return invokeCommand<{ status: string }>('api_get', { path: '/health' });
  }

  async isAvailable(): Promise<boolean> {
    try {
      await this.getHealth();
      return true;
    } catch {
      return false;
    }
  }

  // ============ Auth ============

  async login(data: { username: string; password: string }): Promise<ApiResponse<LoginResponseData>> {
    return invokeCommand<ApiResponse<LoginResponseData>>('login_employee', {
      username: data.username,
      password: data.password
    });
  }

  async logout(): Promise<void> {
    return invokeCommand<void>('logout_employee');
  }

  async getCurrentUser(): Promise<ApiResponse<CurrentUserData>> {
    return invokeCommand<ApiResponse<CurrentUserData>>('get_current_session');
  }

  // ============ Tags ============

  async listTags(): Promise<ApiResponse<TagListData>> {
    return invokeCommand<ApiResponse<TagListData>>('list_tags');
  }

  async getTag(id: string): Promise<ApiResponse<{ tag: Tag }>> {
    return invokeCommand<ApiResponse<{ tag: Tag }>>('get_tag', { id });
  }

  async createTag(data: { name: string; color?: string; display_order?: number }): Promise<ApiResponse<{ tag: Tag }>> {
    return invokeCommand<ApiResponse<{ tag: Tag }>>('create_tag', { data });
  }

  async updateTag(id: string, data: { name?: string; color?: string; display_order?: number; is_active?: boolean }): Promise<ApiResponse<{ tag: Tag }>> {
    return invokeCommand<ApiResponse<{ tag: Tag }>>('update_tag', { id, data });
  }

  async deleteTag(id: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_tag', { id });
  }

  // ============ Categories ============

  async listCategories(): Promise<ApiResponse<CategoryListData>> {
    return invokeCommand<ApiResponse<CategoryListData>>('list_categories');
  }

  async getCategory(id: string): Promise<ApiResponse<{ category: Category }>> {
    return invokeCommand<ApiResponse<{ category: Category }>>('get_category', { id });
  }

  async createCategory(data: Record<string, unknown>): Promise<ApiResponse<{ category: Category }>> {
    return invokeCommand<ApiResponse<{ category: Category }>>('create_category', { data });
  }

  async updateCategory(id: string, data: { name?: string; sort_order?: number; is_active?: boolean }): Promise<ApiResponse<{ category: Category }>> {
    return invokeCommand<ApiResponse<{ category: Category }>>('update_category', { id, data });
  }

  async deleteCategory(id: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_category', { id });
  }

  async batchUpdateCategorySortOrder(updates: { id: string; sort_order: number }[]): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('batch_update_category_sort_order', { updates });
  }

  // ============ Products ============

  async listProducts(): Promise<ApiResponse<ProductListData>> {
    return invokeCommand<ApiResponse<ProductListData>>('list_products');
  }

  async getProduct(id: string): Promise<ApiResponse<{ product: Product; specifications?: ProductSpecification[]; attributes?: ProductAttribute[] }>> {
    return invokeCommand<ApiResponse<{ product: Product; specifications?: ProductSpecification[]; attributes?: ProductAttribute[] }>>('get_product', { id });
  }

  async createProduct(data: Record<string, unknown>): Promise<ApiResponse<{ product: Product }>> {
    return invokeCommand<ApiResponse<{ product: Product }>>('create_product', { data });
  }

  async updateProduct(id: string, data: Record<string, unknown>): Promise<ApiResponse<{ product: Product }>> {
    return invokeCommand<ApiResponse<{ product: Product }>>('update_product', { id, data });
  }

  async deleteProduct(id: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_product', { id });
  }

  async bulkDeleteProducts(ids: number[]): Promise<ApiResponse<{ deleted: boolean }>> {
    // Delete products one by one (bulk delete may not be implemented in backend)
    for (const id of ids) {
      await this.deleteProduct(String(id));
    }
    return { data: { deleted: true } } as ApiResponse<{ deleted: boolean }>;
  }

  // ============ Product Specifications ============

  async listProductSpecs(productId: string | number): Promise<ApiResponse<ProductSpecListData>> {
    return invokeCommand<ApiResponse<ProductSpecListData>>('list_specs', { product_id: String(productId) });
  }

  async createProductSpec(productId: string | number, data: Record<string, unknown>): Promise<ApiResponse<{ spec: ProductSpecification }>> {
    return invokeCommand<ApiResponse<{ spec: ProductSpecification }>>('create_spec', { product_id: String(productId), data });
  }

  async updateProductSpec(productId: string | number, specId: string | number, data: Record<string, unknown>): Promise<ApiResponse<{ spec: ProductSpecification }>> {
    return invokeCommand<ApiResponse<{ spec: ProductSpecification }>>('update_spec', { id: String(specId), data });
  }

  async deleteProductSpec(productId: string | number, specId: string | number): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_spec', { id: String(specId) });
  }

  // ============ Product Attributes ============

  async fetchProductAttributes(productId: string): Promise<ApiResponse<ProductAttributeListData>> {
    return invokeCommand<ApiResponse<ProductAttributeListData>>('list_product_attributes', { product_id: productId });
  }

  async bindProductAttribute(data: Record<string, unknown>): Promise<ApiResponse<{ binding: ProductAttribute }>> {
    return invokeCommand<ApiResponse<{ binding: ProductAttribute }>>('bind_product_attribute', { data });
  }

  async unbindProductAttribute(id: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('unbind_product_attribute', { id });
  }

  // ============ Category Attributes ============

  async listCategoryAttributes(categoryId?: string | number): Promise<ApiResponse<{ category_attributes: unknown[] }>> {
    return invokeCommand<ApiResponse<{ category_attributes: unknown[] }>>('list_category_attributes', { category_id: categoryId ? String(categoryId) : undefined });
  }

  async bindCategoryAttribute(data: Record<string, unknown>): Promise<ApiResponse<{ binding: unknown }>> {
    return invokeCommand<ApiResponse<{ binding: unknown }>>('bind_category_attribute', { data });
  }

  async unbindCategoryAttribute(categoryId: string, attributeId: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('unbind_category_attribute', { category_id: categoryId, attribute_id: attributeId });
  }

  // ============ Attributes ============

  async listAttributeTemplates(): Promise<ApiResponse<AttributeTemplateListData>> {
    return invokeCommand<ApiResponse<AttributeTemplateListData>>('list_attributes');
  }

  async getAttributeTemplate(id: string): Promise<ApiResponse<{ template: Attribute }>> {
    return invokeCommand<ApiResponse<{ template: Attribute }>>('get_attribute', { id });
  }

  async createAttributeTemplate(data: Record<string, unknown>): Promise<ApiResponse<{ template: Attribute }>> {
    return invokeCommand<ApiResponse<{ template: Attribute }>>('create_attribute', { data });
  }

  async updateAttributeTemplate(id: string, data: Record<string, unknown>): Promise<ApiResponse<{ template: Attribute }>> {
    return invokeCommand<ApiResponse<{ template: Attribute }>>('update_attribute', { id, data });
  }

  async deleteAttributeTemplate(id: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_attribute', { id });
  }

  // ============ Zones ============

  async listZones(): Promise<ApiResponse<ZoneListData>> {
    return invokeCommand<ApiResponse<ZoneListData>>('list_zones');
  }

  async getZone(id: string): Promise<ApiResponse<{ zone: Zone }>> {
    return invokeCommand<ApiResponse<{ zone: Zone }>>('get_zone', { id });
  }

  async createZone(data: { name: string; description?: string }): Promise<ApiResponse<{ zone: Zone }>> {
    return invokeCommand<ApiResponse<{ zone: Zone }>>('create_zone', { data });
  }

  async updateZone(id: string | number, data: { name?: string; description?: string; is_active?: boolean }): Promise<ApiResponse<{ zone: Zone }>> {
    return invokeCommand<ApiResponse<{ zone: Zone }>>('update_zone', { id: String(id), data });
  }

  async deleteZone(id: string | number): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_zone', { id: String(id) });
  }

  // ============ Tables ============

  async listTables(): Promise<ApiResponse<TableListData>> {
    return invokeCommand<ApiResponse<TableListData>>('list_tables');
  }

  async getTablesByZone(zoneId: string): Promise<ApiResponse<TableListData>> {
    return invokeCommand<ApiResponse<TableListData>>('list_tables_by_zone', { zone_id: zoneId });
  }

  async getTable(id: string): Promise<ApiResponse<{ table: Table }>> {
    return invokeCommand<ApiResponse<{ table: Table }>>('get_table', { id });
  }

  async createTable(data: { name: string; zone: string; capacity?: number }): Promise<ApiResponse<{ table: Table }>> {
    return invokeCommand<ApiResponse<{ table: Table }>>('create_table', { data });
  }

  async updateTable(id: string | number, data: { name?: string; zone?: string; capacity?: number; is_active?: boolean }): Promise<ApiResponse<{ table: Table }>> {
    return invokeCommand<ApiResponse<{ table: Table }>>('update_table', { id: String(id), data });
  }

  async deleteTable(id: string | number): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_table', { id: String(id) });
  }

  // ============ Kitchen Printers ============

  async listPrinters(): Promise<ApiResponse<PrinterListData>> {
    return invokeCommand<ApiResponse<PrinterListData>>('list_kitchen_printers');
  }

  async getPrinter(id: string): Promise<ApiResponse<{ printer: KitchenPrinter }>> {
    return invokeCommand<ApiResponse<{ printer: KitchenPrinter }>>('get_kitchen_printer', { id });
  }

  async createPrinter(data: { name: string; printer_name?: string; description?: string }): Promise<ApiResponse<{ printer: KitchenPrinter }>> {
    return invokeCommand<ApiResponse<{ printer: KitchenPrinter }>>('create_kitchen_printer', { data });
  }

  async updatePrinter(id: string, data: { name?: string; printer_name?: string; description?: string; is_active?: boolean }): Promise<ApiResponse<{ printer: KitchenPrinter }>> {
    return invokeCommand<ApiResponse<{ printer: KitchenPrinter }>>('update_kitchen_printer', { id, data });
  }

  async deletePrinter(id: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_kitchen_printer', { id });
  }

  // ============ Employees ============

  async listEmployees(): Promise<ApiResponse<{ employees: unknown[] }>> {
    return invokeCommand<ApiResponse<{ employees: unknown[] }>>('list_employees');
  }

  async getEmployee(id: string): Promise<ApiResponse<{ employee: unknown }>> {
    return invokeCommand<ApiResponse<{ employee: unknown }>>('get_employee', { id });
  }

  async createEmployee(data: { username: string; password: string; role: string }): Promise<ApiResponse<{ employee: unknown }>> {
    return invokeCommand<ApiResponse<{ employee: unknown }>>('create_employee', { data });
  }

  async updateEmployee(id: string, data: { username?: string; password?: string; role?: string; is_active?: boolean }): Promise<ApiResponse<{ employee: unknown }>> {
    return invokeCommand<ApiResponse<{ employee: unknown }>>('update_employee', { id, data });
  }

  async deleteEmployee(id: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_employee', { id });
  }

  // ============ Price Rules ============

  async listPriceAdjustments(): Promise<ApiResponse<{ rules: unknown[] }>> {
    return invokeCommand<ApiResponse<{ rules: unknown[] }>>('list_price_rules');
  }

  async listActivePriceAdjustments(): Promise<ApiResponse<{ rules: unknown[] }>> {
    return invokeCommand<ApiResponse<{ rules: unknown[] }>>('list_active_price_rules');
  }

  async getPriceAdjustment(id: string): Promise<ApiResponse<{ rule: unknown }>> {
    return invokeCommand<ApiResponse<{ rule: unknown }>>('get_price_rule', { id });
  }

  async createPriceAdjustment(data: Record<string, unknown>): Promise<ApiResponse<{ rule: unknown }>> {
    return invokeCommand<ApiResponse<{ rule: unknown }>>('create_price_rule', { data });
  }

  async updatePriceAdjustment(id: string, data: Record<string, unknown>): Promise<ApiResponse<{ rule: unknown }>> {
    return invokeCommand<ApiResponse<{ rule: unknown }>>('update_price_rule', { id, data });
  }

  async deletePriceAdjustment(id: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_price_rule', { id });
  }

  // ============ Roles ============

  async listRoles(): Promise<ApiResponse<RoleListData>> {
    return invokeCommand<ApiResponse<RoleListData>>('list_roles');
  }

  async getRole(id: string): Promise<ApiResponse<{ role: Role }>> {
    return invokeCommand<ApiResponse<{ role: Role }>>('get_role', { id });
  }

  async createRole(data: { name: string }): Promise<ApiResponse<{ role: Role }>> {
    return invokeCommand<ApiResponse<{ role: Role }>>('create_role', { data });
  }

  async updateRole(id: string, data: { name?: string }): Promise<ApiResponse<{ role: Role }>> {
    return invokeCommand<ApiResponse<{ role: Role }>>('update_role', { id, data });
  }

  async deleteRole(id: string): Promise<ApiResponse<{ deleted: boolean }>> {
    return invokeCommand<ApiResponse<{ deleted: boolean }>>('delete_role', { id });
  }

  async getRolePermissions(roleId: string): Promise<ApiResponse<RolePermissionListData>> {
    return invokeCommand<ApiResponse<RolePermissionListData>>('get_role_permissions', { role_id: roleId });
  }

  // ============ Orders ============

  async listOrders() {
    return invokeCommand('list_orders');
  }

  async listOpenOrders() {
    return invokeCommand('list_open_orders');
  }

  async getOrder(id: string) {
    return invokeCommand('get_order', { id });
  }

  async getOrderByReceipt(receiptNumber: string) {
    return invokeCommand('get_order_by_receipt', { receipt_number: receiptNumber });
  }

  async createOrder(data: Record<string, unknown>) {
    return invokeCommand('create_order', { data });
  }

  async addOrderItem(orderId: string, item: Record<string, unknown>) {
    return invokeCommand('add_order_item', { order_id: orderId, item });
  }

  async addOrderPayment(orderId: string, payment: Record<string, unknown>) {
    return invokeCommand('add_order_payment', { order_id: orderId, payment });
  }

  // ============ System ============

  async getSystemState() {
    return invokeCommand('get_system_state');
  }

  // ============ Generic API (fallback) ============

  async apiGet<T>(path: string): Promise<T> {
    return invokeCommand('api_get', { path });
  }

  async apiPost<T>(path: string, body: unknown): Promise<T> {
    return invokeCommand('api_post', { path, body });
  }

  async apiPut<T>(path: string, body: unknown): Promise<T> {
    return invokeCommand('api_put', { path, body });
  }

  async apiDelete<T>(path: string): Promise<T> {
    return invokeCommand('api_delete', { path });
  }

  // ============ Token Management ============
  // Note: In Tauri mode, authentication is handled by ClientBridge
  // These methods are provided for API compatibility but don't manage local tokens

  /**
   * Set access token (no-op in Tauri mode - auth is handled by ClientBridge)
   */
  setAccessToken(_token: string): void {
    // In Tauri mode, the ClientBridge manages authentication state
    // This is a no-op for API compatibility
  }

  /**
   * Clear access token (no-op in Tauri mode - auth is handled by ClientBridge)
   */
  clearAccessToken(): void {
    // In Tauri mode, the ClientBridge manages authentication state
    // This is a no-op for API compatibility
  }

  /**
   * Refresh token - delegates to Tauri command
   */
  async refreshToken(): Promise<{ data?: { access_token: string } }> {
    try {
      const result = await invokeCommand<{ access_token: string }>('refresh_token', {});
      return { data: result };
    } catch {
      return { data: undefined };
    }
  }
}

// 创建单例
let clientInstance: TauriApiClient | null = null;

export function createTauriClient(): TauriApiClient {
  if (!clientInstance) {
    clientInstance = new TauriApiClient();
  }
  return clientInstance;
}

// 默认导出
export default TauriApiClient;
