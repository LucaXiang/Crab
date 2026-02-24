import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Plus, Users, X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { listEmployees, createEmployee, updateEmployee, deleteEmployee } from '@/infrastructure/api/management';
import { ApiError } from '@/infrastructure/api/client';
import { DataTable, type Column } from '@/shared/components/DataTable';
import { FilterBar } from '@/shared/components/FilterBar/FilterBar';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog/ConfirmDialog';
import { FormField, inputClass } from '@/shared/components/FormField/FormField';
import { SelectField } from '@/shared/components/FormField/SelectField';
import type { Employee, EmployeeCreate, EmployeeUpdate } from '@/core/types/store';

function useRoleOptions(t: (key: string) => string) {
  return [
    { value: 1, label: t('settings.employee.role_admin') },
    { value: 2, label: t('settings.employee.role_manager') },
    { value: 3, label: t('settings.employee.role_user') },
  ];
}

function roleBadge(roleId: number, t: (key: string) => string) {
  switch (roleId) {
    case 1: return { label: t('settings.employee.role_admin'), cls: 'bg-red-50 text-red-700 border-red-200' };
    case 2: return { label: t('settings.employee.role_manager'), cls: 'bg-blue-50 text-blue-700 border-blue-200' };
    case 3: return { label: t('settings.employee.role_user'), cls: 'bg-green-50 text-green-700 border-green-200' };
    default: return { label: `Role ${roleId}`, cls: 'bg-gray-50 text-gray-700 border-gray-200' };
  }
}

function getInitials(name: string): string {
  return name
    .split(/\s+/)
    .map(w => w[0])
    .filter(Boolean)
    .slice(0, 2)
    .join('')
    .toUpperCase();
}

