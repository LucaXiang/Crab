import { request } from './client';

export interface AuditEntry {
  id: number;
  action: string;
  detail: Record<string, unknown> | null;
  ip_address: string | null;
  created_at: number;
}

export function getAuditLog(
  token: string,
  page: number = 1,
  perPage: number = 20,
): Promise<AuditEntry[]> {
  return request('GET', `/api/tenant/audit-log?page=${page}&per_page=${perPage}`, undefined, token);
}
