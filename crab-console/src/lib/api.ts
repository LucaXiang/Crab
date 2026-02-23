const API_BASE = 'https://auth.redcoral.app';

export class ApiError extends Error {
	status: number;
	code: number | null;

	constructor(status: number, message: string, code: number | null = null) {
		super(message);
		this.status = status;
		this.code = code;
	}
}

async function request<T>(
	method: string,
	path: string,
	body?: unknown,
	token?: string
): Promise<T> {
	const headers: Record<string, string> = { 'Content-Type': 'application/json' };
	if (token) headers['Authorization'] = `Bearer ${token}`;

	const res = await fetch(`${API_BASE}${path}`, {
		method,
		headers,
		body: body ? JSON.stringify(body) : undefined
	});

	const data = await res.json().catch(() => null);

	if (!res.ok) {
		const msg = data?.error ?? data?.message ?? res.statusText;
		const code = typeof data?.code === 'number' ? data.code : null;
		throw new ApiError(res.status, msg, code);
	}

	return data as T;
}

// === Auth ===

export interface LoginResponse {
	token: string;
	tenant_id: string;
}

export function login(email: string, password: string): Promise<LoginResponse> {
	return request('POST', '/api/tenant/login', { email, password });
}

// === Profile ===

export interface TenantProfile {
	id: string;
	email: string;
	name: string | null;
	status: string;
	created_at: number;
}

export interface Subscription {
	id: string;
	status: string;
	plan: string;
	max_edge_servers: number;
	max_clients: number;
	current_period_end: number | null;
	cancel_at_period_end: boolean;
	billing_interval: string | null;
	created_at: number;
}

export interface P12Info {
	has_p12: boolean;
	fingerprint: string | null;
	subject: string | null;
	expires_at: number | null;
}

export interface ProfileResponse {
	profile: TenantProfile;
	subscription: Subscription | null;
	p12: P12Info | null;
}

export function getProfile(token: string): Promise<ProfileResponse> {
	return request('GET', '/api/tenant/profile', undefined, token);
}

export function updateProfile(token: string, name: string): Promise<{ message: string }> {
	return request('PUT', '/api/tenant/profile', { name }, token);
}

// === Stores ===

export interface StoreDetail {
	id: number;
	entity_id: string;
	name?: string;
	address?: string;
	phone?: string;
	nif?: string;
	email?: string;
	website?: string;
	business_day_cutoff?: string;
	device_id: string;
	is_online: boolean;
	last_sync_at: number | null;
	registered_at: number;
	store_info: Record<string, unknown> | null;
}

export function getStores(token: string): Promise<StoreDetail[]> {
	return request('GET', '/api/tenant/stores', undefined, token);
}

export function updateStore(
	token: string,
	storeId: number,
	name?: string,
	address?: string,
	phone?: string,
	nif?: string,
	email?: string,
	website?: string,
	business_day_cutoff?: string
): Promise<void> {
	return request(
		'PATCH',
		`/api/tenant/stores/${storeId}`,
		{ name, address, phone, nif, email, website, business_day_cutoff },
		token
	);
}

// === Orders ===

export interface OrderSummary {
	id: number;
	source_id: string;
	receipt_number: string | null;
	status: string;
	end_time: number | null;
	total: number | null;
	synced_at: number;
}

export function getOrders(
	token: string,
	storeId: number,
	page = 1,
	perPage = 20,
	status?: string
): Promise<OrderSummary[]> {
	let path = `/api/tenant/stores/${storeId}/orders?page=${page}&per_page=${perPage}`;
	if (status) path += `&status=${status}`;
	return request('GET', path, undefined, token);
}

export interface OrderItemOption {
	attribute_name: string;
	option_name: string;
	price: number;
	quantity: number;
}

export interface OrderItem {
	name: string;
	spec_name: string | null;
	category_name: string | null;
	price: number;
	quantity: number;
	unit_price: number;
	line_total: number;
	discount_amount: number;
	surcharge_amount: number;
	tax: number;
	tax_rate: number;
	is_comped: boolean;
	note: string | null;
	options: OrderItemOption[];
}

export interface OrderPayment {
	seq: number;
	method: string;
	amount: number;
	timestamp: number;
	cancelled: boolean;
}

export interface TaxDesglose {
	tax_rate: number;
	base_amount: number;
	tax_amount: number;
}

