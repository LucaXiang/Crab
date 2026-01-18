import React, { useState, useEffect } from 'react';
import { X, User as UserIcon, Mail, Shield, Eye, EyeOff } from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { useI18n } from '../../../hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { User, Role } from '@/core/domain/types';
import { useAuthStore, useCurrentUser } from '@/core/stores/auth/useAuthStore';

interface UserFormModalProps {
  isOpen: boolean;
  onClose: () => void;
  editingUser: User | null;
  onSuccess: () => void;
}

export const UserFormModal: React.FC<UserFormModalProps> = ({
  isOpen,
  onClose,
  editingUser,
  onSuccess,
}) => {
  const { t } = useI18n();
  const currentUser = useCurrentUser();
  const { createUser, updateUser } = useAuthStore();

  const [roles, setRoles] = useState<Role[]>([]);
  const [formData, setFormData] = useState({
    username: '',
    password: '',
    displayName: '',
    role: '' as string,
    isActive: true,
  });
  const [showPassword, setShowPassword] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    // Fetch roles
    invoke<Role[]>('fetch_roles')
      .then((fetchedRoles) => {
        setRoles(fetchedRoles);
        // Set default role if creating new user
        if (!editingUser && fetchedRoles.length > 0) {
           const defaultRole = fetchedRoles.find(r => r.name !== 'admin') || fetchedRoles[0];
           setFormData(prev => ({ ...prev, role: defaultRole.name }));
        }
      })
      .catch((err) => {
        console.error('Failed to fetch roles:', err);
        toast.error(t('settings.user.message.loadRolesFailed'));
      });
  }, []);

  useEffect(() => {
    if (editingUser) {
      setFormData({
        username: editingUser.username,
        password: '', // Don't populate password for editing
        displayName: editingUser.display_name || '',
        role: String(editingUser.role_id),
        isActive: editingUser.is_active,
      });
    } else if (roles.length > 0) {
      const defaultRole = roles.find(r => r.name !== 'admin') || roles[0];
      setFormData({
        username: '',
        password: '',
        displayName: '',
        role: defaultRole.name,
        isActive: true,
      });
    }
  }, [editingUser, roles]); // Added roles dependency so it updates when roles are loaded

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Validation
    if (!formData.username.trim()) {
      toast.error(t('settings.user.form.usernameRequired'));
      return;
    }
    if (!editingUser && !formData.password) {
      toast.error(t('settings.user.form.passwordRequired'));
      return;
    }
    if (!formData.displayName.trim()) {
      toast.error(t('settings.user.form.displayNameRequired'));
      return;
    }

    setIsSubmitting(true);
    try {
      if (editingUser) {
        // Update user
        await updateUser(editingUser.id, {
          displayName: formData.displayName,
          role: formData.role,
          isActive: formData.isActive,
        });
        toast.success(t("settings.user.message.updateSuccess"));
      } else {
        // Create user
        await createUser({
          username: formData.username,
          password: formData.password,
          displayName: formData.displayName,
          role: formData.role,
        });
        toast.success(t("settings.user.message.createSuccess"));
      }
      onSuccess();
    } catch (error: any) {
      console.error('User form error:', error);
      toast.error(error || t('settings.user.message.operationFailed'));
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
            <div className="w-10 h-10 bg-blue-100 rounded-lg flex items-center justify-center">
              <UserIcon size={20} className="text-blue-600" />
            </div>
            <h2 className="text-lg font-bold text-gray-900">
              {editingUser
                ? t('settings.user.action.edit')
                : t('settings.user.action.add')}
            </h2>
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
          {/* Username */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <div className="flex items-center gap-2">
                <Mail size={14} />
                <span>{t('settings.user.form.username')}</span>
                <span className="text-red-500">*</span>
              </div>
            </label>
            <input
              type="text"
              value={formData.username}
              onChange={(e) => setFormData({ ...formData, username: e.target.value })}
              disabled={!!editingUser} // Username cannot be changed
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 disabled:bg-gray-100 disabled:cursor-not-allowed"
              placeholder={t('settings.user.form.usernamePlaceholder')}
            />
            {editingUser && (
              <p className="text-xs text-gray-500 mt-1">
                {t('settings.user.form.usernameCannotChange')}
              </p>
            )}
          </div>

          {/* Password (only for create) */}
          {!editingUser && (
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-2">
                <div className="flex items-center gap-2">
                  <Shield size={14} />
                  <span>{t('settings.user.form.password')}</span>
                  <span className="text-red-500">*</span>
                </div>
              </label>
              <div className="relative">
                <input
                  type={showPassword ? 'text' : 'password'}
                  value={formData.password}
                  onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 pr-10"
                  placeholder={t('settings.user.form.passwordPlaceholder')}
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
            </div>
          )}

          {/* Display Name */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <div className="flex items-center gap-2">
                <UserIcon size={14} />
                <span>{t('settings.user.form.displayName')}</span>
                <span className="text-red-500">*</span>
              </div>
            </label>
            <input
              type="text"
              value={formData.displayName}
              onChange={(e) => setFormData({ ...formData, displayName: e.target.value })}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
              placeholder={t('settings.user.form.displayNamePlaceholder')}
            />
          </div>

          {/* Role */}
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              <div className="flex items-center gap-2">
                <Shield size={14} />
                <span>{t('settings.user.form.role')}</span>
                <span className="text-red-500">*</span>
              </div>
            </label>
            <select
              value={formData.role}
              onChange={(e) => setFormData({ ...formData, role: e.target.value })}
              disabled={editingUser?.username === 'admin'}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 disabled:bg-gray-100 disabled:cursor-not-allowed"
            >
              {roles.map((role) => {
                let label = role.display_name;
                // Try to use i18n for system roles
                if (role.name === 'admin') label = t('auth.roles.admin') || label;

                return (
                  <option key={role.id} value={role.name}>
                    {label}
                  </option>
                );
              })}
            </select>
            {editingUser?.username === 'admin' && (
              <p className="text-xs text-gray-500 mt-1">
                {t('settings.user.form.adminRoleCannotChange')}
              </p>
            )}
          </div>

          {/* Active Status */}
          <div className="flex items-center gap-2">
            <input
              type="checkbox"
              id="isActive"
              checked={formData.isActive}
              onChange={(e) => setFormData({ ...formData, isActive: e.target.checked })}
              disabled={editingUser?.username === 'admin'}
              className="w-4 h-4 text-blue-600 border-gray-300 rounded focus:ring-2 focus:ring-blue-500/20 disabled:cursor-not-allowed disabled:opacity-50"
            />
            <label htmlFor="isActive" className={`text-sm ${editingUser?.username === 'admin' ? 'text-gray-400' : 'text-gray-700'}`}>
              {t('settings.user.form.activeStatus')}
            </label>
            {editingUser?.username === 'admin' && (
              <span className="text-xs text-gray-400 ml-2">
                ({t('settings.user.form.adminCannotDisable')})
              </span>
            )}
          </div>

          {/* Buttons */}
          <div className="flex gap-3 pt-4">
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
              className="flex-1 px-4 py-2.5 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium disabled:opacity-50 disabled:cursor-not-allowed"
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
