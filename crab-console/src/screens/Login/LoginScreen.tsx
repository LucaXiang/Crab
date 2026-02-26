import React, { useState } from 'react';
import { Link, Navigate, useNavigate } from 'react-router-dom';
import { Mail, Lock, ArrowRight, BarChart3, ShieldCheck, Zap, Globe } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { login } from '@/infrastructure/api/auth';
import { ApiError } from '@/infrastructure/api/client';
import { apiErrorMessage } from '@/infrastructure/i18n';
import { Spinner } from '@/presentation/components/ui/Spinner';
import { LogoIcon } from '@/presentation/components/layout/Logo';

export const LoginScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const setAuth = useAuthStore(s => s.setAuth);
  const isAuthenticated = useAuthStore(s => s.isAuthenticated);

  const [email, setEmail] = useState('');
  const [password, setPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');

  if (isAuthenticated) {
    return <Navigate to="/" replace />;
  }

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      const res = await login(email, password);
      setAuth(res.token, res.refresh_token, res.tenant_id);
      navigate('/');
    } catch (err) {
      if (err instanceof ApiError) {
        setError(apiErrorMessage(t, err.code, err.message, err.status));
      } else {
        setError(t('auth.error_generic'));
      }
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="min-h-screen relative overflow-hidden" style={{ background: 'linear-gradient(168deg, #0c0f1a 0%, #111827 50%, #0c0f1a 100%)' }}>
      {/* Dot grid texture */}
      <div className="absolute inset-0 opacity-40" style={{ backgroundImage: 'radial-gradient(circle at 1px 1px, rgba(255,255,255,0.04) 1px, transparent 0)', backgroundSize: '32px 32px' }} />

      {/* Radial glow overlays */}
      <div className="absolute inset-0" style={{ background: 'radial-gradient(ellipse 60% 50% at 30% 20%, rgba(255,94,94,0.12) 0%, transparent 70%)' }} />
      <div className="absolute inset-0" style={{ background: 'radial-gradient(ellipse 40% 50% at 75% 80%, rgba(139,92,246,0.06) 0%, transparent 70%)' }} />

      {/* Floating accent elements */}
      <div className="absolute top-[15%] left-[8%] w-14 h-14 rounded-2xl bg-slate-800/60 backdrop-blur-sm border border-white/8 flex items-center justify-center animate-bounce" style={{ animationDuration: '4s', animationDelay: '0s' }}>
        <BarChart3 className="w-6 h-6 text-primary-400/70" />
      </div>
      <div className="absolute top-[25%] right-[10%] w-12 h-12 rounded-xl bg-slate-800/60 backdrop-blur-sm border border-white/8 flex items-center justify-center animate-bounce" style={{ animationDuration: '5s', animationDelay: '1s' }}>
        <ShieldCheck className="w-5 h-5 text-emerald-400/70" />
      </div>
      <div className="absolute bottom-[20%] left-[12%] w-11 h-11 rounded-xl bg-slate-800/60 backdrop-blur-sm border border-white/8 flex items-center justify-center animate-bounce" style={{ animationDuration: '3.5s', animationDelay: '0.5s' }}>
        <Zap className="w-5 h-5 text-amber-400/70" />
      </div>
      <div className="absolute bottom-[30%] right-[8%] w-12 h-12 rounded-xl bg-slate-800/60 backdrop-blur-sm border border-white/8 flex items-center justify-center animate-bounce" style={{ animationDuration: '4.5s', animationDelay: '1.5s' }}>
        <Globe className="w-5 h-5 text-purple-400/70" />
      </div>

      {/* Main content */}
      <div className="relative z-10 min-h-screen flex flex-col items-center justify-center px-4">
        {/* Logo + Brand */}
        <div className="text-center mb-10">
          <div className="mx-auto mb-5 w-16 h-16 bg-gradient-to-br from-primary-400 to-primary-600 rounded-2xl flex items-center justify-center shadow-lg shadow-primary-500/25">
            <LogoIcon className="w-8 h-8" />
          </div>
          <h1 className="text-3xl md:text-4xl font-extrabold text-white tracking-tight">
            Red<span className="text-primary-400">Coral</span>
          </h1>
          <p className="text-sm text-slate-400 mt-2 tracking-wide">{t('auth.login_subtitle')}</p>
        </div>

        {/* Glass card */}
        <div className="w-full max-w-sm">
          <div className="rounded-2xl border border-white/10 bg-white/[0.04] backdrop-blur-xl p-7 shadow-2xl shadow-black/40">
            {error && (
              <div className="mb-5 p-3 bg-red-500/10 border border-red-500/20 rounded-xl text-sm text-red-300">{error}</div>
            )}

            <form className="space-y-5" onSubmit={handleSubmit}>
              <div>
                <label htmlFor="email" className="block text-sm font-medium text-slate-300 mb-2">{t('auth.email')}</label>
                <div className="relative group">
                  <div className="absolute left-3.5 top-1/2 -translate-y-1/2 w-8 h-8 flex items-center justify-center text-slate-500 group-focus-within:text-primary-400 transition-colors">
                    <Mail className="w-5 h-5" />
                  </div>
                  <input
                    type="email"
                    id="email"
                    name="email"
                    autoComplete="email"
                    required
                    value={email}
                    onChange={e => setEmail(e.target.value)}
                    placeholder={t('auth.email')}
                    className="w-full pl-12 pr-4 py-3.5 bg-white/[0.06] border border-white/10 rounded-xl text-base text-white placeholder:text-slate-500 focus:outline-none focus:ring-2 focus:ring-primary-500/30 focus:border-primary-500/50 transition-all duration-200"
                  />
                </div>
              </div>

              <div>
                <label htmlFor="password" className="block text-sm font-medium text-slate-300 mb-2">{t('auth.password')}</label>
                <div className="relative group">
                  <div className="absolute left-3.5 top-1/2 -translate-y-1/2 w-8 h-8 flex items-center justify-center text-slate-500 group-focus-within:text-primary-400 transition-colors">
                    <Lock className="w-5 h-5" />
                  </div>
                  <input
                    type="password"
                    id="password"
                    name="password"
                    autoComplete="current-password"
                    required
                    value={password}
                    onChange={e => setPassword(e.target.value)}
                    placeholder={t('auth.password')}
                    className="w-full pl-12 pr-4 py-3.5 bg-white/[0.06] border border-white/10 rounded-xl text-base text-white placeholder:text-slate-500 focus:outline-none focus:ring-2 focus:ring-primary-500/30 focus:border-primary-500/50 transition-all duration-200"
                  />
                </div>
              </div>

              <button
                type="submit"
                disabled={loading}
                className="w-full bg-gradient-to-r from-primary-500 to-primary-600 hover:from-primary-400 hover:to-primary-500 text-white font-semibold py-3.5 rounded-xl transition-all duration-200 cursor-pointer flex items-center justify-center gap-2 disabled:opacity-50 disabled:cursor-not-allowed shadow-lg shadow-primary-500/20 hover:shadow-xl hover:shadow-primary-500/30"
              >
                {loading ? (
                  <>
                    <Spinner />
                    <span>{t('auth.loading')}</span>
                  </>
                ) : (
                  <>
                    <span>{t('auth.login_cta')}</span>
                    <ArrowRight className="w-4 h-4" />
                  </>
                )}
              </button>
            </form>

            <div className="text-center mt-5">
              <Link to="/forgot-password" className="text-sm text-slate-500 hover:text-primary-400 transition-colors duration-150">
                {t('auth.forgot')}
              </Link>
            </div>
          </div>

          <p className="text-center text-sm text-slate-500 mt-6">
            {t('auth.no_account')}{' '}
            <a href="https://redcoral.app" className="text-primary-400 hover:text-primary-300 font-medium transition-colors duration-150">
              {t('auth.register')}
            </a>
          </p>
        </div>

        {/* Footer */}
        <div className="mt-12 flex items-center gap-3 text-xs text-slate-600">
          <div className="flex items-center gap-1.5">
            <div className="w-1.5 h-1.5 rounded-full bg-emerald-500 animate-pulse" />
            <span>System Online</span>
          </div>
          <span className="text-slate-700">·</span>
          <span>RedCoral Console</span>
        </div>
      </div>
    </div>
  );
};
