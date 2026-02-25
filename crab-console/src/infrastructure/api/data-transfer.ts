import { API_BASE, ApiError } from './client';

export async function exportCatalog(token: string, storeId: number): Promise<void> {
  const res = await fetch(`${API_BASE}/api/tenant/stores/${storeId}/data-transfer/export`, {
    headers: { Authorization: `Bearer ${token}` },
  });
  if (!res.ok) {
    const data = await res.json().catch(() => null);
    throw new ApiError(res.status, data?.message ?? 'Export failed', data?.code ?? null);
  }
  const blob = await res.blob();
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `catalog_export_${storeId}.zip`;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}

export async function importCatalog(token: string, storeId: number, file: File): Promise<void> {
  const res = await fetch(`${API_BASE}/api/tenant/stores/${storeId}/data-transfer/import`, {
    method: 'POST',
    headers: {
      Authorization: `Bearer ${token}`,
      'Content-Type': 'application/zip',
    },
    body: file,
  });
  const data = await res.json().catch(() => null);
  if (!res.ok) {
    throw new ApiError(res.status, data?.message ?? 'Import failed', data?.code ?? null);
  }
}
