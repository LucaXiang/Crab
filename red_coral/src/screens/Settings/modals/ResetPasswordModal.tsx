import React, { useState } from 'react';
import { X, Key, Eye, EyeOff } from 'lucide-react';
import { useI18n } from '../../../hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { User } from '@/core/domain/types';
import { useAuthStore } from '@/core/stores/auth/useAuthStore';

interface ResetPasswordModalProps {
  isOpen: boolean;
  onClose: () => void;
  user: User;
}

export const ResetPasswordModal: React.FC<ResetPasswordModalProps> = ({
  isOpen,
  onClose,
  user,
}) => {
  const { t } = useI18n();
  const { resetPassword } = useAuthStore();

  const [newPassword, setNewPassword] = useState('');
  const [confirmPassword, setConfirmPassword] = useState('');
  const [showPassword, setShowPassword] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Validation
    if (!newPassword) {
      toast.error(t("settings.user.form.passwordRequired"));
      return;
    }
    if (newPassword.length < 6) {
      toast.error(t("settings.user.form.passwordTooShort"));
      return;
    }
    if (newPassword !== confirmPassword) {
      toast.error(t("settings.user.form.passwordMismatch"));
      return;
    }

    setIsSubmitting(true);
    try {
      await resetPassword(user.id, newPassword);
      toast.success(t("settings.user.message.resetPasswordSuccess"));
      onClose();
    } catch (error: any) {
      console.error('Reset password error:', error);
      toast.error(error || t("settings.user.message.resetPasswordFailed"));
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 bg-black/50 z-50 flex items-center justify-center p-4">
      <div className="bg-white rounded-xl shadow-2xl w-full max-w-md">
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-gray-200">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-orange-100 rounded-lg flex items-center justify-center">
              <Key size={20} className="text-orange-600" />
            </div>
            <div>
              <h2 className="text-lg font-bold text-gray-900">
                {t('settings.resetPassword')}
              </h2>
              <p className="text-sm text-gray-500">{user.display_name} (@{user.username})</p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <X size={20} className="text-gray-400" />
          </button>
        </div>

        {/* Form */}
        <form onSubmit={handleSubmit} className="p-5 space-y-4">
          {/* New Password */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <div className="flex items-center gap-2">
                <Key size={14} />
                <span>{t('settings.newPassword')}</span>
                <span className="text-red-500">*</span>
              </div>
            </label>
            <div className="relative">
              <input
                type={showPassword ? 'text' : 'password'}
                value={newPassword}
                onChange={(e) => setNewPassword(e.target.value)}
                className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500/20 focus:border-orange-500 pr-10"
                placeholder={t('settings.enterNewPassword')}
              />
              <button
                type="button"
                onClick={() => setShowPassword(!showPassword)}
                className="absolute right-2 top-1/2 -translate-y-1/2 p-1.5 hover:bg-gray-100 rounded transition-colors"
              >
                {showPassword ? (
                  <EyeOff size={16} className="text-gray-400" />
                ) : (
                  <Eye size={16} className="text-gray-400" />
                )}
              </button>
            </div>
            <p className="text-xs text-gray-500 mt-1">
              {t('settings.passwordMinLength')}
            </p>
          </div>

          {/* Confirm Password */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <div className="flex items-center gap-2">
                <Key size={14} />
                <span>{t('settings.confirmPassword')}</span>
                <span className="text-red-500">*</span>
              </div>
            </label>
            <input
              type={showPassword ? 'text' : 'password'}
              value={confirmPassword}
              onChange={(e) => setConfirmPassword(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-orange-500/20 focus:border-orange-500"
              placeholder={t('settings.confirmPasswordPlaceholder')}
            />
          </div>

          {/* Warning */}
          <div className="bg-orange-50 border border-orange-200 rounded-lg p-3">
            <p className="text-xs text-orange-700">
              {t('settings.resetPasswordWarning')}
            </p>
          </div>

          {/* Buttons */}
          <div className="flex gap-3 pt-2">
            <button
              type="button"
              onClick={onClose}
              className="flex-1 px-4 py-2.5 border border-gray-300 text-gray-700 rounded-lg hover:bg-gray-50 transition-colors font-medium"
            >
              {t('common.cancel')}
            </button>
            <button
              type="submit"
              disabled={isSubmitting}
              className="flex-1 px-4 py-2.5 bg-orange-600 text-white rounded-lg hover:bg-orange-700 transition-colors font-medium disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isSubmitting
                ? t('common.submitting')
                : t('common.confirm')}
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};
