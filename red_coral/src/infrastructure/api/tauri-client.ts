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
  Tag,
  TagCreate,
  TagUpdate,
  TagListData,
  CategoryListData,
  Category,
  CategoryCreate,
  CategoryUpdate,
  ProductListData,
  Product,
  ProductCreate,
  ProductUpdate,
  ProductFull,
  Zone,
  ZoneListData,
  Table,
  TableListData,
  PrintDestination,
  PrintDestinationCreate,
  PrintDestinationUpdate,
  Attribute,
  AttributeCreate,
  AttributeUpdate,
  AttributeTemplateListData,
  RoleListData,
  RolePermissionListData,
  ProductAttribute,
  ProductAttributeListData,
  Employee,
  PriceRule,
  PriceRuleCreate,
  PriceRuleUpdate,
  CreateProductAttributeRequest,
  CreateCategoryAttributeRequest,
} from '@/core/domain/types/api';

// API Error class - aligned with shared::error::ErrorCode (u16)
export class ApiError extends Error {
  code: number;
  details?: Record<string, unknown>;

  constructor(code: number, message: string, details?: Record<string, unknown>) {
    super(message);
    this.code = code;
    this.details = details;
    this.name = 'ApiError';
  }

  /** Check if this is a specific error code */
  is(errorCode: number): boolean {
    return this.code === errorCode;
  }
}

/**
 * 调用 Tauri command 并自动解包 ApiResponse
 * 返回 data 字段，错误时抛出 ApiError
 */
export async function invokeApi<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    const response = await invoke<ApiResponse<T>>(command, args);
    if (response.code && response.code > 0) {
      throw new ApiError(response.code, response.message, response.details ?? undefined);
    }
    return response.data as T;
  } catch (error) {
    if (error instanceof ApiError) throw error;
    const message = error instanceof Error ? error.message : String(error);
    throw new ApiError(9001, message); // 9001 = InternalError
  }
}

// Alias for internal use
const invokeAndUnwrap = invokeApi;

/**
 * Tauri API Client
 *
 * 与原 ApiClient 接口保持一致，但使用 Tauri commands
 */
export class TauriApiClient {
  // ============ Health ============

  async isAvailable(): Promise<boolean> {
    try {
      await invoke('health_check');
      return true;
    } catch {
      return false;
    }
  }

  // ============ Auth ============

  async login(data: { username: string; password: string }): Promise<LoginResponseData> {
    return invokeAndUnwrap<LoginResponseData>('login_employee', {
      username: data.username,
      password: data.password,
    });
  }

  async logout(): Promise<void> {
    await invokeAndUnwrap<void>('logout_employee');
  }

  // ============ Tags ============

  async listTags(): Promise<Tag[]> {
    const data = await invokeAndUnwrap<TagListData>('list_tags');
    return data.tags;
  }

  async getTag(id: string): Promise<Tag> {
    const data = await invokeAndUnwrap<{ tag: Tag }>('get_tag', { id });
    return data.tag;
  }

  async createTag(data: TagCreate): Promise<Tag> {
    const result = await invokeAndUnwrap<{ tag: Tag }>('create_tag', { data });
    return result.tag;
  }

  async updateTag(id: string, data: TagUpdate): Promise<Tag> {
    const result = await invokeAndUnwrap<{ tag: Tag }>('update_tag', { id, data });
    return result.tag;
  }

