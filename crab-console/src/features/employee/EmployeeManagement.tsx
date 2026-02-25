import React, { useCallback, useEffect, useMemo, useState } from 'react';
import { Users } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreId } from '@/hooks/useStoreId';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { listEmployees, createEmployee, updateEmployee, deleteEmployee } from '@/infrastructure/api/management';
import { ApiError } from '@/infrastructure/api/client';
import { MasterDetail } from '@/shared/components/MasterDetail';
import { DetailPanel } from '@/shared/components/DetailPanel';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { FormField, inputClass, CheckboxField } from '@/shared/components/FormField';
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
    case 1: return { label: t('settings.employee.role_admin'), cls: 'bg-red-50 text-red-700' };
    case 2: return { label: t('settings.employee.role_manager'), cls: 'bg-blue-50 text-blue-700' };
    case 3: return { label: t('settings.employee.role_user'), cls: 'bg-green-50 text-green-700' };
    default: return { label: `Role ${roleId}`, cls: 'bg-gray-50 text-gray-700' };
  }
}

function getInitials(name: string): string {
  return name.split(/\s+/).map(w => w[0]).filter(Boolean).slice(0, 2).join('').toUpperCase();
}

type PanelState =
  | { type: 'closed' }
  | { type: 'create' }
  | { type: 'edit'; item: Employee }
  | { type: 'delete'; item: Employee };

