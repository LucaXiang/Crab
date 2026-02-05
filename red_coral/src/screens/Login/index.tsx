import React, { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { User, Lock, AlertCircle, ChevronRight, Store, Terminal, Power, WifiOff, Building2, Server, Monitor } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useBridgeStore, useAppState, AppStateHelpers, type LoginMode } from '@/core/stores/bridge';
import { useI18n } from '@/hooks/useI18n';

interface LocationState {
  from?: { pathname: string };
}

export const LoginScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const location = useLocation();

  // 同步登录状态到 AuthStore
  const setAuthUser = useAuthStore((state) => state.setUser);
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);

  // Bridge store for actual login
  const {
    currentSession,
    modeInfo,
    loginEmployee,
    fetchModeInfo,
    fetchTenants,
    checkFirstRun,
    isLoading,
  } = useBridgeStore();

  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loginMode, setLoginMode] = useState<LoginMode | null>(null);
  const [focusedField, setFocusedField] = useState<'username' | 'password' | null>(null);

  // Tenant check state
  const [isCheckingTenants, setIsCheckingTenants] = useState(true);

  // Check if we should be here - redirect if no tenants or subscription blocked
  useEffect(() => {
    const checkAccess = async () => {
      const isFirst = await checkFirstRun();
      await fetchTenants();
      const currentTenants = useBridgeStore.getState().tenants;

      // If no tenants, redirect to activate
      if (isFirst || currentTenants.length === 0) {
        navigate('/activate', { replace: true });
        return;
      }

      // 检查订阅状态 — 被阻止时不应停留在登录页
      await useBridgeStore.getState().fetchAppState();
      const currentState = useBridgeStore.getState().appState;
      if (AppStateHelpers.isSubscriptionBlocked(currentState)) {
        navigate(AppStateHelpers.getRouteForState(currentState), { replace: true });
        return;
      }

      setIsCheckingTenants(false);
    };
    checkAccess();
  }, [checkFirstRun, fetchTenants, navigate]);

  // Fetch mode info on mount
  useEffect(() => {
    fetchModeInfo();
  }, [fetchModeInfo]);

  // Use appState for navigation decisions (consistent with ProtectedRoute)
  const appState = useAppState();

  // Navigate when authenticated (check both appState AND isAuthenticated)
  useEffect(() => {
    if (AppStateHelpers.canAccessPOS(appState) && isAuthenticated) {
      const from = (location.state as LocationState)?.from?.pathname || '/pos';
      navigate(from, { replace: true });
    }
  }, [appState, isAuthenticated, navigate, location]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoginMode(null);

    if (!username.trim() || !password.trim()) {
      setError(t('auth.login.error.empty_fields'));
      return;
    }

    try {
      // Use unified loginEmployee for both Server and Client modes
      // - Server mode: In-Process communication via CrabClient
      // - Client mode: mTLS HTTP communication via CrabClient
      const response = await loginEmployee(username, password);

      if (response.success && response.session) {
        setLoginMode(response.mode);

        const userInfo = response.session.user_info;

        // 同步登录状态到 AuthStore
        setAuthUser({
          id: userInfo.id,
          username: userInfo.username,
          display_name: userInfo.display_name,
          role_id: userInfo.role_id,
          role_name: userInfo.role_name,
          permissions: userInfo.permissions,
          is_system: userInfo.is_system,
          is_active: userInfo.is_active,
          created_at: userInfo.created_at,
        });

        // Navigation handled by useEffect
      } else {
        setError(response.error || t('auth.login.error.invalid_credentials'));
      }
    } catch (err: unknown) {
      setError(err instanceof Error ? err.message : t('auth.login.error.invalid_credentials'));
    }
  };

  const handleCloseApp = async () => {
    try {
      const appWindow = getCurrentWindow();
      await appWindow.close();
    } catch (error) {
      console.error('Failed to close window:', error);
    }
  };

  const isDisconnected = modeInfo?.mode === 'Disconnected';

  // Show loading while checking tenants
  if (isCheckingTenants) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50 relative">
        <button
          onClick={handleCloseApp}
          className="absolute top-6 right-6 p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors z-20"
          title={t('common.dialog.close_app')}
        >
          <Power size={24} />
        </button>
        <div className="w-8 h-8 border-4 border-primary-500/30 border-t-primary-500 rounded-full animate-spin" />
      </div>
    );
  }

  return (
    <div className="min-h-screen w-full flex font-sans overflow-hidden bg-gray-50">
      {/* Left Side - Brand & Aesthetic */}
      <div className="hidden lg:flex lg:w-1/2 relative bg-primary-500 overflow-hidden items-center justify-center p-12 text-white">
        {/* Abstract Background Patterns */}
        <div className="absolute top-0 left-0 w-full h-full overflow-hidden z-0">
          <div className="absolute top-[-10%] left-[-10%] w-[37.5rem] h-[37.5rem] rounded-full bg-white/5 blur-[6.25rem]" />
          <div className="absolute bottom-[-10%] right-[-10%] w-[31.25rem] h-[31.25rem] rounded-full bg-black/10 blur-[5rem]" />
          <div className="absolute top-[40%] right-[20%] w-[18.75rem] h-[18.75rem] rounded-full bg-orange-400/20 blur-[3.75rem]" />
        </div>

        {/* Brand Content */}
        <div className="relative z-10 max-w-lg">
          <div className="flex items-center gap-4 mb-8">
            <div className="w-16 h-16 bg-white/20 backdrop-blur-md rounded-2xl flex items-center justify-center shadow-inner border border-white/30">
              <span className="text-4xl">🐚</span>
            </div>
            <h1 className="text-5xl font-bold tracking-tight">{t('app.brand.name')}</h1>
          </div>
          
          <h2 className="text-3xl font-light mb-6 text-white/90">
            {t('auth.login.subtitle')}
          </h2>

          <p className="text-lg text-white/70 leading-relaxed mb-12">
            {t('auth.login.subtitle_desc')}
          </p>

          {/* Feature Highlights */}
          <div className="space-y-4">
            <div className="flex items-center gap-4 p-4 rounded-xl bg-white/10 backdrop-blur-sm border border-white/10 transition-transform hover:translate-x-2">
              <Store className="text-white/80" />
              <div>
                <h3 className="font-semibold">{t('auth.login.feature.multi_zone')}</h3>
                <p className="text-sm text-white/60">{t('auth.login.feature.multi_zone_desc')}</p>
              </div>
            </div>
            <div className="flex items-center gap-4 p-4 rounded-xl bg-white/10 backdrop-blur-sm border border-white/10 transition-transform hover:translate-x-2">
              <Terminal className="text-white/80" />
              <div>
                <h3 className="font-semibold">{t('auth.login.feature.fast_checkout')}</h3>
                <p className="text-sm text-white/60">{t('auth.login.feature.fast_checkout_desc')}</p>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Right Side - Login Form */}
      <div className="w-full lg:w-1/2 flex items-center justify-center p-8 relative">
        <button
          onClick={handleCloseApp}
          className="absolute top-6 right-6 p-2 text-gray-400 hover:text-red-500 hover:bg-red-50 rounded-full transition-colors z-20"
          title={t('common.dialog.close_app')}
        >
          <Power size={24} />
        </button>

        <div className="w-full max-w-md space-y-8">
          
          {/* Mobile Header (only visible on small screens) */}
          <div className="lg:hidden text-center mb-8">
            <div className="inline-flex items-center justify-center w-16 h-16 bg-primary-500 rounded-2xl shadow-lg mb-4 text-white">
              <span className="text-3xl">🐚</span>
            </div>
            <h1 className="text-2xl font-bold text-gray-900">{t('app.brand.full_name')}</h1>
          </div>

          {/* Mode/Tenant Info Card */}
          <div className="bg-white rounded-2xl border border-gray-100 shadow-sm p-5 space-y-4">
            {/* Header: Tenant Info */}
            <div className="flex items-start justify-between">
              <div className="flex items-center gap-3">
                <div className={`p-2.5 rounded-xl ${
                  modeInfo?.tenant_id ? 'bg-orange-50 text-primary-500' : 'bg-gray-50 text-gray-400'
                }`}>
                  {modeInfo?.tenant_id ? <Building2 size={24} /> : <WifiOff size={24} />}
                </div>
                <div>
                  <h3 className="font-semibold text-gray-900">
                    {modeInfo?.tenant_id || t('auth.login.tenant_no_tenant')}
                  </h3>
                  <p className="text-xs text-gray-500">
                    {modeInfo?.tenant_id ? t('auth.login.tenant_active') : t('auth.login.tenant_select_prompt')}
                  </p>
                </div>
              </div>
              <button
                type="button"
                onClick={() => navigate('/setup', { replace: true })}
                className="px-3 py-1.5 text-xs font-medium text-primary-500 bg-primary-500/10 hover:bg-primary-500/20 rounded-lg transition-colors"
              >
                {isDisconnected ? t('auth.login.button_setup') : t('auth.login.button_switch')}
              </button>
            </div>

            {/* Status Grid */}
            <div className="grid grid-cols-2 gap-3 pt-2 border-t border-gray-50">
              {/* Mode Status */}
              <div className="bg-gray-50 rounded-lg p-3">
                <p className="text-xs text-gray-500 mb-1">{t('auth.login.mode_current')}</p>
                <div className="flex items-center gap-2">
                  {modeInfo?.mode === 'Server' ? (
                    <Server size={16} className="text-green-600" />
                  ) : modeInfo?.mode === 'Client' ? (
                    <Monitor size={16} className="text-blue-600" />
                  ) : (
                    <WifiOff size={16} className="text-gray-400" />
                  )}
                  <span className={`text-sm font-medium ${
                    modeInfo?.mode === 'Server' ? 'text-green-700' :
                    modeInfo?.mode === 'Client' ? 'text-blue-700' :
                    'text-gray-600'
                  }`}>
                    {modeInfo?.mode || t('auth.login.mode_disconnected')}
                  </span>
                </div>
              </div>

              {/* Connection Status */}
              <div className="bg-gray-50 rounded-lg p-3">
                <p className="text-xs text-gray-500 mb-1">{t('auth.login.connection')}</p>
                <div className="flex items-center gap-2">
                  <div className={`w-2 h-2 rounded-full ${
                    modeInfo?.is_connected ? 'bg-green-500 animate-pulse' : 'bg-red-500'
                  }`} />
                  <span className={`text-sm font-medium ${
                    modeInfo?.is_connected ? 'text-green-700' : 'text-red-700'
                  }`}>
                    {modeInfo?.is_connected ? t('auth.login.connection_online') : t('auth.login.connection_offline')}
                  </span>
                </div>
              </div>
            </div>
          </div>

          <div className="space-y-2">
            <h2 className="text-3xl font-bold text-gray-900">
              {t('auth.login.title')}
            </h2>
            <p className="text-gray-500">
              {t('auth.login.enter_details')}
            </p>
          </div>

          <form onSubmit={handleSubmit} className="space-y-6 mt-8">
            {/* Username Input */}
            <div className="space-y-1">
              <label
                htmlFor="username"
                className={`text-sm font-medium transition-colors ${
                  focusedField === 'username' ? 'text-primary-500' : 'text-gray-700'
                }`}
              >
                {t('auth.login.username')}
              </label>
              <div className={`
                relative flex items-center transition-all duration-200 border rounded-xl bg-white
                ${focusedField === 'username'
                  ? 'border-primary-500 ring-4 ring-primary-500/10 shadow-sm'
                  : 'border-gray-200 hover:border-gray-300'}
              `}>
                <div className="pl-4 text-gray-400">
                  <User size={20} />
                </div>
                <input
                  id="username"
                  type="text"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  onFocus={() => setFocusedField('username')}
                  onBlur={() => setFocusedField(null)}
                  placeholder={t('auth.login.username_placeholder')}
                  className="w-full px-4 py-3.5 bg-transparent focus:outline-none text-gray-900 placeholder-gray-400"
                  disabled={isLoading}
                />
              </div>
            </div>

            {/* Password Input */}
            <div className="space-y-1">
              <div className="flex items-center justify-between">
                <label
                  htmlFor="password"
                  className={`text-sm font-medium transition-colors ${
                    focusedField === 'password' ? 'text-primary-500' : 'text-gray-700'
                  }`}
                >
                  {t('auth.login.password')}
                </label>
                {/* Optional: Forgot Password Link */}
                {/* <button type="button" className="text-sm text-primary-500 hover:underline">
                  Forgot password?
                </button> */}
              </div>
              <div className={`
                relative flex items-center transition-all duration-200 border rounded-xl bg-white
                ${focusedField === 'password'
                  ? 'border-primary-500 ring-4 ring-primary-500/10 shadow-sm'
                  : 'border-gray-200 hover:border-gray-300'}
              `}>
                <div className="pl-4 text-gray-400">
                  <Lock size={20} />
                </div>
                <input
                  id="password"
                  type="password"
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  onFocus={() => setFocusedField('password')}
                  onBlur={() => setFocusedField(null)}
                  placeholder={t('auth.login.password_placeholder')}
                  className="w-full px-4 py-3.5 bg-transparent focus:outline-none text-gray-900 placeholder-gray-400"
                  disabled={isLoading}
                />
              </div>
            </div>

            {/* Error Message */}
            {error && (
              <div className="animate-in fade-in slide-in-from-top-2 duration-300 flex items-center gap-3 text-red-600 bg-red-50 p-4 rounded-xl border border-red-100">
                <AlertCircle size={20} className="shrink-0" />
                <span className="text-sm font-medium">{error}</span>
              </div>
            )}

            {/* Submit Button */}
            <button
              type="submit"
              disabled={isLoading}
              className="group w-full py-4 bg-primary-500 text-white font-bold rounded-xl hover:bg-primary-600 active:scale-[0.98] transition-all shadow-lg shadow-primary-500/25 flex items-center justify-center gap-2 disabled:opacity-70 disabled:cursor-not-allowed disabled:active:scale-100"
            >
              {isLoading ? (
                <div className="w-6 h-6 border-2 border-white/30 border-t-white rounded-full animate-spin" />
              ) : (
                <>
                  <span>{t('auth.login.submit')}</span>
                  <ChevronRight size={20} className="group-hover:translate-x-1 transition-transform" />
                </>
              )}
            </button>
          </form>
        </div>

        {/* Footer Copyright */}
        <div className="absolute bottom-6 text-center w-full text-xs text-gray-400">
          {t('app.copyright', { year: new Date().getFullYear() })}
        </div>
      </div>
    </div>
  );
};
