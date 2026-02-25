import { useAuthStore } from '@/core/stores/useAuthStore';

export const API_BASE = 'https://cloud.redcoral.app';

export class ApiError extends Error {
  status: number;
  code: number | null;

  constructor(status: number, message: string, code: number | null = null) {
    super(message);
    this.status = status;
    this.code = code;
  }
}

// Refresh lock: held until all waiters have read the result
let refreshPromise: Promise<boolean> | null = null;
let lastRefreshAt = 0;

async function tryRefresh(): Promise<boolean> {
  const { refreshToken, updateTokens, clearAuth } = useAuthStore.getState();
  if (!refreshToken) return false;

  try {
    const res = await fetch(`${API_BASE}/api/tenant/refresh`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ refresh_token: refreshToken }),
    });
    const data = await res.json().catch(() => null);
    if (res.ok && data?.token && data?.refresh_token) {
      updateTokens(data.token, data.refresh_token);
      lastRefreshAt = Date.now();
      return true;
    }
  } catch { /* refresh failed */ }

  clearAuth();
  return false;
}

function scheduleRefresh(): Promise<boolean> {
  // Cooldown: skip if refreshed within 5s (prevent double-rotate)
  if (Date.now() - lastRefreshAt < 5000) {
    return Promise.resolve(true);
  }
  if (!refreshPromise) {
    refreshPromise = tryRefresh().finally(() => { refreshPromise = null; });
  }
  return refreshPromise;
}

function parseJsonError(data: Record<string, unknown> | null, statusText: string): { msg: string; code: number | null } {
  const msg = (data?.error ?? data?.message ?? statusText) as string;
  const code = typeof data?.code === 'number' ? data.code : null;
  return { msg, code };
}

export async function request<T>(
  method: string,
  path: string,
  body?: unknown,
  token?: string,
): Promise<T> {
  const headers: Record<string, string> = { 'Content-Type': 'application/json' };
  if (token) headers['Authorization'] = `Bearer ${token}`;

  const res = await fetch(`${API_BASE}${path}`, {
    method,
    headers,
    body: body ? JSON.stringify(body) : undefined,
  });

  // Parse body once (before 401 branch, to avoid "body already consumed")
  const data = await res.json().catch(() => null);

  if (res.status === 401 && token) {
    const refreshed = await scheduleRefresh();
    if (refreshed) {
      const newToken = useAuthStore.getState().token;
      const retryHeaders: Record<string, string> = { 'Content-Type': 'application/json' };
      if (newToken) retryHeaders['Authorization'] = `Bearer ${newToken}`;

      const retryRes = await fetch(`${API_BASE}${path}`, {
        method,
        headers: retryHeaders,
        body: body ? JSON.stringify(body) : undefined,
      });
      const retryData = await retryRes.json().catch(() => null);
      if (!retryRes.ok) {
        const { msg, code } = parseJsonError(retryData, retryRes.statusText);
        throw new ApiError(retryRes.status, msg, code);
      }
      return retryData as T;
    }
  }

  if (!res.ok) {
    const { msg, code } = parseJsonError(data, res.statusText);
    throw new ApiError(res.status, msg, code);
  }

  return data as T;
}

export async function requestFormData<T>(
  path: string,
  formData: FormData,
  token?: string,
): Promise<T> {
  const headers: Record<string, string> = {};
  if (token) headers['Authorization'] = `Bearer ${token}`;

  const res = await fetch(`${API_BASE}${path}`, {
    method: 'POST',
    headers,
    body: formData,
  });

  const data = await res.json().catch(() => null);

  if (res.status === 401 && token) {
    const refreshed = await scheduleRefresh();
    if (refreshed) {
      const newToken = useAuthStore.getState().token;
      const retryHeaders: Record<string, string> = {};
      if (newToken) retryHeaders['Authorization'] = `Bearer ${newToken}`;

      const retryRes = await fetch(`${API_BASE}${path}`, {
        method: 'POST',
        headers: retryHeaders,
        body: formData,
      });
      const retryData = await retryRes.json().catch(() => null);
      if (!retryRes.ok || retryData?.success === false) {
        const msg = retryData?.error ?? retryRes.statusText;
        const code = retryData?.error_code ?? null;
        throw new ApiError(retryRes.status, msg, code);
      }
      return retryData as T;
    }
  }

  if (!res.ok || data?.success === false) {
    const msg = data?.error ?? res.statusText;
    const code = data?.error_code ?? null;
    throw new ApiError(res.status, msg, code);
  }

  return data as T;
}
