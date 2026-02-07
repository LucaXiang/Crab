import React, { useState, useEffect } from 'react';
import { X, User as UserIcon, KeyRound, Shield, Eye, EyeOff, Settings2 } from 'lucide-react';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { getErrorMessage } from '@/utils/error';
import { User, Role } from '@/core/domain/types';
import { RoleListData } from '@/core/domain/types/api';
import { useAuthStore, useCurrentUser } from '@/core/stores/auth/useAuthStore';
import { FormField, FormSection, CheckboxField, inputClass, selectClass } from '@/shared/components/FormField';

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
    role_id: 0 as number,
    isActive: true,
  });
  const [showPassword, setShowPassword] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);

  useEffect(() => {
    // Fetch roles
    invokeApi<RoleListData>('list_roles')
      .then((resp) => {
        const fetchedRoles = resp.roles;
        setRoles(fetchedRoles);
        // Set default role if creating new user
        if (!editingUser && fetchedRoles.length > 0) {
           const defaultRole = fetchedRoles.find(r => r.name !== 'admin') || fetchedRoles[0];
           setFormData(prev => ({ ...prev, role_id: defaultRole.id }));
        }
      })
      .catch((err) => {
        console.error('Failed to fetch roles:', err);
        toast.error(t('settings.user.message.load_roles_failed'));
      });
  }, []);

  useEffect(() => {
    if (editingUser) {
      setFormData({
        username: editingUser.username,
        password: '', // Don't populate password for editing
        displayName: editingUser.display_name || '',
        role_id: editingUser.role_id,
        isActive: editingUser.is_active,
      });
    } else if (roles.length > 0) {
      const defaultRole = roles.find(r => r.name !== 'admin') || roles[0];
      setFormData({
        username: '',
        password: '',
        displayName: '',
        role_id: defaultRole.id,
        isActive: true,
      });
    }
  }, [editingUser, roles]); // Added roles dependency so it updates when roles are loaded

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    // Validation
    if (!formData.username.trim()) {
      toast.error(t('settings.user.form.username_required'));
      return;
    }
    if (!editingUser && !formData.password) {
      toast.error(t('settings.user.form.password_required'));
      return;
    }
    if (!formData.displayName.trim()) {
      toast.error(t('settings.user.form.display_name_required'));
      return;
    }

    setIsSubmitting(true);
    try {
      if (editingUser) {
        // Update user
        await updateUser(editingUser.id, {
          displayName: formData.displayName,
          role_id: formData.role_id,
          isActive: formData.isActive,
        });
        toast.success(t("settings.user.message.update_success"));
      } else {
        // Create user
        await createUser({
          username: formData.username,
          password: formData.password,
          displayName: formData.displayName,
          role_id: formData.role_id,
        });
        toast.success(t("settings.user.message.create_success"));
      }
      onSuccess();
    } catch (error) {
      console.error('User form error:', error);
      toast.error(getErrorMessage(error));
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!isOpen) return null;

  const isAdminUser = editingUser?.username === 'admin';

  return (
    <div
      className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4"
      onClick={onClose}
    >
      <div
        className="bg-gray-50 rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-gray-200 bg-white">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-bold text-gray-900">
              {editingUser ? t('settings.user.edit_user') : t('settings.user.add_user')}
            </h2>
            <button
              onClick={onClose}
              className="p-2 hover:bg-gray-100 rounded-xl transition-colors"
            >
              <X size={18} className="text-gray-500" />
            </button>
          </div>
        </div>

        {/* Content */}
        <form onSubmit={handleSubmit} className="p-4 space-y-4 max-h-[70vh] overflow-y-auto">
          {/* Account Section */}
          <FormSection title={t('settings.user.section.account')} icon={KeyRound}>
            <FormField label={t('settings.user.form.username')} required>
              <input
                type="text"
                value={formData.username}
                onChange={(e) => setFormData({ ...formData, username: e.target.value })}
                disabled={!!editingUser}
                className={inputClass}
                placeholder={t('settings.user.form.username_placeholder')}
              />
              {editingUser && (
                <p className="text-xs text-gray-500 mt-1">
                  {t('settings.user.form.username_cannot_change')}
                </p>
              )}
            </FormField>

            {!editingUser && (
              <FormField label={t('settings.user.form.password')} required>
                <div className="relative">
                  <input
                    type={showPassword ? 'text' : 'password'}
                    value={formData.password}
                    onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                    className={`${inputClass} pr-10`}
                    placeholder={t('settings.user.form.password_placeholder')}
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
              </FormField>
            )}
          </FormSection>

          {/* Profile Section */}
          <FormSection title={t('settings.user.section.profile')} icon={UserIcon}>
            <FormField label={t('settings.user.form.display_name')} required>
              <input
                type="text"
                value={formData.displayName}
                onChange={(e) => setFormData({ ...formData, displayName: e.target.value })}
                className={inputClass}
                placeholder={t('settings.user.form.display_name_placeholder')}
              />
            </FormField>

            <FormField label={t('settings.user.form.role')} required>
              <select
                value={formData.role_id}
                onChange={(e) => setFormData({ ...formData, role_id: Number(e.target.value) })}
                disabled={isAdminUser}
                className={selectClass}
              >
                {roles.map((role) => {
                  let label = role.display_name;
                  if (role.name === 'admin') label = t('auth.roles.admin') || label;
                  return (
                    <option key={role.id} value={role.id}>
                      {label}
                    </option>
                  );
                })}
              </select>
              {isAdminUser && (
                <p className="text-xs text-gray-500 mt-1">
                  {t('settings.user.form.admin_role_cannot_change')}
                </p>
              )}
            </FormField>
          </FormSection>

          {/* Advanced Section */}
          <FormSection title={t('settings.attribute.section.advanced')} icon={Settings2} defaultCollapsed>
            <CheckboxField
              id="isActive"
              label={t('settings.user.form.active_status')}
              description={isAdminUser ? t('settings.user.form.admin_cannot_disable') : undefined}
              checked={formData.isActive}
              onChange={(checked) => setFormData({ ...formData, isActive: checked })}
              disabled={isAdminUser}
            />
          </FormSection>
        </form>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-gray-200 bg-white flex justify-end gap-3">
          <button
            type="button"
            onClick={onClose}
            className="px-5 py-2.5 bg-white border border-gray-200 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-50 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          <button
            onClick={handleSubmit}
            disabled={isSubmitting}
            className="px-5 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-semibold hover:bg-teal-700 transition-colors shadow-lg shadow-teal-600/20 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {isSubmitting ? t('common.message.submitting') : t('common.action.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
};
