/**
 * Tauri API Client - 通过 Tauri Commands 调用 API
 *
 * 替代直接 HTTP 调用，所有请求通过:
 * invoke() → Tauri Command → ClientBridge → CrabClient → EdgeServer
 *
 * 这样可以正确处理 mTLS 认证（自签名证书）
 */

import { invoke } from '@tauri-apps/api/core';

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

  async login(data: { username: string; password: string }) {
    return invokeCommand('login_employee', {
      username: data.username,
      password: data.password
    });
  }

  async logout() {
    return invokeCommand('logout_employee');
  }

  async getCurrentUser() {
    return invokeCommand('get_current_session');
  }

  // ============ Tags ============

  async listTags() {
    return invokeCommand('list_tags');
  }

  async getTag(id: string) {
    return invokeCommand('get_tag', { id });
  }

  async createTag(data: { name: string; color?: string; display_order?: number }) {
    return invokeCommand('create_tag', { data });
  }

  async updateTag(id: string, data: { name?: string; color?: string; display_order?: number; is_active?: boolean }) {
    return invokeCommand('update_tag', { id, data });
  }

  async deleteTag(id: string) {
    return invokeCommand('delete_tag', { id });
  }

  // ============ Categories ============

  async listCategories() {
    return invokeCommand('list_categories');
  }

  async getCategory(id: string) {
    return invokeCommand('get_category', { id });
  }

  async createCategory(data: { name: string; sort_order?: number }) {
    return invokeCommand('create_category', { data });
  }

  async updateCategory(id: string, data: { name?: string; sort_order?: number; is_active?: boolean }) {
    return invokeCommand('update_category', { id, data });
  }

  async deleteCategory(id: string) {
    return invokeCommand('delete_category', { id });
  }

  // ============ Products ============

  async listProducts() {
    return invokeCommand('list_products');
  }

  async getProduct(id: string) {
    return invokeCommand('get_product', { id });
  }

  async createProduct(data: Record<string, unknown>) {
    return invokeCommand('create_product', { data });
  }

  async updateProduct(id: string, data: Record<string, unknown>) {
    return invokeCommand('update_product', { id, data });
  }

  async deleteProduct(id: string) {
    return invokeCommand('delete_product', { id });
  }

  // ============ Product Specifications ============

  async listProductSpecs(productId: string) {
    return invokeCommand('list_specs', { product_id: productId });
  }

  async createProductSpec(data: Record<string, unknown>) {
    return invokeCommand('create_spec', { data });
  }

  async updateProductSpec(id: string, data: Record<string, unknown>) {
    return invokeCommand('update_spec', { id, data });
  }

  async deleteProductSpec(id: string) {
    return invokeCommand('delete_spec', { id });
  }

  // ============ Product Attributes ============

  async fetchProductAttributes(productId: string) {
    return invokeCommand('list_product_attributes', { product_id: productId });
  }

  async bindProductAttribute(data: Record<string, unknown>) {
    return invokeCommand('bind_product_attribute', { data });
  }

  async unbindProductAttribute(id: string) {
    return invokeCommand('unbind_product_attribute', { id });
  }

  // ============ Attributes ============

  async listAttributeTemplates() {
    return invokeCommand('list_attributes');
  }

  async getAttributeTemplate(id: string) {
    return invokeCommand('get_attribute', { id });
  }

  async createAttributeTemplate(data: Record<string, unknown>) {
    return invokeCommand('create_attribute', { data });
  }

  async updateAttributeTemplate(id: string, data: Record<string, unknown>) {
    return invokeCommand('update_attribute', { id, data });
  }

  async deleteAttributeTemplate(id: string) {
    return invokeCommand('delete_attribute', { id });
  }

  // ============ Zones ============

  async listZones() {
    return invokeCommand('list_zones');
  }

  async getZone(id: string) {
    return invokeCommand('get_zone', { id });
  }

  async createZone(data: { name: string; description?: string }) {
    return invokeCommand('create_zone', { data });
  }

  async updateZone(id: string, data: { name?: string; description?: string; is_active?: boolean }) {
    return invokeCommand('update_zone', { id, data });
  }

  async deleteZone(id: string) {
    return invokeCommand('delete_zone', { id });
  }

  // ============ Tables ============

  async listTables() {
    return invokeCommand('list_tables');
  }

  async getTablesByZone(zoneId: string) {
    return invokeCommand('list_tables_by_zone', { zone_id: zoneId });
  }

  async getTable(id: string) {
    return invokeCommand('get_table', { id });
  }

  async createTable(data: { name: string; zone: string; capacity?: number }) {
    return invokeCommand('create_table', { data });
  }

  async updateTable(id: string, data: { name?: string; zone?: string; capacity?: number; is_active?: boolean }) {
    return invokeCommand('update_table', { id, data });
  }

  async deleteTable(id: string) {
    return invokeCommand('delete_table', { id });
  }

  // ============ Kitchen Printers ============

  async listPrinters() {
    return invokeCommand('list_kitchen_printers');
  }

  async getPrinter(id: string) {
    return invokeCommand('get_kitchen_printer', { id });
  }

  async createPrinter(data: { name: string; printer_name?: string; description?: string }) {
    return invokeCommand('create_kitchen_printer', { data });
  }

  async updatePrinter(id: string, data: { name?: string; printer_name?: string; description?: string; is_active?: boolean }) {
    return invokeCommand('update_kitchen_printer', { id, data });
  }

  async deletePrinter(id: string) {
    return invokeCommand('delete_kitchen_printer', { id });
  }

  // ============ Employees ============

  async listEmployees() {
    return invokeCommand('list_employees');
  }

  async getEmployee(id: string) {
    return invokeCommand('get_employee', { id });
  }

  async createEmployee(data: { username: string; password: string; role: string }) {
    return invokeCommand('create_employee', { data });
  }

  async updateEmployee(id: string, data: { username?: string; password?: string; role?: string; is_active?: boolean }) {
    return invokeCommand('update_employee', { id, data });
  }

  async deleteEmployee(id: string) {
    return invokeCommand('delete_employee', { id });
  }

  // ============ Price Rules ============

  async listPriceAdjustments() {
    return invokeCommand('list_price_rules');
  }

  async listActivePriceAdjustments() {
    return invokeCommand('list_active_price_rules');
  }

  async getPriceAdjustment(id: string) {
    return invokeCommand('get_price_rule', { id });
  }

  async createPriceAdjustment(data: Record<string, unknown>) {
    return invokeCommand('create_price_rule', { data });
  }

  async updatePriceAdjustment(id: string, data: Record<string, unknown>) {
    return invokeCommand('update_price_rule', { id, data });
  }

  async deletePriceAdjustment(id: string) {
    return invokeCommand('delete_price_rule', { id });
  }

  // ============ Roles ============

  async listRoles() {
    return invokeCommand('list_roles');
  }

  async getRole(id: string) {
    return invokeCommand('get_role', { id });
  }

  async createRole(data: { name: string }) {
    return invokeCommand('create_role', { data });
  }

  async updateRole(id: string, data: { name?: string }) {
    return invokeCommand('update_role', { id, data });
  }

  async deleteRole(id: string) {
    return invokeCommand('delete_role', { id });
  }

  async getRolePermissions(roleId: string) {
    return invokeCommand('get_role_permissions', { role_id: roleId });
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
