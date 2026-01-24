import React, { useState } from 'react';
import { invokeApi } from '@/infrastructure/api';
import { X, Shield, Lock, User as UserIcon, AlertCircle } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { User } from '@/core/domain/types';

interface AuthData {
  mode: string;
  session: {
    username: string;
    user_info: {
      id: string;
      username: string;
      role: string;
      permissions: string[];
      is_system?: boolean;
    };
  } | null;
}

interface SupervisorAuthModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSuccess: (supervisor: User) => void;
  requiredPermission?: string;
  actionDescription?: string;
}

export const SupervisorAuthModal: React.FC<SupervisorAuthModalProps> = ({
  isOpen,
  onClose,
  onSuccess,
  requiredPermission = 'void_order',
  actionDescription,
}) => {
  const { t } = useI18n();
  const [username, setUsername] = useState('');
  const [password, setPassword] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  if (!isOpen) return null;

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!username || !password) return;

    setIsLoading(true);
    setError(null);

    try {
      // 直接调用 login_employee 验证凭据（不会影响当前 session，因为这是一次性验证）
      const authData = await invokeApi<AuthData>('login_employee', {
        username,
        password,
      });

      if (!authData.session) {
        throw new Error(t('auth.login.invalid_credentials'));
      }

      const userInfo = authData.session.user_info;

      // 检查权限: admin 有所有权限，或者检查特定权限
      const hasPermission =
        userInfo.role === 'admin' ||
        userInfo.permissions.includes('*') ||
        userInfo.permissions.includes(requiredPermission);

      if (!hasPermission) {
        throw new Error(t('auth.unauthorized.permission'));
      }

      // 构造 User 对象返回
      const supervisor: User = {
        id: userInfo.id,
        username: userInfo.username,
        display_name: null,
        role_id: `role:${userInfo.role}`,
        role_name: userInfo.role,
        avatar: null,
        is_active: true,
        is_system: userInfo.is_system ?? false,
        created_at: new Date().toISOString(),
        updated_at: new Date().toISOString(),
      };

      onSuccess(supervisor);
      setPassword('');
      onClose();
    } catch (err) {
      console.error('Supervisor auth failed:', err);
      setError(typeof err === 'string' ? err : (err as Error).message || t('auth.login.failed'));
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="fixed inset-0 z-[9999] bg-black/50 flex items-center justify-center p-4 backdrop-blur-sm">
      <div className="bg-white rounded-2xl shadow-2xl max-w-md w-full overflow-hidden flex flex-col animate-in fade-in zoom-in duration-200">
        {/* Header */}
        <div className="p-6 border-b border-gray-100 flex justify-between items-center bg-teal-50">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-teal-100 rounded-full text-teal-700">
              <Shield size={24} />
            </div>
            <div>
              <h2 className="text-xl font-bold text-gray-800">
                {t('auth.supervisor_approval')}
              </h2>
              {actionDescription && (
                <p className="text-sm text-gray-500 mt-0.5">{actionDescription}</p>
              )}
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 hover:bg-teal-100/50 rounded-full transition-colors text-gray-500"
          >
            <X size={20} />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-6 space-y-4">
          {error && (
            <div className="p-3 bg-red-50 border border-red-100 rounded-xl flex items-start gap-3 text-red-600 text-sm">
              <AlertCircle size={18} className="shrink-0 mt-0.5" />
              <span>{error}</span>
            </div>
          )}

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1.5 ml-1">
              {t('auth.login.username')}
            </label>
            <div className="relative">
              <div className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400">
                <UserIcon size={18} />
              </div>
              <input
                type="text"
                value={username}
                onChange={(e) => setUsername(e.target.value)}
                className="w-full pl-10 pr-4 py-3 bg-gray-50 border border-gray-200 rounded-xl focus:ring-2 focus:ring-teal-500 focus:border-transparent outline-none transition-all"
                placeholder={t('auth.login.username')}
                autoFocus
              />
            </div>
          </div>

          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1.5 ml-1">
              {t('auth.login.password')}
            </label>
            <div className="relative">
              <div className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400">
                <Lock size={18} />
              </div>
              <input
                type="password"
                value={password}
                onChange={(e) => setPassword(e.target.value)}
                className="w-full pl-10 pr-4 py-3 bg-gray-50 border border-gray-200 rounded-xl focus:ring-2 focus:ring-teal-500 focus:border-transparent outline-none transition-all"
                placeholder={t('auth.login.password')}
              />
            </div>
          </div>

          <div className="pt-2">
            <button
              type="submit"
              disabled={isLoading || !username || !password}
              className="w-full py-3.5 bg-teal-600 text-white font-bold rounded-xl hover:bg-teal-700 active:scale-[0.98] transition-all shadow-lg shadow-teal-600/20 disabled:opacity-50 disabled:cursor-not-allowed flex items-center justify-center gap-2"
            >
              {isLoading ? (
                <div className="w-5 h-5 border-2 border-white/30 border-t-white rounded-full animate-spin" />
              ) : (
                <>
                  <Shield size={18} />
                  <span>{t('common.action.confirm')}</span>
                </>
              )}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
