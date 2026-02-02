/**
 * Tauri API Client - 通过 Tauri Commands 调用 API
 *
 * 替代直接 HTTP 调用，所有请求通过:
 * invoke() → Tauri Command → ClientBridge → CrabClient → EdgeServer
 *
 * 这样可以正确处理 mTLS 认证（自签名证书）
 */

import { invoke } from '@tauri-apps/api/core';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useBridgeStore } from '@/core/stores/bridge';
import { t } from '@/infrastructure/i18n';
import { friendlyError } from '@/utils/error/friendlyError';
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
  AttributeListData,
  RoleListData,
  RolePermissionListData,
  AttributeBindingFull,
  AttributeBinding,
  Employee,
  PriceRule,
  PriceRuleCreate,
  PriceRuleUpdate,
  CreateProductAttributeRequest,
  CreateCategoryAttributeRequest,
  StoreInfo,
  StoreInfoUpdate,
  LabelTemplate,
  LabelTemplateCreate,
  LabelTemplateUpdate,
  Shift,
  ShiftCreate,
  ShiftClose,
  ShiftForceClose,
  ShiftUpdate,
  DailyReport,
  DailyReportGenerate,
  AuditListResponse,
  SystemIssue,
  ResolveSystemIssueRequest,
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

// Auth error codes that indicate the session is invalid
const AUTH_ERROR_CODES = [1001, 1003, 1005]; // NotAuthenticated, TokenExpired, SessionExpired
let isHandlingAuthError = false;

function handleAuthError(code: number) {
  if (!AUTH_ERROR_CODES.includes(code)) return;
  if (isHandlingAuthError) return;
  isHandlingAuthError = true;
  try {
    console.warn(`[invokeApi] Auth error ${code}, clearing auth state`);
    useAuthStore.getState().logout();
    useBridgeStore.setState({ currentSession: null });
  } finally {
    isHandlingAuthError = false;
  }
}

/** 根据错误码 + 原始消息生成用户友好的错误提示 */
function localizeErrorCode(code: number, rawMessage?: string): string {
  // 优先用错误码查 i18n
  const key = `errors.${code}`;
  const localized = t(key);
  if (localized !== key) {
    return localized;
  }
  // i18n 没有对应 key，用 rawMessage 做关键词匹配作为 fallback
  console.warn(`[invokeApi] Missing i18n for error code ${code}, add to zh-CN.json`);
  if (rawMessage) {
    return friendlyError(rawMessage);
  }
  return `${t('error.friendly.unknown')} (${code})`;
}

