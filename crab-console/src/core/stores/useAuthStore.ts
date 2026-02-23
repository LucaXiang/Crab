import { create } from 'zustand';

const STORAGE_KEY = 'redcoral-auth';

interface AuthState {
  token: string | null;
  tenantId: string | null;
}

interface AuthStore extends AuthState {
  isAuthenticated: boolean;
  setAuth: (token: string, tenantId: string) => void;
  clearAuth: () => void;
}

function loadAuth(): AuthState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return { token: parsed.token ?? null, tenantId: parsed.tenant_id ?? null };
    }
  } catch { /* ignore */ }
  return { token: null, tenantId: null };
}

const initial = loadAuth();

export const useAuthStore = create<AuthStore>()((set) => ({
  token: initial.token,
  tenantId: initial.tenantId,
  isAuthenticated: !!initial.token,

  setAuth: (token, tenantId) => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ token, tenant_id: tenantId }));
    set({ token, tenantId, isAuthenticated: true });
  },

  clearAuth: () => {
    localStorage.removeItem(STORAGE_KEY);
    set({ token: null, tenantId: null, isAuthenticated: false });
  },
}));