export interface OrderDetailPayload {
	zone_name: string | null;
	table_name: string | null;
	is_retail: boolean;
	guest_count: number | null;
	original_total: number;
	subtotal: number;
	paid_amount: number;
	discount_amount: number;
	surcharge_amount: number;
	comp_total_amount: number;
	order_manual_discount_amount: number;
	order_manual_surcharge_amount: number;
	order_rule_discount_amount: number;
	order_rule_surcharge_amount: number;
	start_time: number;
	operator_name: string | null;
	void_type: string | null;
	loss_reason: string | null;
	loss_amount: number | null;
	void_note: string | null;
	member_name: string | null;
	items: OrderItem[];
	payments: OrderPayment[];
}

export interface OrderDetailResponse {
	source: string;
	detail: OrderDetailPayload;
	desglose: TaxDesglose[];
}

export function getOrderDetail(
	token: string,
	storeId: number,
	orderKey: string
): Promise<OrderDetailResponse> {
	return request(
		'GET',
		`/api/tenant/stores/${storeId}/orders/${orderKey}/detail`,
		undefined,
		token
	);
}

// === Stats ===

export interface DailyReportEntry {
	id: number;
	source_id: string;
	data: Record<string, unknown>;
	synced_at: number;
}

export function getStats(
	token: string,
	storeId: number,
	from?: number,
	to?: number
): Promise<DailyReportEntry[]> {
	let path = `/api/tenant/stores/${storeId}/stats?`;
	if (from) path += `from=${from}&`;
	if (to) path += `to=${to}&`;
	return request('GET', path, undefined, token);
}

// === Store Overview (real-time statistics) ===

export interface RevenueTrendPoint {
	hour: number;
	revenue: number;
	orders: number;
}

export interface TaxBreakdownStat {
	tax_rate: number;
	base_amount: number;
	tax_amount: number;
}

export interface PaymentBreakdownStat {
	method: string;
	amount: number;
	count: number;
}

export interface TopProductStat {
	name: string;
	quantity: number;
	revenue: number;
}

export interface CategorySaleStat {
	name: string;
	revenue: number;
}

export interface StoreOverview {
	revenue: number;
	orders: number;
	guests: number;
	average_order_value: number;
	per_guest_spend: number;
	average_dining_minutes: number;
	total_tax: number;
	total_discount: number;
	voided_orders: number;
	voided_amount: number;
	loss_orders: number;
	loss_amount: number;
	revenue_trend: RevenueTrendPoint[];
	tax_breakdown: TaxBreakdownStat[];
	payment_breakdown: PaymentBreakdownStat[];
	top_products: TopProductStat[];
	category_sales: CategorySaleStat[];
}

export function getTenantOverview(
	token: string,
	from: number,
	to: number
): Promise<StoreOverview> {
	return request('GET', `/api/tenant/overview?from=${from}&to=${to}`, undefined, token);
}

export function getStoreOverview(
	token: string,
	storeId: number,
	from: number,
	to: number
): Promise<StoreOverview> {
	return request(
		'GET',
		`/api/tenant/stores/${storeId}/overview?from=${from}&to=${to}`,
		undefined,
		token
	);
}

// === Red Flags ===

export interface RedFlagsSummary {
	item_removals: number;
	item_comps: number;
	order_voids: number;
	order_discounts: number;
	price_modifications: number;
}

export interface OperatorRedFlags {
	operator_id: number | null;
	operator_name: string | null;
	item_removals: number;
	item_comps: number;
	order_voids: number;
	order_discounts: number;
	price_modifications: number;
	total_flags: number;
}

export interface RedFlagsResponse {
	summary: RedFlagsSummary;
	operator_breakdown: OperatorRedFlags[];
}

export function getStoreRedFlags(
	token: string,
	storeId: number,
	from: number,
	to: number
): Promise<RedFlagsResponse> {
	return request(
		'GET',
		`/api/tenant/stores/${storeId}/red-flags?from=${from}&to=${to}`,
		undefined,
		token
	);
}

// === Catalog Types ===

export interface CatalogOpResult {
	success: boolean;
	created_id?: number;
	data?: unknown;
	error?: string;
}

export interface ProductSpec {
	source_id: number;
	name: string;
	price: number;
	display_order: number;
	is_default: boolean;
	is_active: boolean;
	receipt_name: string | null;
	is_root: boolean;
}

