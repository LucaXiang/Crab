import { writable, derived } from 'svelte/store';
import { browser } from '$app/environment';

const STORAGE_KEY = 'redcoral-auth';

interface AuthState {
	token: string | null;
	tenant_id: string | null;
}

function loadAuth(): AuthState {
	if (!browser) return { token: null, tenant_id: null };
	try {
		const raw = localStorage.getItem(STORAGE_KEY);
		if (raw) return JSON.parse(raw);
	} catch {
		// ignore
	}
	return { token: null, tenant_id: null };
}

const initial = loadAuth();
export const authToken = writable<string | null>(initial.token);
export const tenantId = writable<string | null>(initial.tenant_id);
export const isAuthenticated = derived(authToken, ($token) => !!$token);

export function setAuth(token: string, tenant_id: string) {
	authToken.set(token);
	tenantId.set(tenant_id);
	if (browser) {
		localStorage.setItem(STORAGE_KEY, JSON.stringify({ token, tenant_id }));
	}
}

export function clearAuth() {
	authToken.set(null);
	tenantId.set(null);
	if (browser) {
		localStorage.removeItem(STORAGE_KEY);
	}
}
