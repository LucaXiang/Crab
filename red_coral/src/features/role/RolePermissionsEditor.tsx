import React, { useEffect, useState } from 'react';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { getErrorMessage } from '@/utils/error';
import { Shield, Save, RefreshCw, Check, Plus, Trash2, Info } from 'lucide-react';
import { Role, RoleListData, RolePermissionListData } from '@/core/domain/types';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';

// Simplified permission labels (12 configurable permissions)
const usePermissionLabels = () => {
  const { t } = useI18n();
  return {
    // === 模块化权限 (6) ===
    'menu:manage': t('settings.permissions.menu_manage'),
    'tables:manage': t('settings.permissions.tables_manage'),
    'shifts:manage': t('settings.permissions.shifts_manage'),
    'reports:view': t('settings.permissions.reports_view'),
    'price_rules:manage': t('settings.permissions.price_rules_manage'),
    'settings:manage': t('settings.permissions.settings_manage'),
    // === 敏感操作 (6) ===
    'orders:void': t('settings.permissions.orders_void'),
    'orders:discount': t('settings.permissions.orders_discount'),
    'orders:comp': t('settings.permissions.orders_comp'),
    'orders:refund': t('settings.permissions.orders_refund'),
    'orders:modify_price': t('settings.permissions.orders_modify_price'),
    'cash_drawer:open': t('settings.permissions.cash_drawer_open'),
  };
};