export const EmployeeManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const roleOptions = useRoleOptions(t);

  const [employees, setEmployees] = useState<Employee[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [searchQuery, setSearchQuery] = useState('');

  // Modal state
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<Employee | null>(null);
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  // Form fields
  const [formUsername, setFormUsername] = useState('');
  const [formPassword, setFormPassword] = useState('');
  const [formDisplayName, setFormDisplayName] = useState('');
  const [formRoleId, setFormRoleId] = useState<number>(3);

  // Delete confirmation
  const [deleteTarget, setDeleteTarget] = useState<Employee | null>(null);

  const loadData = useCallback(async () => {
    if (!token) return;
    try {
      setLoading(true);
      const data = await listEmployees(token, storeId);
      setEmployees(data);
      setError('');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  }, [token, storeId, t]);

  useEffect(() => { loadData(); }, [loadData]);

  const filtered = useMemo(() => {
    if (!searchQuery.trim()) return employees;
    const q = searchQuery.toLowerCase();
    return employees.filter(e =>
      e.username.toLowerCase().includes(q) ||
      e.display_name.toLowerCase().includes(q)
    );
  }, [employees, searchQuery]);

  const openCreate = () => {
    setEditing(null);
    setFormUsername('');
    setFormPassword('');
    setFormDisplayName('');
    setFormRoleId(3);
    setFormError('');
    setModalOpen(true);
  };

  const openEdit = (emp: Employee) => {
    setEditing(emp);
    setFormUsername(emp.username);
    setFormPassword('');
    setFormDisplayName(emp.display_name);
    setFormRoleId(emp.role_id);
    setFormError('');
    setModalOpen(true);
  };

  const handleSave = async () => {
    if (!token) return;
    if (!formUsername.trim()) { setFormError(t('settings.common.required_field')); return; }
    if (!editing && !formPassword.trim()) { setFormError(t('settings.common.required_field')); return; }

    setSaving(true);
    setFormError('');
    try {
      if (editing) {
        const payload: EmployeeUpdate = {
          username: formUsername.trim(),
          display_name: formDisplayName.trim() || undefined,
          role_id: formRoleId,
        };
        if (formPassword.trim()) payload.password = formPassword.trim();
        await updateEmployee(token, storeId, editing.id, payload);
      } else {
        const payload: EmployeeCreate = {
          username: formUsername.trim(),
          password: formPassword.trim(),
          display_name: formDisplayName.trim() || undefined,
          role_id: formRoleId,
        };
        await createEmployee(token, storeId, payload);
      }
      setModalOpen(false);
      await loadData();
    } catch (err) {
      setFormError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!token || !deleteTarget) return;
    try {
      await deleteEmployee(token, storeId, deleteTarget.id);
      setDeleteTarget(null);
      await loadData();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : t('auth.error_generic'));
      setDeleteTarget(null);
    }
  };

  const columns: Column<Employee>[] = [
    {
      key: 'name',
      header: t('settings.common.name'),
      render: (e) => (
        <div className="flex items-center gap-3">
          <div className="w-8 h-8 rounded-full bg-orange-100 text-orange-700 flex items-center justify-center text-xs font-bold">
            {getInitials(e.display_name || e.username)}
          </div>
          <div>
            <div className="font-medium text-gray-900">{e.display_name || e.username}</div>
            <div className="text-xs text-gray-500">@{e.username}</div>
          </div>
        </div>
      ),
    },
    {
      key: 'role',
      header: t('settings.employee.role'),
      width: '120px',
      render: (e) => {
        const badge = roleBadge(e.role_id, t);
        return (
          <span className={`inline-flex px-2.5 py-0.5 rounded-full text-xs font-medium border ${badge.cls}`}>
            {badge.label}
          </span>
        );
      },
    },
    {
      key: 'status',
      header: t('settings.common.status'),
      width: '100px',
      render: (e) => (
        <span className={`inline-flex px-2 py-0.5 rounded-full text-xs font-medium ${
          e.is_active ? 'bg-green-50 text-green-700' : 'bg-gray-100 text-gray-500'
        }`}>
          {e.is_active ? t('settings.common.active') : t('settings.common.inactive')}
        </span>
      ),
    },
  ];

  return (
    <div className="max-w-5xl mx-auto px-4 py-4 md:px-6 md:py-8 space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          <div className="w-10 h-10 bg-orange-100 rounded-xl flex items-center justify-center">
            <Users size={20} className="text-orange-600" />
          </div>
          <div>
            <h2 className="text-lg font-bold text-gray-900">{t('settings.employee.title')}</h2>
            <p className="text-sm text-gray-500">{t('settings.employee.subtitle')}</p>
          </div>
        </div>
        <button
          onClick={openCreate}
          className="inline-flex items-center gap-2 px-4 py-2.5 bg-orange-600 text-white rounded-xl text-sm font-medium hover:bg-orange-700 transition-colors shadow-sm"
        >
          <Plus size={16} />
          {t('common.action.add')}
        </button>
      </div>

      {error && (
        <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{error}</div>
      )}

      {/* Filter */}
      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        totalCount={filtered.length}
        countUnit={t('settings.employee.unit')}
        themeColor="orange"
      />

      {/* Table */}
      <DataTable
        data={filtered}
        columns={columns}
        loading={loading}
        onEdit={openEdit}
        onDelete={(e) => setDeleteTarget(e)}
        isEditable={(e) => !e.is_system}
        isDeletable={(e) => !e.is_system}
        getRowKey={(e) => e.id}
        themeColor="orange"
      />

      {/* Modal */}
      {modalOpen && (
        <div
          className="fixed inset-0 z-50 flex items-end md:items-center justify-center md:p-4 bg-black/50 backdrop-blur-sm"
          onClick={(e) => { if (e.target === e.currentTarget) setModalOpen(false); }}
        >
          <div className="bg-white rounded-t-2xl md:rounded-2xl shadow-xl w-full max-w-md overflow-hidden" style={{ animation: 'slideUp 0.25s ease-out' }}>
            {/* Modal Header */}
            <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100">
              <h3 className="text-lg font-bold text-gray-900">
                {editing ? t('common.action.edit') : t('common.action.add')} {t('settings.employee.title')}
              </h3>
              <button onClick={() => setModalOpen(false)} className="p-1 hover:bg-gray-100 rounded-lg transition-colors">
                <X size={20} className="text-gray-400" />
              </button>
            </div>

            {/* Modal Body */}
            <div className="px-6 py-5 space-y-4">
              {formError && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{formError}</div>
              )}

              <FormField label={t('settings.employee.username')} required>
                <input
                  type="text"
                  value={formUsername}
                  onChange={(e) => setFormUsername(e.target.value)}
                  className={inputClass}
                  placeholder={t('settings.employee.username')}
                />
              </FormField>

              <FormField label={t('settings.employee.password')} required={!editing}>
                <input
                  type="password"
                  value={formPassword}
                  onChange={(e) => setFormPassword(e.target.value)}
                  className={inputClass}
                  placeholder={editing ? t('settings.employee.password_keep') : t('settings.employee.password')}
                />
              </FormField>

              <FormField label={t('settings.employee.display_name')}>
                <input
                  type="text"
                  value={formDisplayName}
                  onChange={(e) => setFormDisplayName(e.target.value)}
                  className={inputClass}
                  placeholder={t('settings.employee.display_name')}
                />
              </FormField>

              <SelectField
                label={t('settings.employee.role')}
                value={formRoleId}
                onChange={(v) => setFormRoleId(Number(v))}
                options={roleOptions}
                required
              />
            </div>

            {/* Modal Footer */}
            <div className="px-6 py-4 border-t border-gray-100 flex justify-end gap-3">
              <button
                onClick={() => setModalOpen(false)}
                className="px-4 py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-medium hover:bg-gray-200 transition-colors"
              >
                {t('common.action.cancel')}
              </button>
              <button
                onClick={handleSave}
                disabled={saving}
                className="px-4 py-2.5 bg-orange-600 text-white rounded-xl text-sm font-medium hover:bg-orange-700 transition-colors disabled:opacity-50"
              >
                {saving ? t('auth.loading') : t('common.action.save')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Delete Confirmation */}
      <ConfirmDialog
        isOpen={!!deleteTarget}
        title={t('common.dialog.confirm_delete')}
        description={t('settings.employee.confirm.delete')}
        onConfirm={handleDelete}
        onCancel={() => setDeleteTarget(null)}
        variant="danger"
      />
    </div>
  );
};