export interface CatalogProduct {
	source_id: number;
	name: string;
	image: string;
	category_source_id: number;
	category_name: string | null;
	sort_order: number;
	tax_rate: number;
	receipt_name: string | null;
	kitchen_print_name: string | null;
	is_kitchen_print_enabled: number;
	is_label_print_enabled: number;
	is_active: boolean;
	external_id: number | null;
	specs: ProductSpec[];
	tag_ids: number[];
}

export interface ProductSpecInput {
	name: string;
	price: number;
	display_order: number;
	is_default: boolean;
	is_active: boolean;
	receipt_name?: string | null;
	is_root: boolean;
}

export interface ProductCreate {
	name: string;
	image?: string;
	category_id: number;
	sort_order?: number;
	tax_rate?: number;
	receipt_name?: string;
	kitchen_print_name?: string;
	is_kitchen_print_enabled?: number;
	is_label_print_enabled?: number;
	external_id?: number;
	tags?: number[];
	specs: ProductSpecInput[];
}

export interface ProductUpdate {
	name?: string;
	image?: string;
	category_id?: number;
	sort_order?: number;
	tax_rate?: number;
	receipt_name?: string;
	kitchen_print_name?: string;
	is_kitchen_print_enabled?: number;
	is_label_print_enabled?: number;
	is_active?: boolean;
	external_id?: number;
	tags?: number[];
	specs?: ProductSpecInput[];
}

export interface CatalogCategory {
	source_id: number;
	name: string;
	sort_order: number;
	is_kitchen_print_enabled: boolean;
	is_label_print_enabled: boolean;
	is_active: boolean;
	is_virtual: boolean;
	match_mode: string;
	is_display: boolean;
	kitchen_print_destinations: number[];
	label_print_destinations: number[];
	tag_ids: number[];
}

export interface CategoryCreate {
	name: string;
	sort_order?: number;
	is_kitchen_print_enabled?: boolean;
	is_label_print_enabled?: boolean;
	is_virtual?: boolean;
	tag_ids?: number[];
	match_mode?: string;
	is_display?: boolean;
}

export interface CategoryUpdate {
	name?: string;
	sort_order?: number;
	is_kitchen_print_enabled?: boolean;
	is_label_print_enabled?: boolean;
	is_virtual?: boolean;
	tag_ids?: number[];
	match_mode?: string;
	is_active?: boolean;
	is_display?: boolean;
}

export interface CatalogTag {
	source_id: number;
	name: string;
	color: string;
	display_order: number;
	is_active: boolean;
	is_system: boolean;
}

export interface TagCreate {
	name: string;
	color?: string;
	display_order?: number;
}

export interface TagUpdate {
	name?: string;
	color?: string;
	display_order?: number;
	is_active?: boolean;
}

export interface CatalogAttributeOption {
	source_id: number;
	name: string;
	price_modifier: number;
	display_order: number;
	is_active: boolean;
	receipt_name: string | null;
	kitchen_print_name: string | null;
	enable_quantity: boolean;
	max_quantity: number | null;
}

export interface CatalogAttribute {
	source_id: number;
	name: string;
	is_multi_select: boolean;
	max_selections: number | null;
	default_option_ids: number[] | null;
	display_order: number;
	is_active: boolean;
	show_on_receipt: boolean;
	receipt_name: string | null;
	show_on_kitchen_print: boolean;
	kitchen_print_name: string | null;
	options: CatalogAttributeOption[];
}

export interface AttributeOptionInput {
	name: string;
	price_modifier: number;
	display_order: number;
	receipt_name?: string;
	kitchen_print_name?: string;
	enable_quantity: boolean;
	max_quantity?: number;
}

export interface AttributeCreate {
	name: string;
	is_multi_select?: boolean;
	max_selections?: number;
	display_order?: number;
	show_on_receipt?: boolean;
	receipt_name?: string;
	show_on_kitchen_print?: boolean;
	kitchen_print_name?: string;
	options?: AttributeOptionInput[];
}

export interface AttributeUpdate {
	name?: string;
	is_multi_select?: boolean;
	max_selections?: number;
	display_order?: number;
	show_on_receipt?: boolean;
	receipt_name?: string;
	show_on_kitchen_print?: boolean;
	kitchen_print_name?: string;
	options?: AttributeOptionInput[];
	is_active?: boolean;
}

