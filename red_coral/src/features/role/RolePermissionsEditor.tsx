import React, { useEffect, useState } from 'react';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { getErrorMessage } from '@/utils/error';
import { Shield, Save, RefreshCw, Check, Plus, Trash2, Info } from 'lucide-react';
import { Role, RoleListData, RolePermissionListData } from '@/core/domain/types';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';

// Map permissions to readable labels
const usePermissionLabels = () => {
  const { t } = useI18n();
  return {
    manage_users: t('settings.permissions.manage_users'),
    create_product: t('settings.permissions.create_product'),
    update_product: t('settings.permissions.update_product'),
    delete_product: t('settings.permissions.delete_product'),
    manage_categories: t('settings.permissions.manage_categories'),
    manage_zones: t('settings.permissions.manage_zones'),
    manage_tables: t('settings.permissions.manage_tables'),
    void_order: t('settings.permissions.void_order'),
    restore_order: t('settings.permissions.restore_order'),
    modify_price: t('settings.permissions.modify_price'),
    apply_discount: t('settings.permissions.apply_discount'),
    view_statistics: t('settings.permissions.view_statistics'),
    manage_printers: t('settings.permissions.manage_printers'),
    manage_attributes: t('settings.permissions.manage_attributes'),
    refund_order: t('settings.permissions.refund_order'),
    split_bill: t('settings.permissions.split_bill'),
    merge_bill: t('settings.permissions.merge_bill'),
    transfer_table: t('settings.permissions.transfer_table'),
    open_cash_drawer: t('settings.permissions.open_cash_drawer'),
    reprint_receipt: t('settings.permissions.reprint_receipt'),
    free_of_charge: t('settings.permissions.free_of_charge'),
    view_reports: t('settings.permissions.view_reports'),
    adjust_stock: t('settings.permissions.adjust_stock'),
    system_settings: t('settings.permissions.system_settings'),
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
  const [confirmDelete, setConfirmDelete] = useState<{ isOpen: boolean; roleId: string; roleName: string } | null>(null);

  const [selectedRole, setSelectedRole] = useState<Role | null>(null);

  // Group permissions
  const getPermissionGroups = () => {
    const groups = {
      menu: {
        title: t('settings.permissions.group.menu'),
        perms: ['create_product', 'update_product', 'delete_product', 'manage_categories', 'manage_attributes']
      },
      pos: {
        title: t('settings.permissions.group.pos'),
        perms: ['void_order', 'restore_order', 'modify_price', 'apply_discount', 'refund_order', 'split_bill', 'merge_bill', 'transfer_table', 'open_cash_drawer', 'reprint_receipt', 'free_of_charge']
      },
      store: {
        title: t('settings.permissions.group.store'),
        perms: ['manage_zones', 'manage_tables', 'manage_printers', 'adjust_stock']
      },
      system: {
        title: t('settings.permissions.group.system'),
        perms: ['manage_users', 'view_statistics', 'view_reports', 'system_settings']
      }
    };

    // Catch any unassigned permissions
    const assigned = new Set(Object.values(groups).flatMap(g => g.perms));
    const other = availablePermissions.filter(p => !assigned.has(p));

    if (other.length > 0) {
      return {
        ...groups,
        other: {
          title: t('settings.permissions.group.other'),
          perms: other
        }
      };
    }

    return groups;
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
        const permsData = await invokeApi<RolePermissionListData>('get_role_permissions', { role_id: role.id });
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
          role_id: role.id,
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
                    setConfirmDelete({ isOpen: true, roleId: role.id, roleName: role.display_name });
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
                {Object.entries(permissionGroups).map(([key, group]: [string, any]) => (
                  <div key={key} className="bg-gray-50 rounded-xl p-4 border border-gray-100">
                    <h4 className="font-bold text-gray-700 mb-3 pb-2 border-b border-gray-200 flex items-center justify-between">
                      {group.title}
                      <span className="text-xs font-normal text-gray-400 bg-white px-2 py-0.5 rounded-full border border-gray-200">
                        {group.perms.length}
                      </span>
                    </h4>
                    <div className="space-y-2">
                      {group.perms.map((perm: string) => {
                        const isChecked = rolePermissions[selectedRole.name]?.includes(perm) || false;
                        const isSystemAdmin = selectedRole.name === 'admin';

                        return (
                          <label key={perm} className={`flex items-center gap-3 p-2 rounded-lg transition-colors ${
                            isSystemAdmin ? 'opacity-70 cursor-not-allowed' : 'hover:bg-white cursor-pointer'
                          }`}>
                            <div className="relative flex items-center">
                              <input
                                type="checkbox"
                                className="peer sr-only"
                                checked={isSystemAdmin || isChecked}
                                onChange={() => !isSystemAdmin && handleToggle(selectedRole.name, perm)}
                                disabled={isSystemAdmin}
                              />
                              <div className={`w-5 h-5 border-2 rounded transition-all flex items-center justify-center
                                ${isSystemAdmin || isChecked
                                  ? 'bg-blue-600 border-blue-600'
                                  : 'bg-white border-gray-300 peer-hover:border-blue-400'
                                }`}>
                                <Check size={12} className="text-white" strokeWidth={3} />
                              </div>
                            </div>
                            <span className="text-sm text-gray-700 font-medium">
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
