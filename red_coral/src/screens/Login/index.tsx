import React, { useState, useEffect } from 'react';
import { useNavigate, useLocation } from 'react-router-dom';
import { User, Lock, AlertCircle, ChevronRight, Store, Terminal, Power, Wifi, WifiOff, Building2 } from 'lucide-react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';
import { useBridgeStore, type LoginMode } from '@/core/stores/bridge';
import { useI18n } from '@/hooks/useI18n';

export const LoginScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const location = useLocation();

  // Auth store for compatibility with ProtectedRoute
  const setAuthUser = useAuthStore((state) => state.setUser);
  const isAuthenticated = useAuthStore((state) => state.isAuthenticated);

  // Bridge store for actual login
  const {
    currentSession,
    modeInfo,
    loginEmployee,
    loginAuto,
    fetchModeInfo,
    isLoading,
  } = useBridgeStore();

  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState('');
  const [loginMode, setLoginMode] = useState<LoginMode | null>(null);
  const [focusedField, setFocusedField] = useState<'username' | 'password' | null>(null);

  // Fetch mode info on mount
  useEffect(() => {
    fetchModeInfo();
  }, [fetchModeInfo]);

  // Navigate when authenticated
  useEffect(() => {
    if (isAuthenticated || currentSession) {
      const from = (location.state as any)?.from?.pathname || '/pos';
      navigate(from, { replace: true });
    }
  }, [isAuthenticated, currentSession, navigate, location]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoginMode(null);

    if (!username.trim() || !password.trim()) {
      setError(t('auth.login.error.emptyFields'));
      return;
    }

    try {
      // Use appropriate login method based on mode
      // Server mode: use unified loginEmployee (In-Process)
      // Client mode: use loginAuto (mTLS HTTP)
      let response;
      if (modeInfo?.mode === 'Server') {
        response = await loginEmployee(username, password);
      } else {
        // TODO: Get edge URL from config
        const edgeUrl = 'https://localhost:9625';
        response = await loginAuto(username, password, edgeUrl);
      }

      if (response.success && response.session) {
        setLoginMode(response.mode);

        // Update auth store for ProtectedRoute compatibility
        // Adapt from shared::client::UserInfo to legacy User format
        setAuthUser({
          id: parseInt(response.session.user_info.id) || 0,
          uuid: response.session.user_info.id,
          username: response.session.user_info.username,
          display_name: response.session.user_info.username, // Fallback to username
          password_hash: '',
          role_id: 0, // Role info now in permissions array
          avatar: null,
          is_active: true,
          created_at: new Date().toISOString(),
          updated_at: new Date().toISOString(),
        });

        // Navigation handled by useEffect
      } else {
        setError(response.error || t('auth.login.error.invalidCredentials'));
      }
    } catch (err: any) {
      setError(err.message || t('auth.login.error.invalidCredentials'));
    }
  };

  const handleSwitchTenant = () => {
    navigate('/tenant-select', { replace: true });
  };

  const handleCloseApp = async () => {
    const appWindow = getCurrentWindow();
    await appWindow.close();
  };

  return (
    <div className="min-h-screen w-full flex font-sans overflow-hidden bg-gray-50">
      {/* Left Side - Brand & Aesthetic */}
      <div className="hidden lg:flex lg:w-1/2 relative bg-[#FF5E5E] overflow-hidden items-center justify-center p-12 text-white">
        {/* Abstract Background Patterns */}
        <div className="absolute top-0 left-0 w-full h-full overflow-hidden z-0">
          <div className="absolute top-[-10%] left-[-10%] w-[600px] h-[600px] rounded-full bg-white/5 blur-[100px]" />
          <div className="absolute bottom-[-10%] right-[-10%] w-[500px] h-[500px] rounded-full bg-black/10 blur-[80px]" />
          <div className="absolute top-[40%] right-[20%] w-[300px] h-[300px] rounded-full bg-orange-400/20 blur-[60px]" />
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
            {t('auth.login.subtitleDesc')}
          </p>

          {/* Feature Highlights */}
          <div className="space-y-4">
            <div className="flex items-center gap-4 p-4 rounded-xl bg-white/10 backdrop-blur-sm border border-white/10 transition-transform hover:translate-x-2">
              <Store className="text-white/80" />
              <div>
                <h3 className="font-semibold">{t('auth.login.feature.multiZone')}</h3>
                <p className="text-sm text-white/60">{t('auth.login.feature.multiZoneDesc')}</p>
              </div>
            </div>
            <div className="flex items-center gap-4 p-4 rounded-xl bg-white/10 backdrop-blur-sm border border-white/10 transition-transform hover:translate-x-2">
              <Terminal className="text-white/80" />
              <div>
                <h3 className="font-semibold">{t('auth.login.feature.fastCheckout')}</h3>
                <p className="text-sm text-white/60">{t('auth.login.feature.fastCheckoutDesc')}</p>
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
          title={t('common.closeApp')}
        >
          <Power size={24} />
        </button>

        <div className="w-full max-w-md space-y-8">
          
          {/* Mobile Header (only visible on small screens) */}
          <div className="lg:hidden text-center mb-8">
            <div className="inline-flex items-center justify-center w-16 h-16 bg-[#FF5E5E] rounded-2xl shadow-lg mb-4 text-white">
              <span className="text-3xl">🐚</span>
            </div>
            <h1 className="text-2xl font-bold text-gray-900">{t('app.brand.fullName')}</h1>
          </div>

          {/* Tenant Info */}
          {modeInfo?.tenant_id && (
            <div className="flex items-center justify-between p-4 bg-gray-100 rounded-xl">
              <div className="flex items-center gap-3">
                <Building2 size={20} className="text-gray-500" />
                <div>
                  <p className="text-sm font-medium text-gray-900">{modeInfo.tenant_id}</p>
                  <p className="text-xs text-gray-500 flex items-center gap-1">
                    {modeInfo.mode === 'Server' ? (
                      <><Wifi size={12} /> Server Mode</>
                    ) : modeInfo.mode === 'Client' ? (
                      <><Wifi size={12} /> Client Mode</>
                    ) : (
                      <><WifiOff size={12} /> Disconnected</>
                    )}
                  </p>
                </div>
              </div>
              <button
                type="button"
                onClick={handleSwitchTenant}
                className="text-sm text-[#FF5E5E] hover:underline"
              >
                Switch
              </button>
            </div>
          )}

          <div className="space-y-2">
            <h2 className="text-3xl font-bold text-gray-900">
              {t('auth.login.title')}
            </h2>
            <p className="text-gray-500">
              {t('auth.login.enterDetails')}
            </p>
          </div>

          <form onSubmit={handleSubmit} className="space-y-6 mt-8">
            {/* Username Input */}
            <div className="space-y-1">
              <label
                htmlFor="username"
                className={`text-sm font-medium transition-colors ${
                  focusedField === 'username' ? 'text-[#FF5E5E]' : 'text-gray-700'
                }`}
              >
                {t('auth.login.username')}
              </label>
              <div className={`
                relative flex items-center transition-all duration-200 border rounded-xl bg-white
                ${focusedField === 'username'
                  ? 'border-[#FF5E5E] ring-4 ring-[#FF5E5E]/10 shadow-sm'
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
                  placeholder={t('auth.login.usernamePlaceholder')}
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
                    focusedField === 'password' ? 'text-[#FF5E5E]' : 'text-gray-700'
                  }`}
                >
                  {t('auth.login.password')}
                </label>
                {/* Optional: Forgot Password Link */}
                {/* <button type="button" className="text-sm text-[#FF5E5E] hover:underline">
                  Forgot password?
                </button> */}
              </div>
              <div className={`
                relative flex items-center transition-all duration-200 border rounded-xl bg-white
                ${focusedField === 'password'
                  ? 'border-[#FF5E5E] ring-4 ring-[#FF5E5E]/10 shadow-sm'
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
                  placeholder={t('auth.login.passwordPlaceholder')}
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
              className="group w-full py-4 bg-[#FF5E5E] text-white font-bold rounded-xl hover:bg-[#E54545] active:scale-[0.98] transition-all shadow-lg shadow-[#FF5E5E]/25 flex items-center justify-center gap-2 disabled:opacity-70 disabled:cursor-not-allowed disabled:active:scale-100"
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