export interface PriceRule {
	id: number;
	name: string;
	display_name: string;
	receipt_name: string;
	description: string | null;
	rule_type: 'DISCOUNT' | 'SURCHARGE';
	product_scope: 'GLOBAL' | 'CATEGORY' | 'TAG' | 'PRODUCT';
	target_id: number | null;
	zone_scope: string | null;
	adjustment_type: 'PERCENTAGE' | 'FIXED_AMOUNT';
	adjustment_value: number;
	is_stackable: boolean;
	is_exclusive: boolean;
	valid_from: number | null;
	valid_until: number | null;
	active_days: number[] | null;
	active_start_time: string | null;
	active_end_time: string | null;
	is_active: boolean;
	created_by: number | null;
}

export interface PriceRuleCreate {
	name: string;
	display_name: string;
	receipt_name: string;
	description?: string;
	rule_type: 'DISCOUNT' | 'SURCHARGE';
	product_scope: 'GLOBAL' | 'CATEGORY' | 'TAG' | 'PRODUCT';
	target_id?: number;
	zone_scope?: string;
	adjustment_type: 'PERCENTAGE' | 'FIXED_AMOUNT';
	adjustment_value: number;
	is_stackable?: boolean;
	is_exclusive?: boolean;
	valid_from?: number;
	valid_until?: number;
	active_days?: number[];
	active_start_time?: string;
	active_end_time?: string;
}

export interface PriceRuleUpdate {
	name?: string;
	display_name?: string;
	receipt_name?: string;
	description?: string;
	rule_type?: 'DISCOUNT' | 'SURCHARGE';
	product_scope?: 'GLOBAL' | 'CATEGORY' | 'TAG' | 'PRODUCT';
	target_id?: number;
	zone_scope?: string;
	adjustment_type?: 'PERCENTAGE' | 'FIXED_AMOUNT';
	adjustment_value?: number;
	is_stackable?: boolean;
	is_exclusive?: boolean;
	valid_from?: number;
	valid_until?: number;
	active_days?: number[];
	active_start_time?: string;
	active_end_time?: string;
	is_active?: boolean;
}

// === Catalog API — Products ===

const cat = (storeId: number) => `/api/tenant/stores/${storeId}/catalog`;

export function getProducts(token: string, storeId: number): Promise<CatalogProduct[]> {
	return request('GET', `${cat(storeId)}/products`, undefined, token);
}

export function createProduct(token: string, storeId: number, data: ProductCreate): Promise<CatalogOpResult> {
	return request('POST', `${cat(storeId)}/products`, data, token);
}

export function updateProduct(token: string, storeId: number, id: number, data: ProductUpdate): Promise<CatalogOpResult> {
	return request('PUT', `${cat(storeId)}/products/${id}`, data, token);
}

export function deleteProduct(token: string, storeId: number, id: number): Promise<CatalogOpResult> {
	return request('DELETE', `${cat(storeId)}/products/${id}`, undefined, token);
}

// === Catalog API — Categories ===

export function getCategories(token: string, storeId: number): Promise<CatalogCategory[]> {
	return request('GET', `${cat(storeId)}/categories`, undefined, token);
}

export function createCategory(token: string, storeId: number, data: CategoryCreate): Promise<CatalogOpResult> {
	return request('POST', `${cat(storeId)}/categories`, data, token);
}

export function updateCategory(token: string, storeId: number, id: number, data: CategoryUpdate): Promise<CatalogOpResult> {
	return request('PUT', `${cat(storeId)}/categories/${id}`, data, token);
}

export function deleteCategory(token: string, storeId: number, id: number): Promise<CatalogOpResult> {
	return request('DELETE', `${cat(storeId)}/categories/${id}`, undefined, token);
}

// === Catalog API — Tags ===

export function getTags(token: string, storeId: number): Promise<CatalogTag[]> {
	return request('GET', `${cat(storeId)}/tags`, undefined, token);
}

export function createTag(token: string, storeId: number, data: TagCreate): Promise<CatalogOpResult> {
	return request('POST', `${cat(storeId)}/tags`, data, token);
}

export function updateTag(token: string, storeId: number, id: number, data: TagUpdate): Promise<CatalogOpResult> {
	return request('PUT', `${cat(storeId)}/tags/${id}`, data, token);
}

export function deleteTag(token: string, storeId: number, id: number): Promise<CatalogOpResult> {
	return request('DELETE', `${cat(storeId)}/tags/${id}`, undefined, token);
}