export async function invokeApi<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    const response = await invoke<ApiResponse<T>>(command, args);
    if (response.code && response.code > 0) {
      handleAuthError(response.code);
      throw new ApiError(response.code, localizeErrorCode(response.code, response.message), response.details ?? undefined);
    }
    return response.data as T;
  } catch (error) {
    if (error instanceof ApiError) {
      handleAuthError(error.code);
      throw error;
    }
    const raw = error instanceof Error ? error.message : String(error);
    throw new ApiError(9001, friendlyError(raw));
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

  async listProducts(): Promise<ProductFull[]> {
    const data = await invokeAndUnwrap<{ products: ProductFull[] }>('list_products');
    return data.products;
  }

  async getProductFull(id: string): Promise<ProductFull> {
    const result = await invokeAndUnwrap<{ product: ProductFull }>('get_product_full', { id });
    return result.product;
  }

  async createProduct(data: ProductCreate): Promise<ProductFull> {
    const result = await invokeAndUnwrap<{ product: ProductFull }>('create_product', { data });
    return result.product;
  }

  async updateProduct(id: string, data: ProductUpdate): Promise<ProductFull> {
    const result = await invokeAndUnwrap<{ product: ProductFull }>('update_product', { id, data });
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

  async fetchProductAttributes(productId: string): Promise<AttributeBindingFull[]> {
    return await invokeAndUnwrap<AttributeBindingFull[]>('list_product_attributes', { productId });
  }

  async bindProductAttribute(data: CreateProductAttributeRequest): Promise<AttributeBinding> {
    const result = await invokeAndUnwrap<{ binding: AttributeBinding }>('bind_product_attribute', { data });
    return result.binding;
  }

  async unbindProductAttribute(id: string): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('unbind_product_attribute', { id });
  }

  // ============ Category Attributes ============

  async listCategoryAttributes(categoryId: string | number): Promise<Attribute[]> {
    const data = await invokeAndUnwrap<{ templates: Attribute[] }>('list_category_attributes', { categoryId: String(categoryId) });
    return data.templates;
  }

  async bindCategoryAttribute(data: CreateCategoryAttributeRequest): Promise<unknown> {
    const result = await invokeAndUnwrap<{ binding: unknown }>('bind_category_attribute', { data });
    return result.binding;
  }

  async unbindCategoryAttribute(categoryId: string, attributeId: string): Promise<void> {
    await invokeAndUnwrap<{ deleted: boolean }>('unbind_category_attribute', { categoryId, attributeId });
  }

  // ============ Attributes ============

  async listAttributes(): Promise<Attribute[]> {
    const data = await invokeAndUnwrap<AttributeListData>('list_attributes');
    return data.templates;
  }

  async getAttribute(id: string): Promise<Attribute> {
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
    const result = await invokeAndUnwrap<{ template: Attribute }>('add_attribute_option', { attributeId, data });
    return result.template;
  }

  async updateAttributeOption(attributeId: string, index: number, data: { name?: string; value_code?: string; price_modifier?: number; is_default?: boolean; display_order?: number; is_active?: boolean; receipt_name?: string; kitchen_print_name?: string }): Promise<Attribute> {
    const result = await invokeAndUnwrap<{ template: Attribute }>('update_attribute_option', { attributeId, index, data });
    return result.template;
  }

  async deleteAttributeOption(attributeId: string, index: number): Promise<Attribute> {
    const result = await invokeAndUnwrap<{ template: Attribute }>('delete_attribute_option', { attributeId, index });
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

  // ============ Store Info ============

  async getStoreInfo(): Promise<StoreInfo> {
    return invokeAndUnwrap<StoreInfo>('get_store_info');
  }

  async updateStoreInfo(data: StoreInfoUpdate): Promise<StoreInfo> {
    return invokeAndUnwrap<StoreInfo>('update_store_info', { data });
  }

  // ============ Label Templates ============

  async listLabelTemplates(): Promise<LabelTemplate[]> {
    return invokeAndUnwrap<LabelTemplate[]>('list_label_templates');
  }

  async getLabelTemplate(id: string): Promise<LabelTemplate> {
    return invokeAndUnwrap<LabelTemplate>('get_label_template', { id });
  }

  async createLabelTemplate(data: LabelTemplateCreate): Promise<LabelTemplate> {
    return invokeAndUnwrap<LabelTemplate>('create_label_template', { data });
  }

  async updateLabelTemplate(id: string, data: LabelTemplateUpdate): Promise<LabelTemplate> {
    return invokeAndUnwrap<LabelTemplate>('update_label_template', { id, data });
  }

  async deleteLabelTemplate(id: string): Promise<boolean> {
    return invokeAndUnwrap<boolean>('delete_label_template', { id });
  }

  // ============ Price Rules ============

  async listPriceRules(): Promise<PriceRule[]> {
    const data = await invokeAndUnwrap<{ rules: PriceRule[] }>('list_price_rules');
    return data.rules;
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
    return invokeAndUnwrap<RolePermissionListData>('get_role_permissions', { roleId });
  }

  // ============ Token Management ============
  // In Tauri mode, authentication is handled by ClientBridge on Rust side

  async refreshToken(): Promise<void> {
    await invokeAndUnwrap<void>('refresh_token');
  }

  // ============ Shifts (班次管理) ============

  async listShifts(params?: { limit?: number; offset?: number; startDate?: string; endDate?: string }): Promise<Shift[]> {
    return invokeAndUnwrap<Shift[]>('list_shifts', params);
  }

  async getShift(id: string): Promise<Shift> {
    return invokeAndUnwrap<Shift>('get_shift', { id });
  }

  async getCurrentShift(operatorId?: string): Promise<Shift | null> {
    return invokeAndUnwrap<Shift | null>('get_current_shift', { operatorId });
  }

  async openShift(data: ShiftCreate): Promise<Shift> {
    return invokeAndUnwrap<Shift>('open_shift', { data });
  }

  async updateShift(id: string, data: ShiftUpdate): Promise<Shift> {
    return invokeAndUnwrap<Shift>('update_shift', { id, data });
  }

  async closeShift(id: string, data: ShiftClose): Promise<Shift> {
    return invokeAndUnwrap<Shift>('close_shift', { id, data });
  }

  async forceCloseShift(id: string, data?: ShiftForceClose): Promise<Shift> {
    return invokeAndUnwrap<Shift>('force_close_shift', { id, data: data ?? {} });
  }

  async heartbeatShift(id: string): Promise<boolean> {
    return invokeAndUnwrap<boolean>('heartbeat_shift', { id });
  }

  async recoverStaleShifts(): Promise<Shift[]> {
    return invokeAndUnwrap<Shift[]>('recover_stale_shifts');
  }

  /** @TEST 上线前删除 */
  async debugSimulateShiftAutoClose(): Promise<Shift[]> {
    return invokeAndUnwrap<Shift[]>('debug_simulate_shift_auto_close');
  }

  // ============ Daily Reports (日结报告) ============

  async listDailyReports(params?: { limit?: number; offset?: number; startDate?: string; endDate?: string }): Promise<DailyReport[]> {
    return invokeAndUnwrap<DailyReport[]>('list_daily_reports', params);
  }

  async getDailyReport(id: string): Promise<DailyReport> {
    return invokeAndUnwrap<DailyReport>('get_daily_report', { id });
  }

  async getDailyReportByDate(date: string): Promise<DailyReport> {
    return invokeAndUnwrap<DailyReport>('get_daily_report_by_date', { date });
  }

  async generateDailyReport(data: DailyReportGenerate): Promise<DailyReport> {
    return invokeAndUnwrap<DailyReport>('generate_daily_report', { data });
  }

  // ============ Audit Log (审计日志) ============

  async listAuditLogs(query: {
    from?: number;
    to?: number;
    action?: string;
    operator_id?: string;
    resource_type?: string;
    offset?: number;
    limit?: number;
  }): Promise<AuditListResponse> {
    const params = new URLSearchParams();
    for (const [key, value] of Object.entries(query)) {
      if (value !== undefined) {
        params.set(key, String(value));
      }
    }
    const qs = params.toString();
    const path = qs ? `/api/audit-log?${qs}` : '/api/audit-log';
    return invokeAndUnwrap<AuditListResponse>('api_get', { path });
  }

  // ============ System Issues (系统问题) ============

  async getSystemIssues(): Promise<SystemIssue[]> {
    return invokeAndUnwrap<SystemIssue[]>('api_get', { path: '/api/system-issues/pending' });
  }

  async resolveSystemIssue(data: ResolveSystemIssueRequest): Promise<void> {
    await invokeAndUnwrap<unknown>('api_post', {
      path: '/api/system-issues/resolve',
      body: data,
    });
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