  async deleteTag(id: string): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('delete_tag', { id });
  }

  // ============ Categories ============

  async listCategories(): Promise<Category[]> {
    const data = await invokeAndUnwrap<CategoryListData>('list_categories');
    return data.categories;
  }

  async createCategory(data: CategoryCreate): Promise<Category> {
    const result = await invokeAndUnwrap<{ category: Category }>('create_category', { data });
    return result.category;
  }

  async updateCategory(id: string, data: CategoryUpdate): Promise<Category> {
    const result = await invokeAndUnwrap<{ category: Category }>('update_category', { id, data });
    return result.category;
  }

  async deleteCategory(id: string): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('delete_category', { id });
  }

  async batchUpdateCategorySortOrder(updates: { id: string; sort_order: number }[]): Promise<void> {
    await invokeAndUnwrap<{ updated: boolean }>('batch_update_category_sort_order', { updates });
  }

  // ============ Products ============

  async listProducts(): Promise<Product[]> {
    const data = await invokeAndUnwrap<ProductListData>('list_products');
    return data.products;
  }

  async getProductFull(id: string): Promise<ProductFull> {
    const data = await invokeAndUnwrap<{ product: ProductFull }>('get_product_full', { id });
    return data.product;
  }

  async createProduct(data: ProductCreate): Promise<Product> {
    const result = await invokeAndUnwrap<{ product: Product }>('create_product', { data });
    return result.product;
  }

  async updateProduct(id: string, data: ProductUpdate): Promise<Product> {
    const result = await invokeAndUnwrap<{ product: Product }>('update_product', { id, data });
    return result.product;
  }

  async deleteProduct(id: string): Promise<void> {
    await invokeAndUnwrap<void>('delete_product', { id });
  }

  async bulkDeleteProducts(ids: (string | number)[]): Promise<void> {
    for (const id of ids) {
      await this.deleteProduct(String(id));
    }
  }

  async batchUpdateProductSortOrder(updates: { id: string; sort_order: number }[]): Promise<void> {
    await invokeAndUnwrap<{ updated: boolean }>('batch_update_product_sort_order', { updates });
  }

  // ============ Product Attributes ============

  async fetchProductAttributes(productId: string): Promise<ProductAttribute[]> {
    const data = await invokeAndUnwrap<ProductAttributeListData>('list_product_attributes', { product_id: productId });
    return data.product_attributes;
  }

  async bindProductAttribute(data: CreateProductAttributeRequest): Promise<ProductAttribute> {
    const result = await invokeAndUnwrap<{ binding: ProductAttribute }>('bind_product_attribute', { data });
    return result.binding;
  }

  async unbindProductAttribute(id: string): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('unbind_product_attribute', { id });
  }

  // ============ Category Attributes ============

  async listCategoryAttributes(categoryId: string | number): Promise<Attribute[]> {
    const data = await invokeAndUnwrap<{ templates: Attribute[] }>('list_category_attributes', { category_id: String(categoryId) });
    return data.templates;
  }

  async bindCategoryAttribute(data: CreateCategoryAttributeRequest): Promise<unknown> {
    const result = await invokeAndUnwrap<{ binding: unknown }>('bind_category_attribute', { data });
    return result.binding;
  }

  async unbindCategoryAttribute(categoryId: string, attributeId: string): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('unbind_category_attribute', { category_id: categoryId, attribute_id: attributeId });
  }

  // ============ Attributes ============

  async listAttributeTemplates(): Promise<Attribute[]> {
    const data = await invokeAndUnwrap<AttributeTemplateListData>('list_attributes');
    return data.templates;
  }

  async getAttributeTemplate(id: string): Promise<Attribute> {
    const data = await invokeAndUnwrap<{ template: Attribute }>('get_attribute', { id });
    return data.template;
  }

  async createAttribute(data: AttributeCreate): Promise<Attribute> {
    const result = await invokeAndUnwrap<{ attribute: Attribute }>('create_attribute', { data });
    return result.attribute;
  }

  async updateAttribute(id: string, data: AttributeUpdate): Promise<Attribute> {
    const result = await invokeAndUnwrap<{ attribute: Attribute }>('update_attribute', { id, data });
    return result.attribute;
  }

  async deleteAttribute(id: string): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('delete_attribute', { id });
  }

  // ============ Attribute Options ============

  async addAttributeOption(attributeId: string, data: { name: string; value_code?: string; price_modifier?: number; is_default?: boolean; display_order?: number; is_active?: boolean; receipt_name?: string; kitchen_print_name?: string }): Promise<Attribute> {
    const result = await invokeAndUnwrap<{ template: Attribute }>('add_attribute_option', { attribute_id: attributeId, data });
    return result.template;
  }

  async updateAttributeOption(attributeId: string, index: number, data: { name?: string; value_code?: string; price_modifier?: number; is_default?: boolean; display_order?: number; is_active?: boolean; receipt_name?: string; kitchen_print_name?: string }): Promise<Attribute> {
    const result = await invokeAndUnwrap<{ template: Attribute }>('update_attribute_option', { attribute_id: attributeId, index, data });
    return result.template;
  }

  async deleteAttributeOption(attributeId: string, index: number): Promise<Attribute> {
    const result = await invokeAndUnwrap<{ template: Attribute }>('delete_attribute_option', { attribute_id: attributeId, index });
    return result.template;
  }

  // ============ Zones ============

  async listZones(): Promise<Zone[]> {
    const data = await invokeAndUnwrap<ZoneListData>('list_zones');
    return data.zones;
  }

  async createZone(data: { name: string; description?: string }): Promise<Zone> {
    const result = await invokeAndUnwrap<{ zone: Zone }>('create_zone', { data });
    return result.zone;
  }

  async updateZone(id: string | number, data: { name?: string; description?: string; is_active?: boolean }): Promise<Zone> {
    const result = await invokeAndUnwrap<{ zone: Zone }>('update_zone', { id: String(id), data });
    return result.zone;
  }

  async deleteZone(id: string | number): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('delete_zone', { id: String(id) });
  }

  // ============ Tables ============

  async listTables(): Promise<Table[]> {
    const data = await invokeAndUnwrap<TableListData>('list_tables');
    return data.tables;
  }

  async createTable(data: { name: string; zone: string; capacity?: number }): Promise<Table> {
    const result = await invokeAndUnwrap<{ table: Table }>('create_table', { data });
    return result.table;
  }

  async updateTable(id: string | number, data: { name?: string; zone?: string; capacity?: number; is_active?: boolean }): Promise<Table> {
    const result = await invokeAndUnwrap<{ table: Table }>('update_table', { id: String(id), data });
    return result.table;
  }

  async deleteTable(id: string | number): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('delete_table', { id: String(id) });
  }

  // ============ Print Destinations ============

  async listPrintDestinations(): Promise<PrintDestination[]> {
    const data = await invokeAndUnwrap<{ print_destinations: PrintDestination[] }>('list_print_destinations');
    return data.print_destinations ?? [];
  }

  async createPrintDestination(data: PrintDestinationCreate): Promise<PrintDestination> {
    const result = await invokeAndUnwrap<{ destination: PrintDestination }>('create_print_destination', { data });
    return result.destination;
  }

  async updatePrintDestination(id: string, data: PrintDestinationUpdate): Promise<PrintDestination> {
    const result = await invokeAndUnwrap<{ destination: PrintDestination }>('update_print_destination', { id, data });
    return result.destination;
  }

  async deletePrintDestination(id: string): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('delete_print_destination', { id });
  }

  // ============ Employees ============

  async listEmployees(): Promise<Employee[]> {
    const data = await invokeAndUnwrap<{ employees: Employee[] }>('list_employees');
    return data.employees;
  }

  async createEmployee(data: { username: string; password: string; role: string }): Promise<Employee> {
    const result = await invokeAndUnwrap<{ employee: Employee }>('create_employee', { data });
    return result.employee;
  }

  async updateEmployee(id: string, data: { password?: string; role?: string; is_active?: boolean }): Promise<Employee> {
    const result = await invokeAndUnwrap<{ employee: Employee }>('update_employee', { id, data });
    return result.employee;
  }

  async deleteEmployee(id: string): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('delete_employee', { id });
  }

  // ============ Price Rules ============

  async listPriceRules(): Promise<PriceRule[]> {
    const data = await invokeAndUnwrap<{ rules: PriceRule[] }>('list_price_rules');
    return data.rules;
  }

  async getPriceRule(id: string): Promise<PriceRule> {
    return invokeAndUnwrap<PriceRule>('get_price_rule', { id });
  }

  async createPriceRule(data: PriceRuleCreate): Promise<PriceRule> {
    return invokeAndUnwrap<PriceRule>('create_price_rule', { data });
  }

  async updatePriceRule(id: string, data: PriceRuleUpdate): Promise<PriceRule> {
    return invokeAndUnwrap<PriceRule>('update_price_rule', { id, data });
  }

  async deletePriceRule(id: string): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('delete_price_rule', { id });
  }

  // ============ Roles ============

  async listRoles(): Promise<RoleListData> {
    return invokeAndUnwrap<RoleListData>('list_roles');
  }

  async getRolePermissions(roleId: string): Promise<RolePermissionListData> {
    return invokeAndUnwrap<RolePermissionListData>('get_role_permissions', { role_id: roleId });
  }

  // ============ Token Management ============
  // In Tauri mode, authentication is handled by ClientBridge on Rust side

  async refreshToken(): Promise<void> {
    await invokeAndUnwrap<void>('refresh_token');
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