// === Catalog API — Attributes ===

export function getAttributes(token: string, storeId: number): Promise<CatalogAttribute[]> {
	return request('GET', `${cat(storeId)}/attributes`, undefined, token);
}

export function createAttribute(token: string, storeId: number, data: AttributeCreate): Promise<CatalogOpResult> {
	return request('POST', `${cat(storeId)}/attributes`, data, token);
}

export function updateAttribute(token: string, storeId: number, id: number, data: AttributeUpdate): Promise<CatalogOpResult> {
	return request('PUT', `${cat(storeId)}/attributes/${id}`, data, token);
}

export function deleteAttribute(token: string, storeId: number, id: number): Promise<CatalogOpResult> {
	return request('DELETE', `${cat(storeId)}/attributes/${id}`, undefined, token);
}

// === Catalog API — Price Rules ===

export function getPriceRules(token: string, storeId: number): Promise<PriceRule[]> {
	return request('GET', `${cat(storeId)}/price-rules`, undefined, token);
}

export function createPriceRule(token: string, storeId: number, data: PriceRuleCreate): Promise<CatalogOpResult> {
	return request('POST', `${cat(storeId)}/price-rules`, data, token);
}

export function updatePriceRule(token: string, storeId: number, id: number, data: PriceRuleUpdate): Promise<CatalogOpResult> {
	return request('PUT', `${cat(storeId)}/price-rules/${id}`, data, token);
}

export function deletePriceRule(token: string, storeId: number, id: number): Promise<CatalogOpResult> {
	return request('DELETE', `${cat(storeId)}/price-rules/${id}`, undefined, token);
}

// === Catalog API — Employees ===

export interface CatalogEmployee {
	id: number;
	username: string;
	display_name: string;
	role_id: number;
	is_system: boolean;
	is_active: boolean;
	created_at: number;
}

export interface EmployeeCreate {
	username: string;
	password: string;
	display_name?: string;
	role_id: number;
}

export interface EmployeeUpdate {
	username?: string;
	password?: string;
	display_name?: string;
	role_id?: number;
	is_active?: boolean;
}

export function getEmployees(token: string, storeId: number): Promise<CatalogEmployee[]> {
	return request('GET', `${cat(storeId)}/employees`, undefined, token);
}

export function createEmployee(token: string, storeId: number, data: EmployeeCreate): Promise<CatalogOpResult> {
	return request('POST', `${cat(storeId)}/employees`, data, token);
}

export function updateEmployee(token: string, storeId: number, id: number, data: EmployeeUpdate): Promise<CatalogOpResult> {
	return request('PUT', `${cat(storeId)}/employees/${id}`, data, token);
}

export function deleteEmployee(token: string, storeId: number, id: number): Promise<CatalogOpResult> {
	return request('DELETE', `${cat(storeId)}/employees/${id}`, undefined, token);
}

// === Catalog API — Zones ===

export interface CatalogZone {
	id: number;
	name: string;
	description: string | null;
	is_active: boolean;
}

export interface ZoneCreate {
	name: string;
	description?: string;
}

export interface ZoneUpdate {
	name?: string;
	description?: string;
	is_active?: boolean;
}

export function getZones(token: string, storeId: number): Promise<CatalogZone[]> {
	return request('GET', `${cat(storeId)}/zones`, undefined, token);
}

export function createZone(token: string, storeId: number, data: ZoneCreate): Promise<CatalogOpResult> {
	return request('POST', `${cat(storeId)}/zones`, data, token);
}

export function updateZone(token: string, storeId: number, id: number, data: ZoneUpdate): Promise<CatalogOpResult> {
	return request('PUT', `${cat(storeId)}/zones/${id}`, data, token);
}

export function deleteZone(token: string, storeId: number, id: number): Promise<CatalogOpResult> {
	return request('DELETE', `${cat(storeId)}/zones/${id}`, undefined, token);
}

// === Catalog API — Dining Tables ===

export interface CatalogDiningTable {
	id: number;
	name: string;
	zone_id: number;
	capacity: number;
	is_active: boolean;
}

export interface DiningTableCreate {
	name: string;
	zone_id: number;
	capacity?: number;
}

export interface DiningTableUpdate {
	name?: string;
	zone_id?: number;
	capacity?: number;
	is_active?: boolean;
}

