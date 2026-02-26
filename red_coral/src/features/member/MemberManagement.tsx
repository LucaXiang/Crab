import React, { useEffect, useState, useMemo } from 'react';
import { UserCheck, Plus, X, Phone, Users, CreditCard, Star, Calendar, Mail, Wallet } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useMemberStore } from './store';
import { useMarketingGroupStore } from '@/features/marketing-group/store';
import { createMember, updateMember, deleteMember } from './mutations';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { useModalState } from '@/shared/hooks/useModalState';
import { Permission } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';
import type { MemberWithGroup, MemberCreate, MemberUpdate } from '@/core/domain/types/api';
import { DataTable, Column } from '@/shared/components/DataTable';
import { ManagementHeader, FilterBar } from '@/screens/Settings/components';
import { FormField, FormSection, SelectField, inputClass, WheelDatePicker } from '@/shared/components/FormField';
import { MAX_NAME_LEN, MAX_SHORT_TEXT_LEN, MAX_EMAIL_LEN, MAX_NOTE_LEN } from '@/shared/constants/validation';
import { formatCurrency } from '@/utils/currency';

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
  const memberForm = useModalState<MemberWithGroup>();
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
        const updated = await updateMember(id, data);
        useMemberStore.setState((s) => ({
          items: s.items.map((i) => (i.id === id ? updated : i)),
        }));
      } else {
        const created = await createMember(data as MemberCreate);
        useMemberStore.setState((s) => ({
          items: [...s.items, created],
        }));
      }
      toast.success(t('common.message.save_success'));
      memberForm.close();
    } catch (e) {
      logger.error('Failed to save member', e);
      toast.error(t('common.message.save_failed'));
    }
  };

  const handleDelete = async () => {
    if (!deleteConfirm) return;
    try {
      await deleteMember(deleteConfirm.id);
      useMemberStore.setState((s) => ({
        items: s.items.filter((i) => i.id !== deleteConfirm.id),
      }));
      toast.success(t('common.message.delete_success'));
      setDeleteConfirm(null);
    } catch (e) {
      logger.error('Failed to delete member', e);
      toast.error(t('common.message.delete_failed'));
    }
  };

  const columns: Column<MemberWithGroup>[] = useMemo(() => [
    {
      key: 'name',
      header: t('settings.member.field.name'),
      render: (m) => {
        const initial = m.name.charAt(0).toUpperCase();
        return (
          <div className="flex items-center gap-3">
            <div className="w-9 h-9 rounded-full bg-gradient-to-br from-teal-400 to-teal-600 flex items-center justify-center text-white font-bold text-sm shrink-0 shadow-sm">
              {initial}
            </div>
            <div className="min-w-0">
              <div className="font-semibold text-gray-900 truncate">{m.name}</div>
              {m.card_number && (
                <div className="flex items-center gap-1 mt-0.5">
                  <CreditCard size={10} className="text-gray-400 shrink-0" />
                  <span className="text-[0.6875rem] text-gray-400 font-mono">{m.card_number}</span>
                </div>
              )}
            </div>
          </div>
        );
      },
    },
    {
      key: 'phone',
      header: t('settings.member.field.phone'),
      width: '150px',
      render: (m) => m.phone ? (
        <div className="flex items-center gap-1.5">
          <Phone size={13} className="text-gray-400 shrink-0" />
          <span className="text-sm text-gray-700 font-medium">{m.phone}</span>
        </div>
      ) : (
        <span className="text-sm text-gray-300">-</span>
      ),
    },
    {
      key: 'group',
      header: t('settings.member.field.group'),
      width: '160px',
      render: (m) => (
        <span className="inline-flex items-center gap-1 text-xs bg-violet-100 text-violet-700 px-2.5 py-1 rounded-full font-semibold">
          <Users size={12} className="shrink-0" />
          {m.marketing_group_name}
        </span>
      ),
    },
    {
      key: 'points',
      header: t('settings.member.field.points'),
      width: '100px',
      align: 'center',
      render: (m) => (
        <div className="flex items-center justify-center gap-1">
          <Star size={13} className="text-amber-400 shrink-0" />
          <span className="text-sm font-bold text-gray-700 tabular-nums">{m.points_balance}</span>
        </div>
      ),
    },
    {
      key: 'total_spent',
      header: t('settings.member.field.total_spent'),
      width: '120px',
      align: 'right',
      render: (m) => (
        <span className="text-sm font-semibold text-gray-700 tabular-nums">
          {formatCurrency(m.total_spent)}
        </span>
      ),
    },
    {
      key: 'status',
      header: t('common.label.status'),
      width: '80px',
      align: 'center',
      render: (m) => (
        <span className={`inline-flex items-center gap-1 text-xs px-2 py-0.5 rounded-full font-semibold ${
          m.is_active ? 'bg-emerald-100 text-emerald-700' : 'bg-gray-100 text-gray-500'
        }`}>
          <span className={`w-1.5 h-1.5 rounded-full ${m.is_active ? 'bg-emerald-500' : 'bg-gray-400'}`} />
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
        onAdd={() => memberForm.open()}
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
        onEdit={canManage ? (m) => memberForm.open(m) : undefined}
        onDelete={canManage ? (m) => setDeleteConfirm(m) : undefined}
        emptyText={t('common.empty.no_data')}
        themeColor="teal"
      />

      {/* Member Form Modal */}
      {memberForm.isOpen && (
        <MemberFormModal
          member={memberForm.editing}
          groups={groups}
          onSave={handleSave}
          onClose={memberForm.close}
          t={t}
        />
      )}

      {/* Delete Confirm */}
      {deleteConfirm && (
        <div className="fixed inset-0 z-90 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl max-w-sm w-full overflow-hidden animate-in zoom-in-95">
            <div className="p-6">
              <h3 className="text-lg font-bold text-gray-900 mb-2">{t('common.action.delete')}</h3>
              <p className="text-sm text-gray-600 mb-6">{t('common.confirm_delete', { name: deleteConfirm.name })}</p>
              <div className="grid grid-cols-2 gap-3">
                <button
                  onClick={() => setDeleteConfirm(null)}
                  className="w-full py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-200 transition-colors"
                >
                  {t('common.action.cancel')}
                </button>
                <button
                  onClick={handleDelete}
                  className="w-full py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors shadow-lg shadow-red-600/20"
                >
                  {t('common.action.delete')}
                </button>
              </div>
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
  const [email, setEmail] = useState(member?.email || '');
  const [groupId, setGroupId] = useState<number>(member?.marketing_group_id || groups[0]?.id || 0);
  const [birthday, setBirthday] = useState(member?.birthday || '');
  const [notes, setNotes] = useState(member?.notes || '');

  return (
    <div className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-gray-50 rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden animate-in zoom-in-95 duration-200">
        {/* Header */}
        <div className="px-6 py-4 border-b border-gray-200 bg-white">
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-bold text-gray-900">
              {member ? t('settings.member.edit') : t('settings.member.add')}
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
        <div className="p-4 space-y-4 max-h-[70vh] overflow-y-auto">
          <FormField label={t('settings.member.field.name')} required>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              maxLength={MAX_NAME_LEN}
              className={inputClass}
            />
          </FormField>

          <FormSection title={t('settings.member.section.contact')} icon={Phone}>
            <FormField label={t('settings.member.field.phone')}>
              <input
                value={phone}
                onChange={(e) => setPhone(e.target.value)}
                maxLength={MAX_SHORT_TEXT_LEN}
                className={inputClass}
              />
            </FormField>

            <FormField label={t('settings.member.field.card_number')}>
              <input
                value={cardNumber}
                onChange={(e) => setCardNumber(e.target.value)}
                maxLength={MAX_SHORT_TEXT_LEN}
                className={inputClass}
              />
            </FormField>

            <FormField label={t('settings.member.field.email')}>
              <input
                type="email"
                value={email}
                onChange={(e) => setEmail(e.target.value)}
                maxLength={MAX_EMAIL_LEN}
                className={inputClass}
              />
            </FormField>
          </FormSection>

          <FormSection title={t('settings.member.section.membership')} icon={Users}>
            <SelectField
              label={t('settings.member.field.group')}
              required
              value={groupId}
              onChange={(v) => setGroupId(Number(v))}
              options={groups.map((g) => ({ value: g.id, label: g.name }))}
            />

            <FormField label={t('settings.member.field.birthday')}>
              <WheelDatePicker
                value={birthday}
                onChange={setBirthday}
                placeholder={t('settings.member.field.birthday')}
              />
            </FormField>

            <FormField label={t('settings.member.field.notes')}>
              <textarea
                value={notes}
                onChange={(e) => setNotes(e.target.value)}
                maxLength={MAX_NOTE_LEN}
                rows={2}
                className={inputClass}
              />
            </FormField>
          </FormSection>
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-gray-200 bg-white flex justify-end gap-3">
          <button
            onClick={onClose}
            className="px-5 py-2.5 bg-white border border-gray-200 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-50 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          <button
            onClick={() => onSave({
              name,
              phone: phone || null,
              card_number: cardNumber || null,
              email: email || null,
              marketing_group_id: groupId,
              birthday: birthday || null,
              notes: notes || null,
            }, member?.id)}
            disabled={!name.trim() || !groupId}
            className="px-5 py-2.5 bg-teal-600 text-white rounded-xl text-sm font-semibold hover:bg-teal-700 transition-colors shadow-lg shadow-teal-600/20 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
