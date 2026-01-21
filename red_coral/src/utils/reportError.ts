import { invoke } from '@tauri-apps/api/core';

interface ReportErrorOptions {
  source?: string;
  userActionOverride?: string | null;
  extras?: Record<string, unknown>;
}

export async function reportError(
  message: string,
  error: unknown,
  context?: string,
  options?: ReportErrorOptions
): Promise<void> {
  const err = error instanceof Error ? error : new Error(String(error));

  let authState: { user?: { id?: number; username?: string; role_name?: string } | null } | null = null;
  let checkoutState: { currentOrderKey?: string | null; checkoutOrder?: { key?: string; receipt_number?: string; table_name?: string; zone_name?: string } | null } | null = null;

  try {
    const { useAuthStore } = await import('@/core/stores/auth/useAuthStore');
    authState = useAuthStore.getState();
  } catch {
    // Store not available
  }

  try {
    const { useCheckoutStore } = await import('@/core/stores/order/useCheckoutStore');
    checkoutState = useCheckoutStore.getState();
  } catch {
    // Store not available
  }

  const activeOrderKey = checkoutState?.currentOrderKey ?? checkoutState?.checkoutOrder?.key ?? null;
  const receiptNumber = checkoutState?.checkoutOrder?.receipt_number ?? null;
  const tableName = checkoutState?.checkoutOrder?.table_name ?? null;
  const zone_name = checkoutState?.checkoutOrder?.zone_name ?? null;

  const payload: Record<string, unknown> = {
    source: options?.source ?? 'frontend',
    message,
    stack: err.stack ?? null,
    route: typeof window !== 'undefined' ? window.location.pathname : null,
    user_action: options?.userActionOverride ?? context ?? null,
    user_id: authState?.user?.id ?? null,
    username: authState?.user?.username ?? null,
    role: authState?.user?.role_name ?? null,
    order_key: activeOrderKey,
    receipt_number: receiptNumber,
    table_name: tableName,
    zone_name: zone_name,
  };

  if (options?.extras) {
    Object.assign(payload, options.extras);
  }

  try {
    await invoke('record_error_cmd', { payload });
  } catch {}
}