export function getTables(token: string, storeId: number): Promise<CatalogDiningTable[]> {
	return request('GET', `${cat(storeId)}/tables`, undefined, token);
}

export function createTable(token: string, storeId: number, data: DiningTableCreate): Promise<CatalogOpResult> {
	return request('POST', `${cat(storeId)}/tables`, data, token);
}

export function updateTable(token: string, storeId: number, id: number, data: DiningTableUpdate): Promise<CatalogOpResult> {
	return request('PUT', `${cat(storeId)}/tables/${id}`, data, token);
}

export function deleteTable(token: string, storeId: number, id: number): Promise<CatalogOpResult> {
	return request('DELETE', `${cat(storeId)}/tables/${id}`, undefined, token);
}

// === Commands ===

export interface CommandRecord {
	id: number;
	command_type: string;
	payload: Record<string, unknown>;
	status: string;
	created_at: number;
	executed_at: number | null;
	result: Record<string, unknown> | null;
}

export function getCommands(
	token: string,
	storeId: number,
	page = 1,
	perPage = 20
): Promise<CommandRecord[]> {
	return request(
		'GET',
		`/api/tenant/stores/${storeId}/commands?page=${page}&per_page=${perPage}`,
		undefined,
		token
	);
}

export function createCommand(
	token: string,
	storeId: number,
	commandType: string,
	payload: Record<string, unknown> = {}
): Promise<{
	command_id: number;
	success: boolean;
	data?: unknown;
	error?: string;
}> {
	return request(
		'POST',
		`/api/tenant/stores/${storeId}/commands`,
		{ command_type: commandType, payload },
		token
	);
}

// === Audit ===

export interface AuditEntry {
	id: number;
	action: string;
	detail: Record<string, unknown> | null;
	ip_address: string | null;
	created_at: number;
}

export function getAuditLog(token: string, page = 1, perPage = 20): Promise<AuditEntry[]> {
	return request(
		'GET',
		`/api/tenant/audit-log?page=${page}&per_page=${perPage}`,
		undefined,
		token
	);
}

// === Account ===

export function changePassword(
	token: string,
	currentPassword: string,
	newPassword: string
): Promise<{ message: string }> {
	return request(
		'POST',
		'/api/tenant/change-password',
		{ current_password: currentPassword, new_password: newPassword },
		token
	);
}

export function changeEmail(
	token: string,
	currentPassword: string,
	newEmail: string
): Promise<{ message: string }> {
	return request(
		'POST',
		'/api/tenant/change-email',
		{ current_password: currentPassword, new_email: newEmail },
		token
	);
}

export function confirmEmailChange(
	token: string,
	newEmail: string,
	code: string
): Promise<{ message: string }> {
	return request(
		'POST',
		'/api/tenant/confirm-email-change',
		{ new_email: newEmail, code },
		token
	);
}

export function forgotPassword(email: string): Promise<{ message: string }> {
	return request('POST', '/api/tenant/forgot-password', { email });
}

export function resetPassword(
	email: string,
	code: string,
	newPassword: string
): Promise<{ message: string }> {
	return request('POST', '/api/tenant/reset-password', {
		email,
		code,
		new_password: newPassword
	});
}

export function createBillingPortal(token: string): Promise<{ url: string }> {
	return request('POST', '/api/tenant/billing-portal', undefined, token);
}

export function createCheckout(token: string, plan: string): Promise<{ checkout_url: string }> {
	return request('POST', '/api/tenant/create-checkout', { plan }, token);
}

// === P12 Certificate ===

export interface P12UploadResponse {
	success: boolean;
	fingerprint: string;
	common_name: string;
	organization: string | null;
	tax_id: string | null;
	issuer: string;
	expires_at: number;
}

export async function uploadP12(
	token: string,
	p12File: File,
	p12Password: string
): Promise<P12UploadResponse> {
	const form = new FormData();
	form.append('token', token);
	form.append('p12_password', p12Password);
	form.append('p12_file', p12File);

	const res = await fetch(`${API_BASE}/api/p12/upload`, {
		method: 'POST',
		body: form
	});

	const data = await res.json().catch(() => null);

	if (!res.ok || data?.success === false) {
		const msg = data?.error ?? res.statusText;
		const code = data?.error_code ?? null;
		throw new ApiError(res.status, msg, code);
	}

	return data as P12UploadResponse;
}
