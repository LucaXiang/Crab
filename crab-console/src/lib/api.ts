const API_BASE = 'https://auth.redcoral.app';

export class ApiError extends Error {
	status: number;
	code: string | null;

	constructor(status: number, message: string, code: string | null = null) {
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
		const code = data?.code ?? null;
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
	created_at: number;
}

export interface ProfileResponse {
	profile: TenantProfile;
	subscription: Subscription;
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
	device_id: string;
	last_sync_at: number | null;
	registered_at: number;
	store_info: Record<string, unknown> | null;
}

export function getStores(token: string): Promise<StoreDetail[]> {
	return request('GET', '/api/tenant/stores', undefined, token);
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

// === Products ===

export interface ProductEntry {
	id: number;
	source_id: string;
	data: Record<string, unknown>;
	synced_at: number;
}

export function getProducts(token: string, storeId: number): Promise<ProductEntry[]> {
	return request('GET', `/api/tenant/stores/${storeId}/products`, undefined, token);
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
): Promise<{ command_id: number; status: string; ws_queued: boolean }> {
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

export function createBillingPortal(token: string): Promise<{ url: string }> {
	return request('POST', '/api/tenant/billing-portal', undefined, token);
}

export function createCheckout(token: string, plan: string): Promise<{ checkout_url: string }> {
	return request('POST', '/api/tenant/create-checkout', { plan }, token);
}
