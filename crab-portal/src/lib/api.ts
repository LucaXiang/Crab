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

async function request<T>(method: string, path: string, body?: unknown, token?: string): Promise<T> {
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

// === Registration ===

export interface RegisterRequest {
	email: string;
	password: string;
	plan?: string;
}

export interface RegisterResponse {
	tenant_id: string;
	message: string;
}

export function register(body: RegisterRequest): Promise<RegisterResponse> {
	return request('POST', '/api/register', body);
}

export interface VerifyEmailRequest {
	email: string;
	code: string;
}

export interface VerifyEmailResponse {
	checkout_url: string;
	message: string;
}

export function verifyEmail(body: VerifyEmailRequest): Promise<VerifyEmailResponse> {
	return request('POST', '/api/verify-email', body);
}

export function resendCode(email: string): Promise<{ message: string }> {
	return request('POST', '/api/resend-code', { email });
}

// === Auth ===

export interface LoginRequest {
	email: string;
	password: string;
}

export interface LoginResponse {
	token: string;
	tenant_id: string;
}

export function login(body: LoginRequest): Promise<LoginResponse> {
	return request('POST', '/api/tenant/login', body);
}

export function forgotPassword(email: string): Promise<{ message: string }> {
	return request('POST', '/api/tenant/forgot-password', { email });
}

export interface ResetPasswordRequest {
	email: string;
	code: string;
	new_password: string;
}

export function resetPassword(body: ResetPasswordRequest): Promise<{ message: string }> {
	return request('POST', '/api/tenant/reset-password', body);
}

// === Dashboard (authenticated) ===

export interface TenantProfile {
	tenant_id: string;
	email: string;
	name: string | null;
	status: string;
	plan: string;
	created_at: string;
}

export interface Subscription {
	plan: string;
	status: string;
	current_period_end: number | null;
	stripe_customer_id: string | null;
}

export interface ProfileResponse {
	profile: TenantProfile;
	subscription: Subscription;
}

export function getProfile(token: string): Promise<ProfileResponse> {
	return request('GET', '/api/tenant/profile', undefined, token);
}

export interface StoreInfo {
	id: number;
	entity_id: number;
	device_id: string;
	last_sync_at: number;
	registered_at: number;
	store_info: Record<string, unknown>;
}

export function getStores(token: string): Promise<StoreInfo[]> {
	return request('GET', '/api/tenant/stores', undefined, token);
}

export function createBillingPortal(token: string): Promise<{ url: string }> {
	return request('POST', '/api/tenant/billing-portal', undefined, token);
}

export function changePassword(
	token: string,
	current_password: string,
	new_password: string
): Promise<{ message: string }> {
	return request('POST', '/api/tenant/change-password', { current_password, new_password }, token);
}
