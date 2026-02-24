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

  const data = await res.json().catch(() => null);

  if (!res.ok) {
    const msg = data?.error ?? data?.message ?? res.statusText;
    const code = typeof data?.code === 'number' ? data.code : null;
    throw new ApiError(res.status, msg, code);
  }

  return data as T;
}

export async function requestFormData<T>(
  path: string,
  formData: FormData,
): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    method: 'POST',
    body: formData,
  });

  const data = await res.json().catch(() => null);

  if (!res.ok || data?.success === false) {
    const msg = data?.error ?? res.statusText;
    const code = data?.error_code ?? null;
    throw new ApiError(res.status, msg, code);
  }

  return data as T;
}