export const RolePermissionsEditor: React.FC = () => {
  const { t } = useI18n();
  const permissionLabels = usePermissionLabels();
  const [loading, setLoading] = useState(false);
  const [roles, setRoles] = useState<Role[]>([]);
  const [availablePermissions, setAvailablePermissions] = useState<string[]>([]);
  const [rolePermissions, setRolePermissions] = useState<Record<string, string[]>>({});

  // Create Role State
  const [isCreateModalOpen, setIsCreateModalOpen] = useState(false);
  const [newRoleName, setNewRoleName] = useState('');
  const [newRoleDisplayName, setNewRoleDisplayName] = useState('');
  const [newRoleDesc, setNewRoleDesc] = useState('');

  // Delete Role State
  const [confirmDelete, setConfirmDelete] = useState<{ isOpen: boolean; roleId: number; roleName: string } | null>(null);

  const [selectedRole, setSelectedRole] = useState<Role | null>(null);

  // Simplified permission groups (2 groups, 12 permissions total)
  const getPermissionGroups = () => {
    return {
      modular: {
        title: t('settings.permissions.group.modular'),
        description: t('settings.permissions.group.modular_desc'),
        perms: [
          'menu:manage',
          'tables:manage',
          'shifts:manage',
          'reports:view',
          'price_rules:manage',
          'settings:manage',
        ]
      },
      sensitive: {
        title: t('settings.permissions.group.sensitive'),
        description: t('settings.permissions.group.sensitive_desc'),
        perms: [
          'orders:void',
          'orders:discount',
          'orders:comp',
          'orders:refund',
          'orders:modify_price',
          'cash_drawer:open',
        ]
      }
    };
  };

  // Load data
  const loadData = async () => {
    setLoading(true);
    try {
      // 1. Get all available permissions
      const allPerms = await invokeApi<string[]>('get_all_permissions');
      setAvailablePermissions(allPerms || []);

      // 2. Get roles
      const rolesData = await invokeApi<RoleListData>('list_roles');
      const rolesList = rolesData?.roles || [];
      setRoles(rolesList);

      // Select first role by default if none selected
      if (!selectedRole && rolesList.length > 0) {
        setSelectedRole(rolesList[0]);
      } else if (selectedRole) {
        // Update selected role object from new data
        const updated = rolesList.find(r => r.id === selectedRole.id);
        if (updated) setSelectedRole(updated);
      }

      // 3. Get permissions for each role
      const rolePerms: Record<string, string[]> = {};
      for (const role of rolesList) {
        const permsData = await invokeApi<RolePermissionListData>('get_role_permissions', { roleId: role.id });
        // Extract permission strings from RolePermission objects
        rolePerms[role.name] = permsData?.permissions?.map(p => p.permission) || [];
      }
      setRolePermissions(rolePerms);
    } catch (err) {
      console.error(err);
      toast.error(t('settings.roles.message.load_failed'));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadData();
  }, []);

  const handleToggle = (roleName: string, perm: string) => {
    setRolePermissions(prev => {
      const current = prev[roleName] || [];
      const exists = current.includes(perm);
      let next;
      if (exists) {
        next = current.filter(p => p !== perm);
      } else {
        next = [...current, perm];
      }
      return { ...prev, [roleName]: next };
    });
  };

  const handleSave = async () => {
    setLoading(true);
    try {
      for (const role of roles) {
        // Skip admin role as it has fixed permissions
        if (role.name === 'admin') continue;

        await invokeApi('update_role_permissions', {
          roleId: role.id,
          permissions: rolePermissions[role.name] || []
        });
      }
      toast.success(t('common.message.save_success'));
    } catch (err) {
      console.error(err);
      toast.error(t('common.message.save_failed'));
    } finally {
      setLoading(false);
    }
  };

  const handleCreateRole = async () => {
    if (!newRoleName || !newRoleDisplayName) {
      toast.error(t('settings.roles.form.error_empty'));
      return;
    }

    // Validate role name (lowercase alphanumeric)
    if (!/^[a-z0-9_]+$/.test(newRoleName)) {
      toast.error(t('settings.roles.form.error_format'));
      return;
    }

    try {
      await invokeApi('create_role', {
        data: {
          name: newRoleName,
          display_name: newRoleDisplayName,
          description: newRoleDesc,
          is_system: false,
          is_active: true
        }
      });
      toast.success(t('settings.roles.form.create_success'));
      setIsCreateModalOpen(false);
      setNewRoleName('');
      setNewRoleDisplayName('');
      setNewRoleDesc('');
      loadData();
    } catch (err) {
      console.error(err);
      toast.error(getErrorMessage(err));
    }
  };

  const handleDeleteRole = async () => {
    if (!confirmDelete) return;

    try {
      await invokeApi('delete_role', { id: confirmDelete.roleId });
      toast.success(t('settings.roles.form.delete_success'));
      setConfirmDelete(null);
      loadData();
    } catch (err) {
      console.error(err);
      toast.error(getErrorMessage(err));
    }
  };

  const getRoleDisplayName = (role: Role) => {
    if (role.name === 'admin') return t('auth.roles.admin') || role.display_name;
    return role.display_name;
  };

  // System roles are protected and cannot be deleted
  const isSystemRole = (role: Role) => role.name === 'admin';

  const permissionGroups = getPermissionGroups();

  return (
    <div className="flex h-[calc(100vh-140px)] gap-6">
      {/* Left Sidebar: Role List */}
      <div className="w-1/4 min-w-[15rem] flex flex-col bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
        <div className="p-4 border-b border-gray-100 flex justify-between items-center bg-gray-50">
          <h3 className="font-bold text-gray-700">{t('settings.roles.list.title')}</h3>
          <button
            onClick={() => setIsCreateModalOpen(true)}
            className="p-1.5 text-blue-600 bg-blue-50 hover:bg-blue-100 rounded-lg transition-colors"
            title={t('settings.roles.form.add')}
          >
            <Plus size={18} />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-2 space-y-1">
          {roles.map(role => (
            <div
              key={role.id}
              onClick={() => setSelectedRole(role)}
              className={`group flex items-center justify-between p-3 rounded-lg cursor-pointer transition-all ${
                selectedRole?.id === role.id
                  ? 'bg-blue-50 border-blue-200 text-blue-700 shadow-sm'
                  : 'hover:bg-gray-50 border-transparent text-gray-700'
              } border`}
            >
              <div className="flex flex-col">
                <span className="font-medium">{getRoleDisplayName(role)}</span>
                <span className={`text-xs ${selectedRole?.id === role.id ? 'text-blue-400' : 'text-gray-400'}`}>
                  {role.name}
                </span>
              </div>

              {!isSystemRole(role) && (
                <button
                  onClick={(e) => {
                    e.stopPropagation();
                    if (role.id) {
                      setConfirmDelete({ isOpen: true, roleId: role.id, roleName: role.display_name });
                    }
                  }}
                  className={`p-1.5 rounded-md text-gray-400 hover:text-red-500 hover:bg-red-50 transition-all ${
                    selectedRole?.id === role.id ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
                  }`}
                >
                  <Trash2 size={14} />
                </button>
              )}
              {isSystemRole(role) && (
                <Shield size={14} className="text-gray-300" />
              )}
            </div>
          ))}
        </div>
      </div>

      {/* Right Panel: Permissions */}
      <div className="flex-1 flex flex-col bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden">
        {selectedRole ? (
          <>
            <div className="p-4 border-b border-gray-100 flex justify-between items-center bg-gray-50">
              <div className="flex items-center gap-3">
                <div className={`p-2 rounded-lg ${selectedRole.name === 'admin' ? 'bg-purple-100 text-purple-600' : 'bg-blue-100 text-blue-600'}`}>
                  <Shield size={20} />
                </div>
                <div>
                  <h3 className="font-bold text-gray-800 text-lg">{getRoleDisplayName(selectedRole)}</h3>
                  <p className="text-xs text-gray-500">{selectedRole.description || t('settings.roles.form.no_description')}</p>
                </div>
              </div>

              <div className="flex gap-3">
                <button
                  onClick={loadData}
                  disabled={loading}
                  className="px-3 py-1.5 text-gray-600 bg-white border border-gray-200 rounded-lg hover:bg-gray-50 flex items-center gap-2 transition-colors text-sm"
                >
                  <RefreshCw size={14} className={loading ? 'animate-spin' : ''} />
                  <span>{t('common.action.refresh')}</span>
                </button>
                <button
                  onClick={handleSave}
                  disabled={loading || selectedRole.name === 'admin'}
                  className="px-4 py-1.5 text-white bg-blue-600 rounded-lg hover:bg-blue-700 flex items-center gap-2 shadow-sm transition-all text-sm disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  <Save size={14} />
                  <span>{t('common.action.save')}</span>
                </button>
              </div>
            </div>

            <div className="flex-1 overflow-y-auto p-6">
              {selectedRole.name === 'admin' && (
                <div className="mb-6 p-4 bg-purple-50 border border-purple-100 rounded-lg text-purple-800 flex items-start gap-3">
                  <Info size={18} className="mt-0.5 shrink-0" />
                  <div>
                    <p className="font-medium">{t('settings.roles.admin_lock.title')}</p>
                    <p className="text-sm opacity-80 mt-1">{t('settings.roles.admin_lock.desc')}</p>
                  </div>
                </div>
              )}

              <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
                {Object.entries(permissionGroups).map(([key, group]: [string, { title: string; description?: string; perms: string[] }]) => (
                  <div key={key} className={`rounded-xl p-5 border ${
                    key === 'sensitive' ? 'bg-amber-50/50 border-amber-200' : 'bg-gray-50 border-gray-100'
                  }`}>
                    <div className="mb-4">
                      <h4 className="font-bold text-gray-800 flex items-center gap-2">
                        {group.title}
                        <span className={`text-xs font-normal px-2 py-0.5 rounded-full ${
                          key === 'sensitive' ? 'bg-amber-100 text-amber-700' : 'bg-gray-200 text-gray-500'
                        }`}>
                          {group.perms.length}
                        </span>
                      </h4>
                      {group.description && (
                        <p className="text-xs text-gray-500 mt-1">{group.description}</p>
                      )}
                    </div>
                    <div className="space-y-1">
                      {group.perms.map((perm: string) => {
                        const isChecked = rolePermissions[selectedRole.name]?.includes(perm) || false;
                        const isSystemAdmin = selectedRole.name === 'admin';

                        return (
                          <label key={perm} className={`flex items-center gap-3 p-2.5 rounded-lg transition-all ${
                            isSystemAdmin
                              ? 'opacity-60 cursor-not-allowed'
                              : isChecked
                                ? 'bg-white shadow-sm'
                                : 'hover:bg-white/80 cursor-pointer'
                          }`}>
                            <div className="relative flex items-center">
                              <input
                                type="checkbox"
                                className="peer sr-only"
                                checked={isSystemAdmin || isChecked}
                                onChange={() => !isSystemAdmin && handleToggle(selectedRole.name, perm)}
                                disabled={isSystemAdmin}
                              />
                              <div className={`w-5 h-5 border-2 rounded-md transition-all flex items-center justify-center
                                ${isSystemAdmin || isChecked
                                  ? key === 'sensitive' ? 'bg-amber-500 border-amber-500' : 'bg-blue-600 border-blue-600'
                                  : 'bg-white border-gray-300 peer-hover:border-blue-400'
                                }`}>
                                {(isSystemAdmin || isChecked) && <Check size={12} className="text-white" strokeWidth={3} />}
                              </div>
                            </div>
                            <span className={`text-sm font-medium ${isChecked ? 'text-gray-800' : 'text-gray-600'}`}>
                              {permissionLabels[perm as keyof typeof permissionLabels] || perm}
                            </span>
                          </label>
                        );
                      })}
                    </div>
                  </div>
                ))}
              </div>
            </div>
          </>
        ) : (
          <div className="flex-1 flex flex-col items-center justify-center text-gray-400">
            <Shield size={48} className="mb-4 opacity-20" />
            <p>{t('settings.roles.select_role')}</p>
          </div>
        )}
      </div>

      {/* Create Role Modal */}
      {isCreateModalOpen && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm p-4">
          <div className="bg-white rounded-xl shadow-xl w-full max-w-md p-6 animate-in fade-in zoom-in duration-200">
            <h3 className="text-lg font-bold mb-4">{t('settings.roles.form.title')}</h3>
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.roles.form.id')}
                </label>
                <input
                  type="text"
                  value={newRoleName}
                  onChange={(e) => setNewRoleName(e.target.value.toLowerCase())}
                  placeholder={t('settings.roles.form.id_placeholder')}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
                />
                <p className="text-xs text-gray-500 mt-1">{t('settings.roles.form.id_hint')}</p>
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.roles.form.name')}
                </label>
                <input
                  type="text"
                  value={newRoleDisplayName}
                  onChange={(e) => setNewRoleDisplayName(e.target.value)}
                  placeholder={t('settings.roles.form.name_placeholder')}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all"
                />
              </div>
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {t('settings.roles.form.desc')}
                </label>
                <textarea
                  value={newRoleDesc}
                  onChange={(e) => setNewRoleDesc(e.target.value)}
                  placeholder={t('settings.roles.form.desc_placeholder')}
                  rows={3}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500 focus:border-blue-500 outline-none transition-all resize-none"
                />
              </div>
            </div>
            <div className="flex justify-end gap-3 mt-6">
              <button
                onClick={() => setIsCreateModalOpen(false)}
                className="px-4 py-2 text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
              >
                {t('common.action.cancel')}
              </button>
              <button
                onClick={handleCreateRole}
                className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 shadow-lg shadow-blue-600/20 transition-all"
              >
                {t('settings.roles.form.submit')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirmation */}
      <ConfirmDialog
        isOpen={!!confirmDelete}
        title={t('settings.roles.form.delete_title')}
        description={t('roles.delete.confirm', { name: confirmDelete?.roleName ?? '' }) || `确定要删除角色 "${confirmDelete?.roleName}" 吗？此操作无法撤销，且会影响已分配该角色的用户。`}
        onConfirm={handleDeleteRole}
        onCancel={() => setConfirmDelete(null)}
      />
    </div>
  );
};
