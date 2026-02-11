import React, { useEffect, useState, useMemo } from 'react';
import { UserCheck, Plus, X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useMemberStore } from './store';
import { useMarketingGroupStore } from '@/features/marketing-group/store';
import { createMember, updateMember, deleteMember } from './mutations';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { Permission } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';
import type { MemberWithGroup, MemberCreate, MemberUpdate } from '@/core/domain/types/api';
import { DataTable, Column } from '@/shared/components/DataTable';
import { ManagementHeader, FilterBar } from '@/screens/Settings/components';

export const MemberManagement: React.FC = React.memo(() => {
  const { t } = useI18n();
  const { hasPermission } = usePermission();
  const canManage = hasPermission(Permission.MARKETING_MANAGE);

  const memberStore = useMemberStore();
  const members = memberStore.items;
  const loading = memberStore.isLoading;
  const mgStore = useMarketingGroupStore();
  const groups = mgStore.items;

  const [searchQuery, setSearchQuery] = useState('');
  const [showForm, setShowForm] = useState(false);
  const [editingMember, setEditingMember] = useState<MemberWithGroup | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<MemberWithGroup | null>(null);

  useEffect(() => {
    memberStore.fetchAll();
    mgStore.fetchAll();
  }, []);

  const filtered = useMemo(() => {
    if (!searchQuery.trim()) return members;
    const q = searchQuery.toLowerCase();
    return members.filter((m) =>
      m.name.toLowerCase().includes(q) ||
      m.phone?.toLowerCase().includes(q) ||
      m.card_number?.toLowerCase().includes(q) ||
      m.marketing_group_name.toLowerCase().includes(q)
    );
  }, [members, searchQuery]);

  const handleSave = async (data: MemberCreate | MemberUpdate, id?: number) => {
    try {
      if (id) {
        await updateMember(id, data);
      } else {
        await createMember(data as MemberCreate);
      }
      toast.success(t('common.message.save_success'));
      setShowForm(false);
      setEditingMember(null);
      await memberStore.fetchAll();
    } catch (e) {
      logger.error('Failed to save member', e);
      toast.error(t('common.message.save_failed'));
    }
  };

  const handleDelete = async () => {
    if (!deleteConfirm) return;
    try {
      await deleteMember(deleteConfirm.id);
      toast.success(t('common.message.delete_success'));
      setDeleteConfirm(null);
      await memberStore.fetchAll();
    } catch (e) {
      logger.error('Failed to delete member', e);
      toast.error(t('common.message.delete_failed'));
    }
  };

  const columns: Column<MemberWithGroup>[] = useMemo(() => [
    {
      key: 'name',
      header: t('settings.member.field.name'),
      render: (m) => (
        <div>
          <div className="font-medium text-gray-800">{m.name}</div>
          {m.card_number && <div className="text-xs text-gray-400">{m.card_number}</div>}
        </div>
      ),
    },
    {
      key: 'phone',
      header: t('settings.member.field.phone'),
      width: '140px',
      render: (m) => <span className="text-sm text-gray-600">{m.phone || '-'}</span>,
    },
    {
      key: 'group',
      header: t('settings.member.field.group'),
      width: '160px',
      render: (m) => (
        <span className="text-sm bg-violet-100 text-violet-700 px-2 py-0.5 rounded-full font-medium">
          {m.marketing_group_name}
        </span>
      ),
    },
    {
      key: 'points',
      header: t('settings.member.field.points'),
      width: '100px',
      align: 'center',
      render: (m) => <span className="text-sm font-mono text-gray-600">{m.points_balance}</span>,
    },
    {
      key: 'status',
      header: t('common.label.status'),
      width: '80px',
      align: 'center',
      render: (m) => (
        <span className={`text-xs px-2 py-0.5 rounded-full font-bold ${
          m.is_active ? 'bg-green-100 text-green-700' : 'bg-gray-100 text-gray-500'
        }`}>
          {m.is_active ? t('common.status.active') : t('common.status.inactive')}
        </span>
      ),
    },
  ], [t]);

  return (
    <div className="space-y-5">
      <ManagementHeader
        icon={UserCheck}
        title={t('settings.member.title')}
        description={t('settings.member.description')}
        addButtonText={t('settings.member.add')}
        onAdd={() => { setEditingMember(null); setShowForm(true); }}
        themeColor="teal"
        permission={Permission.MARKETING_MANAGE}
      />

      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder={t('settings.member.search_placeholder')}
        totalCount={filtered.length}
        countUnit={t('settings.member.unit')}
        themeColor="teal"
      />

      <DataTable
        data={filtered}
        columns={columns}
        loading={loading}
        getRowKey={(m) => m.id}
        onEdit={canManage ? (m) => { setEditingMember(m); setShowForm(true); } : undefined}
        onDelete={canManage ? (m) => setDeleteConfirm(m) : undefined}
        emptyText={t('common.empty.no_data')}
        themeColor="teal"
      />

      {/* Member Form Modal */}
      {showForm && (
        <MemberFormModal
          member={editingMember}
          groups={groups}
          onSave={handleSave}
          onClose={() => { setShowForm(false); setEditingMember(null); }}
          t={t}
        />
      )}

      {/* Delete Confirm */}
      {deleteConfirm && (
        <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl max-w-md w-full p-6">
            <h3 className="text-xl font-bold text-gray-800 mb-2">{t('common.action.delete')}</h3>
            <p className="text-gray-600 mb-6">{t('common.confirm_delete', { name: deleteConfirm.name })}</p>
            <div className="flex gap-3">
              <button onClick={() => setDeleteConfirm(null)} className="flex-1 px-4 py-3 bg-gray-200 text-gray-700 rounded-xl font-bold">{t('common.action.cancel')}</button>
              <button onClick={handleDelete} className="flex-1 px-4 py-3 bg-red-500 text-white rounded-xl font-bold hover:bg-red-600">{t('common.action.delete')}</button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
});

// ============================================================================
// Member Form Modal
// ============================================================================

import type { MarketingGroup } from '@/core/domain/types/api';

const MemberFormModal: React.FC<{
  member: MemberWithGroup | null;
  groups: MarketingGroup[];
  onSave: (data: any, id?: number) => void;
  onClose: () => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}> = ({ member, groups, onSave, onClose, t }) => {
  const [name, setName] = useState(member?.name || '');
  const [phone, setPhone] = useState(member?.phone || '');
  const [cardNumber, setCardNumber] = useState(member?.card_number || '');
  const [groupId, setGroupId] = useState<number>(member?.marketing_group_id || groups[0]?.id || 0);
  const [birthday, setBirthday] = useState(member?.birthday || '');
  const [notes, setNotes] = useState(member?.notes || '');

  return (
    <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-lg w-full p-6 max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-xl font-bold text-gray-800">
            {member ? t('settings.member.edit') : t('settings.member.add')}
          </h3>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-full"><X size={20} /></button>
        </div>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.member.field.name')} *</label>
            <input value={name} onChange={(e) => setName(e.target.value)} className="w-full px-3 py-2 border border-gray-300 rounded-lg" />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.member.field.phone')}</label>
            <input value={phone} onChange={(e) => setPhone(e.target.value)} className="w-full px-3 py-2 border border-gray-300 rounded-lg" />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.member.field.card_number')}</label>
            <input value={cardNumber} onChange={(e) => setCardNumber(e.target.value)} className="w-full px-3 py-2 border border-gray-300 rounded-lg" />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.member.field.group')} *</label>
            <select value={groupId} onChange={(e) => setGroupId(Number(e.target.value))} className="w-full px-3 py-2 border border-gray-300 rounded-lg">
              {groups.map((g) => (
                <option key={g.id} value={g.id}>{g.display_name}</option>
              ))}
            </select>
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.member.field.birthday')}</label>
            <input type="date" value={birthday} onChange={(e) => setBirthday(e.target.value)} className="w-full px-3 py-2 border border-gray-300 rounded-lg" />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.member.field.notes')}</label>
            <textarea value={notes} onChange={(e) => setNotes(e.target.value)} rows={2} className="w-full px-3 py-2 border border-gray-300 rounded-lg" />
          </div>
        </div>
        <div className="flex gap-3 mt-6">
          <button onClick={onClose} className="flex-1 px-4 py-3 bg-gray-200 text-gray-700 rounded-xl font-bold">{t('common.action.cancel')}</button>
          <button
            onClick={() => onSave({
              name,
              phone: phone || null,
              card_number: cardNumber || null,
              marketing_group_id: groupId,
              birthday: birthday || null,
              notes: notes || null,
            }, member?.id)}
            disabled={!name.trim() || !groupId}
            className="flex-1 px-4 py-3 bg-teal-500 text-white rounded-xl font-bold hover:bg-teal-600 disabled:opacity-50"
          >
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