export const EmployeeManagement: React.FC = () => {
  const { t } = useI18n();
  const storeId = useStoreId();
  const token = useAuthStore(s => s.token);
  const roleOptions = useRoleOptions(t);

  const [employees, setEmployees] = useState<Employee[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState('');
  const [panel, setPanel] = useState<PanelState>({ type: 'closed' });
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState('');

  // Form fields
  const [formUsername, setFormUsername] = useState('');
  const [formPassword, setFormPassword] = useState('');
  const [formDisplayName, setFormDisplayName] = useState('');
  const [formRoleId, setFormRoleId] = useState<number>(3);
  const [formIsActive, setFormIsActive] = useState(true);

  const loadData = useCallback(async () => {
    if (!token) return;
    try {
      setLoading(true);
      setEmployees(await listEmployees(token, storeId));
    } catch (err) {
      alert(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setLoading(false);
    }
  }, [token, storeId, t]);

  useEffect(() => { loadData(); }, [loadData]);

  const filtered = useMemo(() => {
    if (!search.trim()) return employees;
    const q = search.toLowerCase();
    return employees.filter(e =>
      e.username.toLowerCase().includes(q) || e.display_name.toLowerCase().includes(q)
    );
  }, [employees, search]);

  const selectedId = panel.type === 'edit' ? panel.item.id : null;

  const openCreate = () => {
    setFormUsername(''); setFormPassword(''); setFormDisplayName('');
    setFormRoleId(3); setFormIsActive(true); setFormError('');
    setPanel({ type: 'create' });
  };

  const openEdit = (emp: Employee) => {
    if (emp.is_system) return;
    setFormUsername(emp.username); setFormPassword('');
    setFormDisplayName(emp.display_name); setFormRoleId(emp.role_id);
    setFormIsActive(emp.is_active); setFormError('');
    setPanel({ type: 'edit', item: emp });
  };

  const handleSave = async () => {
    if (!token) return;
    if (!formUsername.trim()) { setFormError(t('settings.common.required_field')); return; }
    if (panel.type === 'create' && !formPassword.trim()) { setFormError(t('settings.common.required_field')); return; }

    setSaving(true);
    setFormError('');
    try {
      if (panel.type === 'edit') {
        const payload: EmployeeUpdate = {
          username: formUsername.trim(),
          display_name: formDisplayName.trim() || undefined,
          role_id: formRoleId,
          is_active: formIsActive,
        };
        if (formPassword.trim()) payload.password = formPassword.trim();
        await updateEmployee(token, storeId, panel.item.id, payload);
      } else if (panel.type === 'create') {
        const payload: EmployeeCreate = {
          username: formUsername.trim(),
          password: formPassword.trim(),
          display_name: formDisplayName.trim() || undefined,
          role_id: formRoleId,
        };
        await createEmployee(token, storeId, payload);
      }
      setPanel({ type: 'closed' });
      await loadData();
    } catch (err) {
      setFormError(err instanceof ApiError ? err.message : t('auth.error_generic'));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async () => {
    if (!token || panel.type !== 'delete') return;
    try {
      await deleteEmployee(token, storeId, panel.item.id);
      setPanel({ type: 'closed' });
      await loadData();
    } catch (err) {
      alert(err instanceof ApiError ? err.message : t('auth.error_generic'));
    }
  };

  const renderItem = (emp: Employee, isSelected: boolean) => (
    <div className={`px-4 py-3.5 flex items-center gap-3 ${isSelected ? 'font-medium' : ''}`}>
      <div className="w-8 h-8 rounded-full bg-orange-100 text-orange-700 flex items-center justify-center text-xs font-bold shrink-0">
        {getInitials(emp.display_name || emp.username)}
      </div>
      <div className="flex-1 min-w-0">
        <div className={`text-sm truncate ${emp.is_active ? 'text-slate-900' : 'text-slate-400 line-through'}`}>
          {emp.display_name || emp.username}
        </div>
        <div className="text-xs text-gray-400 truncate">@{emp.username}</div>
      </div>
      <span className={`text-[10px] px-1.5 py-0.5 rounded font-medium shrink-0 ${roleBadge(emp.role_id, t).cls}`}>
        {roleBadge(emp.role_id, t).label}
      </span>
    </div>
  );

  return (
    <div className="h-full flex flex-col p-4 lg:p-6">
      <div className="flex items-center gap-3 mb-4 shrink-0">
        <div className="w-10 h-10 bg-orange-100 rounded-xl flex items-center justify-center">
          <Users className="w-5 h-5 text-orange-600" />
        </div>
        <div>
          <h1 className="text-xl font-bold text-slate-900">{t('settings.employee.title')}</h1>
          <p className="text-xs text-gray-400">{t('settings.employee.subtitle')}</p>
        </div>
      </div>

      <div className="flex-1 min-h-0">
        <MasterDetail
          items={filtered}
          getItemId={(emp) => emp.id}
          renderItem={renderItem}
          selectedId={selectedId}
          onSelect={openEdit}
          onDeselect={() => setPanel({ type: 'closed' })}
          searchQuery={search}
          onSearchChange={setSearch}
          totalCount={filtered.length}
          countUnit={t('settings.employee.unit')}
          onCreateNew={openCreate}
          createLabel={t('common.action.add')}
          isCreating={panel.type === 'create'}
          themeColor="orange"
          loading={loading}
        >
          {(panel.type === 'create' || panel.type === 'edit') && (
            <DetailPanel
              title={panel.type === 'create' ? `${t('common.action.add')} ${t('settings.employee.title')}` : `${t('common.action.edit')} ${t('settings.employee.title')}`}
              isCreating={panel.type === 'create'}
              onClose={() => setPanel({ type: 'closed' })}
              onSave={handleSave}
              onDelete={panel.type === 'edit' && !panel.item.is_system ? () => setPanel({ type: 'delete', item: panel.item }) : undefined}
              saving={saving}
              saveDisabled={!formUsername.trim() || (panel.type === 'create' && !formPassword.trim())}
            >
              {formError && (
                <div className="p-3 bg-red-50 border border-red-200 rounded-xl text-sm text-red-600">{formError}</div>
              )}

              <FormField label={t('settings.employee.username')} required>
                <input value={formUsername} onChange={e => setFormUsername(e.target.value)} className={inputClass} autoFocus />
              </FormField>

              <FormField label={t('settings.employee.password')} required={panel.type === 'create'}>
                <input
                  type="password"
                  value={formPassword}
                  onChange={e => setFormPassword(e.target.value)}
                  className={inputClass}
                  placeholder={panel.type === 'edit' ? t('settings.employee.password_keep') : ''}
                />
              </FormField>

              <FormField label={t('settings.employee.display_name')}>
                <input value={formDisplayName} onChange={e => setFormDisplayName(e.target.value)} className={inputClass} />
              </FormField>

              <SelectField
                label={t('settings.employee.role')}
                value={formRoleId}
                onChange={v => setFormRoleId(Number(v))}
                options={roleOptions}
                required
              />

              {panel.type === 'edit' && (
                <CheckboxField
                  id="employee-is-active"
                  label={t('settings.common.active')}
                  checked={formIsActive}
                  onChange={setFormIsActive}
                />
              )}
            </DetailPanel>
          )}
        </MasterDetail>
      </div>

      <ConfirmDialog
        isOpen={panel.type === 'delete'}
        title={t('common.dialog.confirm_delete')}
        description={t('settings.employee.confirm.delete')}
        onConfirm={handleDelete}
        onCancel={() => setPanel({ type: 'closed' })}
        variant="danger"
      />
    </div>
  );
};
