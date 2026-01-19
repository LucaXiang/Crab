import React from 'react';
import { useNavigate } from 'react-router-dom';
import { ShieldX, ArrowLeft, Home } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useCurrentUser } from '@/core/stores/auth/useAuthStore';

/**
 * Unauthorized (403) page
 *
 * Displayed when a user tries to access a resource they don't have permission for.
 */
export const UnauthorizedScreen: React.FC = () => {
  const { t } = useI18n();
  const navigate = useNavigate();
  const user = useCurrentUser();

  const handleGoBack = () => {
    navigate(-1);
  };

  const handleGoHome = () => {
    navigate('/pos');
  };

  return (
    <div className="min-h-screen flex items-center justify-center bg-linear-to-br from-gray-50 to-gray-100 p-4 font-sans">
      <div className="text-center max-w-md">
        {/* Icon */}
        <div className="inline-flex items-center justify-center w-24 h-24 bg-red-100 rounded-full mb-6 shadow-lg">
          <ShieldX className="text-red-600" size={48} />
        </div>

        {/* Title */}
        <h1 className="text-4xl font-bold text-gray-900 mb-2">
          {t('auth.unauthorized.title')}
        </h1>

        <p className="text-lg text-gray-500 mb-2">{t('auth.unauthorized.forbiddenCode')}</p>

        {/* Message */}
        <p className="text-gray-600 mb-8 leading-relaxed">
          {t('auth.unauthorized.message')}
          <br />
          {t('auth.unauthorized.contact')}
        </p>

        {/* User Info */}
        {user && (
          <div className="bg-white rounded-lg p-4 mb-8 shadow-sm border border-gray-200">
            <div className="text-sm text-gray-500 mb-1">
              {t('auth.currentUser')}
            </div>
            <div className="font-medium text-gray-900">{user.display_name || user.username}</div>
            <div className="text-sm text-gray-500 mt-1">
              {t('auth.user.role')}: {' '}
              <span className="inline-block px-2 py-0.5 bg-gray-100 rounded text-gray-700 text-xs font-medium uppercase">
                {user.role_name || 'N/A'}
              </span>
            </div>
          </div>
        )}

        {/* Action Buttons */}
        <div className="flex flex-col sm:flex-row gap-3 justify-center">
          <button
            onClick={handleGoBack}
            className="inline-flex items-center justify-center gap-2 px-6 py-3 bg-white text-gray-700 font-medium rounded-xl hover:bg-gray-50 border border-gray-300 transition-all shadow-sm"
          >
            <ArrowLeft size={20} />
            {t('common.goBack')}
          </button>

          <button
            onClick={handleGoHome}
            className="inline-flex items-center justify-center gap-2 px-6 py-3 bg-[#FF5E5E] text-white font-bold rounded-xl hover:bg-[#E54545] transition-all shadow-lg"
          >
            <Home size={20} />
            {t('common.goHome')}
          </button>
        </div>

        {/* Help Text */}
        <p className="text-sm text-gray-400 mt-8">
          {t('auth.unauthorized.hint')}
        </p>
      </div>
    </div>
  );
};
