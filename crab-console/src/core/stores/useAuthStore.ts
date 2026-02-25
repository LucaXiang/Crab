import { create } from 'zustand';

const STORAGE_KEY = 'redcoral-auth';

interface AuthState {
  token: string | null;
  refreshToken: string | null;
  tenantId: string | null;
}

interface AuthStore extends AuthState {
  isAuthenticated: boolean;
  setAuth: (token: string, refreshToken: string | null, tenantId: string) => void;
  updateTokens: (token: string, refreshToken: string) => void;
  clearAuth: () => void;
}

function loadAuth(): AuthState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      return {
        token: parsed.token ?? null,
        refreshToken: parsed.refresh_token || null,
        tenantId: parsed.tenant_id ?? null,
      };
    }
  } catch { /* ignore */ }
  return { token: null, refreshToken: null, tenantId: null };
}

const initial = loadAuth();

export const useAuthStore = create<AuthStore>()((set) => ({
  token: initial.token,
  refreshToken: initial.refreshToken,
  tenantId: initial.tenantId,
  isAuthenticated: !!initial.token,

  setAuth: (token, refreshToken, tenantId) => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ token, refresh_token: refreshToken, tenant_id: tenantId }));
    set({ token, refreshToken: refreshToken || null, tenantId, isAuthenticated: true });
  },

  updateTokens: (token, refreshToken) => {
    const raw = localStorage.getItem(STORAGE_KEY);
    const existing = raw ? JSON.parse(raw) : {};
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ ...existing, token, refresh_token: refreshToken }));
    set({ token, refreshToken, isAuthenticated: true });
  },

  clearAuth: () => {
    localStorage.removeItem(STORAGE_KEY);
    set({ token: null, refreshToken: null, tenantId: null, isAuthenticated: false });
  },
}));
