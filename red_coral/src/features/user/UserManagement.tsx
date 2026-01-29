import React, { useEffect, useMemo, useState } from 'react';
import { Users, Plus, Filter, Search, Shield, Calendar, Key, Lock, Check, Edit3, Trash2, Ban } from 'lucide-react';
import { createTauriClient } from '@/infrastructure/api';
import { useI18n } from '@/hooks/useI18n';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission, User, Role } from '@/core/domain/types';
import { useCanManageUsers } from '@/hooks/usePermission';
import { DataTable, Column } from '@/shared/components/DataTable';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { toast } from '@/presentation/components/Toast';
import { useAuthStore, useCurrentUser } from '@/core/stores/auth/useAuthStore';
import { UserFormModal } from './UserFormModal';
import { ResetPasswordModal } from './ResetPasswordModal';
import { RolePermissionsEditor } from '@/features/role';

export const UserManagement: React.FC = React.memo(() => {
  const { t } = useI18n();

  // Permission check
  const canManageUsers = useCanManageUsers();
  const currentUser = useCurrentUser();

  const fetchUsers = useAuthStore(state => state.fetchUsers);
  const deleteUserAction = useAuthStore(state => state.deleteUser);
  const updateUserAction = useAuthStore(state => state.updateUser);

  const [users, setUsers] = useState<User[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [searchQuery, setSearchQuery] = useState('');
  const [roleFilter, setRoleFilter] = useState<'all' | string>('all');
  const [showInactive, setShowInactive] = useState(false);
  const [activeTab, setActiveTab] = useState<'users' | 'roles'>('users');
  const [roles, setRoles] = useState<Role[]>([]);
  const [userFormOpen, setUserFormOpen] = useState(false);
  const [editingUser, setEditingUser] = useState<User | null>(null);
  const [resetPasswordOpen, setResetPasswordOpen] = useState(false);
  const [resetPasswordUser, setResetPasswordUser] = useState<User | null>(null);
  const [confirmDialog, setConfirmDialog] = useState({
    isOpen: false,
    title: '',
    description: '',
    onConfirm: () => {},
  });

  // Load users and roles on mount
  useEffect(() => {
    if (canManageUsers) {
      setIsLoading(true);
      fetchUsers()
        .then(setUsers)
        .catch(console.error)
        .finally(() => setIsLoading(false));

      createTauriClient().listRoles()
        .then(data => setRoles(data.roles))
        .catch(console.error);
    }
  }, [canManageUsers, fetchUsers]);

  const filteredUsers = useMemo(() => {
    let result = users;

    // Filter by role
    if (roleFilter !== 'all') {
      result = result.filter((u) => u.role_name === roleFilter);
    }

    // Filter inactive
    if (!showInactive) {
      result = result.filter((u) => u.is_active);
    }

    // Filter by search query
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        (u) =>
          (u.display_name || '').toLowerCase().includes(q) ||
          u.username.toLowerCase().includes(q)
      );
    }

    return result;
  }, [users, roleFilter, searchQuery, showInactive]);

  const handleAddUser = () => {
    setEditingUser(null);
    setUserFormOpen(true);
  };

  const handleEditUser = (user: User) => {
    setEditingUser(user);
    setUserFormOpen(true);
  };

  const handleResetPassword = (user: User) => {
    setResetPasswordUser(user);
    setResetPasswordOpen(true);
  };

  const handleDeleteUser = (user: User) => {
    // Prevent deleting own account
    if (currentUser && user.id === currentUser.id) {
      toast.error(t('settings.user.message.cannot_delete_self'));
      return;
    }

    // Prevent deleting admin users
    if (user.username === 'admin') {
      toast.error(t('settings.user.message.cannot_delete_admin'));
      return;
    }

    if (user.is_active) {
      // Disable user logic
      setConfirmDialog({
        isOpen: true,
        title: t('settings.user.disable_user'),
        description: t('settings.user.confirm.disable', { name: user.display_name || user.username }),
        onConfirm: async () => {
          setConfirmDialog((prev) => ({ ...prev, isOpen: false }));
          try {
            await updateUserAction(user.id, { isActive: false });
            toast.success(t('settings.user.message.update_success'));
            const updatedUsers = await fetchUsers();
            setUsers(updatedUsers);
          } catch (error: unknown) {
            toast.error(error instanceof Error ? error.message : t('settings.user.message.update_failed'));
          }
        },
      });
    } else {
      // Permanent delete logic
      setConfirmDialog({
        isOpen: true,
        title: t('settings.user.delete_permanently_user'),
        description: t('settings.user.confirm.deletePermanently', { name: user.display_name || user.username }),
        onConfirm: async () => {
          setConfirmDialog((prev) => ({ ...prev, isOpen: false }));
          try {
            await deleteUserAction(user.id);
            toast.success(t('settings.user.message.delete_success'));
            const updatedUsers = await fetchUsers();
            setUsers(updatedUsers);
          } catch (error: unknown) {
            toast.error(error instanceof Error ? error.message : t('settings.user.message.delete_failed'));
          }
        },
      });
    }
  };

  const getRoleBadgeClass = (role: string | undefined) => {
    const classes: Record<string, string> = {
      admin: 'bg-red-100 text-red-700',
    };
    return classes[role || ''] || 'bg-purple-100 text-purple-700';
  };

  const getRoleLabel = (role: string | undefined) => {
    if (!role) return '';
    // Try to find in fetched roles first
    const roleObj = roles.find(r => r.name === role);
    if (roleObj) {
        if (role === 'admin') return t('auth.roles.admin');
        return roleObj.display_name;
    }

    const labels: Record<string, string> = {
      admin: t('auth.roles.admin'),
    };
    return labels[role] || role;
  };

  const formatDate = (timestamp: number) => {
    return new Date(timestamp).toLocaleDateString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    });
  };

  const columns: Column<User>[] = useMemo(
    () => [
      {
        key: 'user',
        header: t('settings.user.column.user'),
        render: (user) => (
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-gradient-to-br from-blue-400 to-blue-600 rounded-full flex items-center justify-center text-white font-bold">
              {(user.display_name || user.username).charAt(0).toUpperCase()}
            </div>
            <div>
              <div className="flex items-center gap-2">
                <span className="font-medium text-gray-900">{user.display_name || user.username}</span>
                {user.is_system && (
                  <span className="text-xs bg-gray-100 text-gray-500 px-2 py-0.5 rounded">
                    {t('common.label.system')}
                  </span>
                )}
                {!user.is_active && (
                  <span className="text-xs bg-gray-200 text-gray-600 px-2 py-0.5 rounded">
                    {t('common.status.inactive')}
                  </span>
                )}
              </div>
              <div className="text-xs text-gray-400">@{user.username}</div>
            </div>
          </div>
        ),
      },
      {
        key: 'role',
        header: t('auth.user.role'),
        width: '120px',
        render: (user) => (
          <span
            className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium ${getRoleBadgeClass(
              user.role_name
            )}`}
          >
            <Shield size={12} />
            {getRoleLabel(user.role_name)}
          </span>
        ),
      },
      {
        key: 'createdAt',
        header: t('settings.user.column.created_at'),
        width: '120px',
        render: (user) => (
          <div className="flex items-center gap-1.5 text-sm text-gray-600">
            <Calendar size={14} />
            <span>{formatDate(user.created_at)}</span>
          </div>
        ),
      },
      {
        key: 'actions',
        header: t('settings.user.column.actions'),
        width: '160px',
        align: 'right',
        render: (user) => (
          <ProtectedGate permission={Permission.USERS_MANAGE}>
            <div className="flex items-center justify-end gap-2">
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleEditUser(user);
                }}
                className="p-2 bg-amber-50 text-amber-700 rounded-lg hover:bg-amber-100 transition-colors border border-amber-200/50"
                title={t('common.action.edit')}
              >
                <Edit3 size={14} />
              </button>
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleResetPassword(user);
                }}
                className="p-2 bg-orange-50 text-orange-700 rounded-lg hover:bg-orange-100 transition-colors border border-orange-200/50"
                title={t('settings.user.reset_password_user')}
              >
                <Key size={14} />
              </button>
              {user.username !== 'admin' && (
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      handleDeleteUser(user);
                    }}
                    className={`p-2 rounded-lg transition-colors border ${
                      user.is_active
                        ? 'bg-amber-50 text-amber-600 border-amber-200/50 hover:bg-amber-100'
                        : 'bg-red-50 text-red-600 border-red-200/50 hover:bg-red-100'
                    }`}
                    title={user.is_active ? t('settings.user.disable_user') : t('settings.user.delete_permanently_user')}
                  >
                    {user.is_active ? <Ban size={14} /> : <Trash2 size={14} />}
                  </button>
              )}
            </div>
          </ProtectedGate>
        ),
      },
    ],
    [t]
  );

  // Don't render if user doesn't have permission
  if (!canManageUsers) {
    return (
      <div className="flex items-center justify-center h-full">
        <div className="text-center">
          <Shield className="mx-auto text-gray-300 mb-4" size={48} />
          <p className="text-gray-500">
            {t('auth.unauthorized.message')}
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      {/* Header Card */}
      <div className="bg-white rounded-xl border border-gray-200 p-5 shadow-sm">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-blue-100 rounded-xl flex items-center justify-center">
              <Users size={20} className="text-blue-600" />
            </div>
            <div>
              <h2 className="text-lg font-bold text-gray-900">
                {t('settings.user.title')}
              </h2>
              <p className="text-sm text-gray-500">
                {t('settings.user.description')}
              </p>
            </div>
          </div>
          <ProtectedGate permission={Permission.USERS_MANAGE}>
            <button
              onClick={handleAddUser}
              className="inline-flex items-center gap-2 px-4 py-2.5 bg-blue-600 text-white rounded-xl text-sm font-semibold shadow-lg shadow-blue-600/20 hover:bg-blue-700 hover:shadow-blue-600/30 transition-all"
            >
              <Plus size={16} />
              <span>{t('settings.user.add_user')}</span>
            </button>
          </ProtectedGate>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex space-x-1 bg-gray-100 p-1 rounded-xl w-fit">
        <button
          onClick={() => setActiveTab('users')}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === 'users'
              ? 'bg-white text-blue-600 shadow-sm'
              : 'text-gray-600 hover:text-gray-900 hover:bg-gray-200/50'
          }`}
        >
          <Users size={16} />
          {t('settings.user.list')}
        </button>
        <button
          onClick={() => setActiveTab('roles')}
          className={`flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium transition-all ${
            activeTab === 'roles'
              ? 'bg-white text-blue-600 shadow-sm'
              : 'text-gray-600 hover:text-gray-900 hover:bg-gray-200/50'
          }`}
        >
          <Lock size={16} />
          {t('settings.user.roles')}
        </button>
      </div>

      {activeTab === 'users' ? (
        <>
      {/* Filter Bar */}
      <div className="bg-white rounded-xl border border-gray-200 p-4 shadow-sm">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2 text-gray-500">
            <Filter size={16} />
            <span className="text-sm font-medium">{t('common.action.filter')}</span>
          </div>
          <div className="h-5 w-px bg-gray-200" />
          <div className="flex items-center gap-2">
            <label className="text-sm text-gray-600">{t('auth.user.role')}:</label>
            <select
              value={roleFilter}
              onChange={(e) => setRoleFilter(e.target.value)}
              className="border border-gray-200 rounded-lg px-3 py-1.5 text-sm bg-white focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 transition-colors min-w-[8.75rem]"
            >
              <option value="all">{t('table.filter.all')}</option>
              {roles.length > 0 ? (
                roles.map((role) => (
                  <option key={role.id} value={role.name}>
                    {getRoleLabel(role.name)}
                  </option>
                ))
              ) : (
                <>
                  <option value="admin">{t('auth.roles.admin')}</option>
                </>
              )}
            </select>
          </div>

          <div className="h-5 w-px bg-gray-200 ml-2" />

          <label className="flex items-center gap-2 cursor-pointer select-none">
            <div className={`w-4 h-4 rounded border flex items-center justify-center transition-colors ${showInactive ? 'bg-blue-600 border-blue-600' : 'bg-white border-gray-300'}`}>
              {showInactive && <Check size={12} className="text-white" />}
            </div>
            <input
              type="checkbox"
              className="hidden"
              checked={showInactive}
              onChange={(e) => setShowInactive(e.target.checked)}
            />
            <span className="text-sm text-gray-600">{t('settings.user.filter.show_inactive')}</span>
          </label>

          <div className="h-5 w-px bg-gray-200 ml-2" />
          <div className="relative flex-1 max-w-xs">
            <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder={t('common.hint.search_placeholder')}
              className="w-full pl-9 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
            />
          </div>

          <div className="ml-auto text-xs text-gray-400">
            {t('common.label.total')} {filteredUsers.length} {t('settings.user.unit')}
          </div>
        </div>
      </div>

      {/* Data Table */}
      <DataTable
        data={filteredUsers}
        columns={columns}
        loading={isLoading}
        getRowKey={(user) => user.id}
        emptyText={t('common.empty.no_data')}
        themeColor="blue"
      />
        </>
      ) : (
        <RolePermissionsEditor />
      )}

      {/* User Form Modal */}
      {userFormOpen && (
        <UserFormModal
          isOpen={userFormOpen}
          onClose={() => {
            setUserFormOpen(false);
            setEditingUser(null);
          }}
          editingUser={editingUser}
          onSuccess={async () => {
            setUserFormOpen(false);
            setEditingUser(null);
            const updatedUsers = await fetchUsers();
            setUsers(updatedUsers);
          }}
        />
      )}

      {/* Reset Password Modal */}
      {resetPasswordOpen && resetPasswordUser && (
        <ResetPasswordModal
          isOpen={resetPasswordOpen}
          onClose={() => {
            setResetPasswordOpen(false);
            setResetPasswordUser(null);
          }}
          user={resetPasswordUser}
        />
      )}

      {/* Confirm Dialog */}
      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={confirmDialog.onConfirm}
        onCancel={() => setConfirmDialog((prev) => ({ ...prev, isOpen: false }))}
      />
    </div>
  );
});
