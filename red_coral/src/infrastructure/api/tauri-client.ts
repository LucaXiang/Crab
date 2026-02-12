/**
 * Tauri API Client - 通过 Tauri Commands 调用 API
 *
 * 替代直接 HTTP 调用，所有请求通过:
 * invoke() → Tauri Command → ClientBridge → CrabClient → EdgeServer
 *
 * 这样可以正确处理 mTLS 认证（自签名证书）
 */

import { invoke } from '@tauri-apps/api/core';
import { logger } from '@/utils/logger';
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
  Category,
  CategoryCreate,
  CategoryUpdate,
  Product,
  ProductCreate,
  ProductUpdate,
  ProductFull,
  Zone,
  Table,
  PrintDestination,
  PrintDestinationCreate,
  PrintDestinationUpdate,
  Attribute,
  AttributeCreate,
  AttributeUpdate,
  AttributeBindingFull,
  AttributeBinding,
  Employee,
  Role,
  RolePermission,
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
    logger.warn(`Auth error ${code}, clearing auth state`, { component: 'invokeApi' });
    useAuthStore.getState().logout();
    useBridgeStore.setState({ currentSession: null });
  } finally {
    isHandlingAuthError = false;
  }
}

/** 根据错误码 + details + 原始消息生成用户友好的错误提示 */
function localizeErrorCode(
  code: number,
  rawMessage?: string,
  details?: Record<string, unknown>
): string {
  // 优先用错误码查 i18n
  const key = `errors.${code}`;
  // 将 details 转换为 t() 需要的参数格式
  const params = details
    ? Object.fromEntries(
        Object.entries(details).map(([k, v]) => [k, String(v)])
      )
    : undefined;
  const localized = t(key, params);
  if (localized !== key) {
    return localized;
  }
  // i18n 没有对应 key，用 rawMessage 做关键词匹配作为 fallback
  logger.warn(`Missing i18n for error code ${code}, add to zh-CN.json`, { component: 'invokeApi' });
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
      throw new ApiError(
        response.code,
        localizeErrorCode(response.code, response.message, response.details ?? undefined),
        response.details ?? undefined
      );
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
    return invokeApi<LoginResponseData>('login_employee', {
      username: data.username,
      password: data.password,
    });
  }

  async logout(): Promise<void> {
    await invokeApi<void>('logout_employee');
  }

  // ============ Tags ============

  async listTags(): Promise<Tag[]> {
    return invokeApi<Tag[]>('list_tags');
  }

  async createTag(data: TagCreate): Promise<Tag> {
    return invokeApi<Tag>('create_tag', { data });
  }

  async updateTag(id: number, data: TagUpdate): Promise<Tag> {
    return invokeApi<Tag>('update_tag', { id, data });
  }

  async deleteTag(id: number): Promise<void> {
    await invokeApi<void>('delete_tag', { id });
  }

  // ============ Categories ============

  async listCategories(): Promise<Category[]> {
    return invokeApi<Category[]>('list_categories');
  }

  async createCategory(data: CategoryCreate): Promise<Category> {
    return invokeApi<Category>('create_category', { data });
  }

  async updateCategory(id: number, data: CategoryUpdate): Promise<Category> {
    return invokeApi<Category>('update_category', { id, data });
  }

  async deleteCategory(id: number): Promise<void> {
    await invokeApi<void>('delete_category', { id });
  }

  async batchUpdateCategorySortOrder(updates: { id: number; sort_order: number }[]): Promise<void> {
    await invokeApi<void>('batch_update_category_sort_order', { updates });
  }

  // ============ Products ============

  async listProducts(): Promise<ProductFull[]> {
    return invokeApi<ProductFull[]>('list_products');
  }

  async getProductFull(id: number): Promise<ProductFull> {
    return invokeApi<ProductFull>('get_product_full', { id });
  }

  async createProduct(data: ProductCreate): Promise<ProductFull> {
    return invokeApi<ProductFull>('create_product', { data });
  }

  async updateProduct(id: number, data: ProductUpdate): Promise<ProductFull> {
    return invokeApi<ProductFull>('update_product', { id, data });
  }

  async deleteProduct(id: number): Promise<void> {
    await invokeApi<void>('delete_product', { id });
  }

  async bulkDeleteProducts(ids: number[]): Promise<void> {
    for (const id of ids) {
      await this.deleteProduct(id);
    }
  }

  async batchUpdateProductSortOrder(updates: { id: number; sort_order: number }[]): Promise<void> {
    await invokeApi<void>('batch_update_product_sort_order', { updates });
  }

  // ============ Product Attributes ============

  async fetchProductAttributes(productId: number): Promise<AttributeBindingFull[]> {
    return await invokeApi<AttributeBindingFull[]>('list_product_attributes', { productId });
  }

  async bindProductAttribute(data: CreateProductAttributeRequest): Promise<AttributeBinding> {
    return invokeApi<AttributeBinding>('bind_product_attribute', { data });
  }

  async unbindProductAttribute(id: number): Promise<void> {
    await invokeApi<void>('unbind_product_attribute', { id });
  }

  // ============ Category Attributes ============

  async listCategoryAttributes(categoryId: number): Promise<Attribute[]> {
    return invokeApi<Attribute[]>('list_category_attributes', { categoryId });
  }

  async bindCategoryAttribute(data: CreateCategoryAttributeRequest): Promise<AttributeBinding> {
    return invokeApi<AttributeBinding>('bind_category_attribute', { data });
  }

  async unbindCategoryAttribute(categoryId: number, attributeId: number): Promise<void> {
    await invokeApi<void>('unbind_category_attribute', { categoryId, attributeId });
  }

  // ============ Attributes ============

  async listAttributes(): Promise<Attribute[]> {
    return invokeApi<Attribute[]>('list_attributes');
  }

  async getAttribute(id: number): Promise<Attribute> {
    return invokeApi<Attribute>('get_attribute', { id });
  }

  async createAttribute(data: AttributeCreate): Promise<Attribute> {
    return invokeApi<Attribute>('create_attribute', { data });
  }

  async updateAttribute(id: number, data: AttributeUpdate): Promise<Attribute> {
    return invokeApi<Attribute>('update_attribute', { id, data });
  }

  async deleteAttribute(id: number): Promise<void> {
    await invokeApi<void>('delete_attribute', { id });
  }

  // ============ Attribute Options ============

  async addAttributeOption(attributeId: number, data: { name: string; value_code?: string; price_modifier?: number; is_default?: boolean; display_order?: number; is_active?: boolean; receipt_name?: string; kitchen_print_name?: string; enable_quantity?: boolean; max_quantity?: number | null }): Promise<Attribute> {
    return invokeApi<Attribute>('add_attribute_option', { attributeId, data });
  }

  async updateAttributeOption(attributeId: number, index: number, data: { name?: string; value_code?: string; price_modifier?: number; is_default?: boolean; display_order?: number; is_active?: boolean; receipt_name?: string; kitchen_print_name?: string; enable_quantity?: boolean; max_quantity?: number | null }): Promise<Attribute> {
    return invokeApi<Attribute>('update_attribute_option', { attributeId, index, data });
  }

  async deleteAttributeOption(attributeId: number, index: number): Promise<Attribute> {
    return invokeApi<Attribute>('delete_attribute_option', { attributeId, index });
  }

  // ============ Zones ============

  async listZones(): Promise<Zone[]> {
    return invokeApi<Zone[]>('list_zones');
  }

  async createZone(data: { name: string; description?: string }): Promise<Zone> {
    return invokeApi<Zone>('create_zone', { data });
  }

  async updateZone(id: number, data: { name?: string; description?: string; is_active?: boolean }): Promise<Zone> {
    return invokeApi<Zone>('update_zone', { id, data });
  }

  async deleteZone(id: number): Promise<void> {
    await invokeApi<void>('delete_zone', { id });
  }

  // ============ Tables ============

  async listTables(): Promise<Table[]> {
    return invokeApi<Table[]>('list_tables');
  }

  async createTable(data: { name: string; zone_id: number; capacity?: number }): Promise<Table> {
    return invokeApi<Table>('create_table', { data });
  }

  async updateTable(id: number, data: { name?: string; zone_id?: number; capacity?: number; is_active?: boolean }): Promise<Table> {
    return invokeApi<Table>('update_table', { id, data });
  }

  async deleteTable(id: number): Promise<void> {
    await invokeApi<void>('delete_table', { id });
  }

  // ============ Print Destinations ============

  async listPrintDestinations(): Promise<PrintDestination[]> {
    return invokeApi<PrintDestination[]>('list_print_destinations');
  }

  async createPrintDestination(data: PrintDestinationCreate): Promise<PrintDestination> {
    return invokeApi<PrintDestination>('create_print_destination', { data });
  }

  async updatePrintDestination(id: number, data: PrintDestinationUpdate): Promise<PrintDestination> {
    return invokeApi<PrintDestination>('update_print_destination', { id, data });
  }

  async deletePrintDestination(id: number): Promise<void> {
    await invokeApi<void>('delete_print_destination', { id });
  }

  // ============ Employees ============

  async listEmployees(): Promise<Employee[]> {
    return invokeApi<Employee[]>('list_employees');
  }

  async createEmployee(data: { username: string; password: string; role_id: number }): Promise<Employee> {
    return invokeApi<Employee>('create_employee', { data });
  }

  async updateEmployee(id: number, data: { password?: string; role_id?: number; is_active?: boolean }): Promise<Employee> {
    return invokeApi<Employee>('update_employee', { id, data });
  }

  async deleteEmployee(id: number): Promise<void> {
    await invokeApi<void>('delete_employee', { id });
  }

  // ============ Store Info ============

  async getStoreInfo(): Promise<StoreInfo> {
    return invokeApi<StoreInfo>('get_store_info');
  }

  async updateStoreInfo(data: StoreInfoUpdate): Promise<StoreInfo> {
    return invokeApi<StoreInfo>('update_store_info', { data });
  }

  // ============ Label Templates ============

  async listLabelTemplates(): Promise<LabelTemplate[]> {
    return invokeApi<LabelTemplate[]>('list_label_templates');
  }

  async getLabelTemplate(id: number): Promise<LabelTemplate> {
    return invokeApi<LabelTemplate>('get_label_template', { id });
  }

  async createLabelTemplate(data: LabelTemplateCreate): Promise<LabelTemplate> {
    return invokeApi<LabelTemplate>('create_label_template', { data });
  }

  async updateLabelTemplate(id: number, data: LabelTemplateUpdate): Promise<LabelTemplate> {
    return invokeApi<LabelTemplate>('update_label_template', { id, data });
  }

  async deleteLabelTemplate(id: number): Promise<void> {
    await invokeApi<void>('delete_label_template', { id });
  }

  // ============ Price Rules ============

  async listPriceRules(): Promise<PriceRule[]> {
    return invokeApi<PriceRule[]>('list_price_rules');
  }

  async createPriceRule(data: PriceRuleCreate): Promise<PriceRule> {
    return invokeApi<PriceRule>('create_price_rule', { data });
  }

  async updatePriceRule(id: number, data: PriceRuleUpdate): Promise<PriceRule> {
    return invokeApi<PriceRule>('update_price_rule', { id, data });
  }

  async deletePriceRule(id: number): Promise<void> {
    await invokeApi<void>('delete_price_rule', { id });
  }

  // ============ Roles ============

  async listRoles(): Promise<Role[]> {
    return invokeApi<Role[]>('list_roles');
  }

  async getRolePermissions(roleId: number): Promise<RolePermission[]> {
    return invokeApi<RolePermission[]>('get_role_permissions', { roleId });
  }

  // ============ Token Management ============
  // In Tauri mode, authentication is handled by ClientBridge on Rust side

  async refreshToken(): Promise<void> {
    await invokeApi<void>('refresh_token');
  }

  // ============ Shifts (班次管理) ============

  async listShifts(params?: { limit?: number; offset?: number; startDate?: string; endDate?: string }): Promise<Shift[]> {
    return invokeApi<Shift[]>('list_shifts', params);
  }

  async getShift(id: number): Promise<Shift> {
    return invokeApi<Shift>('get_shift', { id });
  }

  async getCurrentShift(): Promise<Shift | null> {
    return invokeApi<Shift | null>('get_current_shift');
  }

  async openShift(data: ShiftCreate): Promise<Shift> {
    return invokeApi<Shift>('open_shift', { data });
  }

  async updateShift(id: number, data: ShiftUpdate): Promise<Shift> {
    return invokeApi<Shift>('update_shift', { id, data });
  }

  async closeShift(id: number, data: ShiftClose): Promise<Shift> {
    return invokeApi<Shift>('close_shift', { id, data });
  }

  async forceCloseShift(id: number, data?: ShiftForceClose): Promise<Shift> {
    return invokeApi<Shift>('force_close_shift', { id, data: data ?? {} });
  }

  async heartbeatShift(id: number): Promise<boolean> {
    return invokeApi<boolean>('heartbeat_shift', { id });
  }

  async recoverStaleShifts(): Promise<Shift[]> {
    return invokeApi<Shift[]>('recover_stale_shifts');
  }

  // ============ Daily Reports (日结报告) ============

  async listDailyReports(params?: { limit?: number; offset?: number; startDate?: string; endDate?: string }): Promise<DailyReport[]> {
    return invokeApi<DailyReport[]>('list_daily_reports', params);
  }

  async getDailyReport(id: number): Promise<DailyReport> {
    return invokeApi<DailyReport>('get_daily_report', { id });
  }

  async getDailyReportByDate(date: string): Promise<DailyReport> {
    return invokeApi<DailyReport>('get_daily_report_by_date', { date });
  }

  async generateDailyReport(data: DailyReportGenerate): Promise<DailyReport> {
    return invokeApi<DailyReport>('generate_daily_report', { data });
  }

  // ============ Audit Log (审计日志) ============

  async listAuditLogs(query: {
    from?: number;
    to?: number;
    action?: string;
    operator_name?: string;
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
    return invokeApi<AuditListResponse>('api_get', { path });
  }

  // ============ System Issues (系统问题) ============

  async getSystemIssues(): Promise<SystemIssue[]> {
    return invokeApi<SystemIssue[]>('api_get', { path: '/api/system-issues/pending' });
  }

  async resolveSystemIssue(data: ResolveSystemIssueRequest): Promise<void> {
    await invokeApi<unknown>('api_post', {
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
