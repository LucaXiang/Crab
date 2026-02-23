import React, { useState } from 'react';
import { Link, useNavigate } from 'react-router-dom';
import { Mail, Lock, KeyRound, ArrowLeft, ArrowRight } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { forgotPassword, resetPassword } from '@/infrastructure/api/auth';
import { ApiError } from '@/infrastructure/api/client';
import { apiErrorMessage } from '@/infrastructure/i18n';
import { Spinner } from '@/presentation/components/ui/Spinner';
import { LogoBadge } from '@/presentation/components/layout/Logo';

export const ForgotPasswordScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();

  const [email, setEmail] = useState('');
  const [code, setCode] = useState('');
  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState('');
  const [success, setSuccess] = useState('');
  const [step, setStep] = useState<'email' | 'reset'>('email');

  const handleSendCode = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setLoading(true);
    try {
      await forgotPassword(email);
      setSuccess(t('auth.reset_code_sent'));
      setStep('reset');
    } catch (err) {
      if (err instanceof ApiError) setError(apiErrorMessage(t, err.code, err.message));
      else setError(t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  };

  const handleReset = async (e: React.FormEvent) => {
    e.preventDefault();
    setError('');
    setSuccess('');

    if (newPassword !== confirmPassword) {
      setError(t('auth.password_mismatch'));
      return;
    }

    setLoading(true);
    try {
      await resetPassword(email, code, newPassword);
      setSuccess(t('auth.password_reset_success'));
      setTimeout(() => navigate('/login'), 1500);
    } catch (err) {
      if (err instanceof ApiError) setError(apiErrorMessage(t, err.code, err.message));
      else setError(t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  };

  const inputClass = "w-full pl-10 pr-4 py-2.5 bg-white border border-slate-200 rounded-lg text-sm text-slate-900 placeholder:text-slate-400 focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500 transition-all duration-150";

  return (
    <div className="min-h-screen flex items-center justify-center bg-slate-50 px-4">
      <div className="w-full max-w-sm">
        <div className="text-center mb-8">
          <div className="mx-auto mb-4 w-12 h-12 bg-primary-500 rounded-xl flex items-center justify-center">
            <LogoBadge />
          </div>
          <h1 className="text-2xl font-bold text-slate-900">{t('auth.forgot')}</h1>
          <p className="text-sm text-slate-500 mt-1">
            {step === 'email' ? t('auth.forgot_desc') : t('auth.reset_desc')}
          </p>
        </div>

        {error && <div className="mb-4 p-3 bg-red-50 border border-red-200 rounded-lg text-sm text-red-600">{error}</div>}
        {success && <div className="mb-4 p-3 bg-green-50 border border-green-200 rounded-lg text-sm text-green-600">{success}</div>}

        {step === 'email' ? (
          <form className="space-y-4" onSubmit={handleSendCode}>
            <div>
              <label htmlFor="email" className="block text-sm font-medium text-slate-700 mb-1.5">{t('auth.email')}</label>
              <div className="relative">
                <Mail className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
                <input type="email" id="email" required value={email} onChange={e => setEmail(e.target.value)} placeholder={t('auth.email')} className={inputClass} />
              </div>
            </div>
            <button type="submit" disabled={loading}
              className="w-full bg-primary-500 hover:bg-primary-600 text-white font-semibold py-3 rounded-lg transition-colors duration-150 cursor-pointer flex items-center justify-center gap-2 disabled:opacity-60 disabled:cursor-not-allowed"
            >
              {loading ? <><Spinner /><span>{t('auth.loading')}</span></> : <><span>{t('auth.send_code')}</span><ArrowRight className="w-4 h-4" /></>}
            </button>
          </form>
        ) : (
          <form className="space-y-4" onSubmit={handleReset}>
            <div>
              <label htmlFor="code" className="block text-sm font-medium text-slate-700 mb-1.5">{t('auth.reset_code')}</label>
              <div className="relative">
                <KeyRound className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
                <input type="text" id="code" required value={code} onChange={e => setCode(e.target.value)} placeholder="000000" className={inputClass} />
              </div>
            </div>
            <div>
              <label htmlFor="new-password" className="block text-sm font-medium text-slate-700 mb-1.5">{t('settings.new_password')}</label>
              <div className="relative">
                <Lock className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
                <input type="password" id="new-password" required minLength={8} value={newPassword} onChange={e => setNewPassword(e.target.value)} placeholder={t('settings.new_password')} className={inputClass} />
              </div>
            </div>
            <div>
              <label htmlFor="confirm-password" className="block text-sm font-medium text-slate-700 mb-1.5">{t('auth.password_confirm')}</label>
              <div className="relative">
                <Lock className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-slate-400" />
                <input type="password" id="confirm-password" required minLength={8} value={confirmPassword} onChange={e => setConfirmPassword(e.target.value)} placeholder={t('auth.password_confirm')} className={inputClass} />
              </div>
            </div>
            <button type="submit" disabled={loading}
              className="w-full bg-primary-500 hover:bg-primary-600 text-white font-semibold py-3 rounded-lg transition-colors duration-150 cursor-pointer flex items-center justify-center gap-2 disabled:opacity-60 disabled:cursor-not-allowed"
            >
              {loading ? <><Spinner /><span>{t('auth.loading')}</span></> : <><span>{t('auth.reset_password')}</span><ArrowRight className="w-4 h-4" /></>}
            </button>
          </form>
        )}

        <p className="text-center text-sm text-slate-500 mt-6">
          <Link to="/login" className="inline-flex items-center gap-1 text-primary-500 hover:text-primary-600 font-medium">
            <ArrowLeft className="w-3.5 h-3.5" />
            {t('auth.back_to_login')}
          </Link>
        </p>
      </div>
    </div>
  );
};
