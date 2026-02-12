import React, { useEffect, useState, useCallback } from 'react';
import { Users, Plus, X, Settings } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { Permission } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { FilterBar } from '@/shared/components/FilterBar';
import { useMarketingGroupStore } from './store';
import {
  createMarketingGroup,
  updateMarketingGroup,
  deleteMarketingGroup,
  getMarketingGroupDetail,
  deleteDiscountRule,
  toggleStampActivity,
} from './mutations';
import { GroupListPanel, GroupDetailPanel, StampEditModal, DiscountRuleEditModal } from './components';
import { DiscountRuleWizard } from './DiscountRuleWizard';
import { StampActivityWizard } from './StampActivityWizard';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import type {
  MarketingGroup,
  MarketingGroupDetail,
  MgDiscountRule,
  StampActivityDetail,
} from '@/core/domain/types/api';

export const MarketingGroupManagement: React.FC = React.memo(() => {
  const { t } = useI18n();
  const { hasPermission } = usePermission();
  const canManage = hasPermission(Permission.MARKETING_MANAGE);

  const store = useMarketingGroupStore();
  const groups = store.items;
  const loading = store.isLoading;

  const [searchQuery, setSearchQuery] = useState('');
  const [selectedGroupId, setSelectedGroupId] = useState<number | null>(null);
  const [detail, setDetail] = useState<MarketingGroupDetail | null>(null);

  // Modal states
  const [showGroupForm, setShowGroupForm] = useState(false);
  const [editingGroup, setEditingGroup] = useState<MarketingGroup | null>(null);
  const [showRuleForm, setShowRuleForm] = useState(false);
  const [editingRule, setEditingRule] = useState<MgDiscountRule | null>(null);
  const [showStampForm, setShowStampForm] = useState(false);
  const [editingStamp, setEditingStamp] = useState<StampActivityDetail | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{
    type: 'group' | 'rule';
    id: number;
    name: string;
  } | null>(null);

  useEffect(() => {
    store.fetchAll();
  }, []);

  useEffect(() => {
    if (!selectedGroupId && groups.length > 0) {
      setSelectedGroupId(groups[0].id);
    }
  }, [groups, selectedGroupId]);

  useEffect(() => {
    if (selectedGroupId) {
      getMarketingGroupDetail(selectedGroupId)
        .then(setDetail)
        .catch((e) => logger.error('Failed to load MG detail', e));
    } else {
      setDetail(null);
    }
  }, [selectedGroupId]);

  const refreshDetail = useCallback(async () => {
    if (selectedGroupId) {
      const d = await getMarketingGroupDetail(selectedGroupId);
      setDetail(d);
    }
  }, [selectedGroupId]);

  // ── Group CRUD ──
  const handleSaveGroup = async (data: { name: string; display_name: string; description?: string }) => {
    try {
      if (editingGroup) {
        const updated = await updateMarketingGroup(editingGroup.id, data);
        useMarketingGroupStore.setState((s) => ({
          items: s.items.map((g) => (g.id === editingGroup.id ? updated : g)),
        }));
      } else {
        const created = await createMarketingGroup(data);
        useMarketingGroupStore.setState((s) => ({
          items: [...s.items, created],
        }));
        setSelectedGroupId(created.id);
      }
      toast.success(t('common.message.save_success'));
      setShowGroupForm(false);
      setEditingGroup(null);
      await refreshDetail();
    } catch (e) {
      logger.error('Failed to save marketing group', e);
      toast.error(t('common.message.save_failed'));
    }
  };

  // ── Delete handler ──
  const handleDelete = async () => {
    if (!deleteConfirm) return;
    try {
      if (deleteConfirm.type === 'group') {
        await deleteMarketingGroup(deleteConfirm.id);
        useMarketingGroupStore.setState((s) => ({
          items: s.items.filter((g) => g.id !== deleteConfirm.id),
        }));
        if (selectedGroupId === deleteConfirm.id) setSelectedGroupId(null);
      } else if (deleteConfirm.type === 'rule' && selectedGroupId) {
        await deleteDiscountRule(selectedGroupId, deleteConfirm.id);
        await refreshDetail();
      }
      toast.success(t('common.message.delete_success'));
    } catch (e) {
      logger.error('Failed to delete', e);
      toast.error(t('common.message.delete_failed'));
    } finally {
      setDeleteConfirm(null);
    }
  };

  const handleAddGroup = () => {
    setEditingGroup(null);
    setShowGroupForm(true);
  };

  // Empty state
  if (!loading && groups.length === 0) {
    return (
      <div className="space-y-5">
        <div className="bg-white rounded-xl border border-gray-200 p-5 shadow-sm">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-3">
              <div className="w-10 h-10 bg-violet-100 rounded-xl flex items-center justify-center">
                <Users size={20} className="text-violet-600" />
              </div>
              <div>
                <h2 className="text-lg font-bold text-gray-900">{t('settings.marketing_group.title')}</h2>
                <p className="text-sm text-gray-500">{t('settings.marketing_group.description')}</p>
              </div>
            </div>
            <ProtectedGate permission={Permission.MARKETING_MANAGE}>
              <button
                onClick={handleAddGroup}
                className="inline-flex items-center gap-2 px-4 py-2.5 bg-violet-600 text-white rounded-xl text-sm font-semibold shadow-lg shadow-violet-600/20 hover:bg-violet-700 hover:shadow-violet-600/30 transition-all"
              >
                <Plus size={16} />
                <span>{t('settings.marketing_group.add')}</span>
              </button>
            </ProtectedGate>
          </div>
        </div>

        <div className="flex flex-col items-center justify-center py-20 text-gray-400">
          <Settings size={64} className="mb-4 opacity-50" />
          <p className="text-lg mb-4">{t('settings.marketing_group.empty')}</p>
          {canManage && (
            <button
              onClick={handleAddGroup}
              className="px-6 py-3 bg-violet-600 text-white rounded-xl font-medium hover:bg-violet-700 transition-colors"
            >
              {t('settings.marketing_group.add')}
            </button>
          )}
        </div>

        {showGroupForm && (
          <GroupFormModal
            group={editingGroup}
            onSave={handleSaveGroup}
            onClose={() => { setShowGroupForm(false); setEditingGroup(null); }}
            t={t}
          />
        )}
      </div>
    );
  }

  return (
    <div className="space-y-5 h-full flex flex-col">
      {/* Custom Header */}
      <div className="bg-white rounded-xl border border-gray-200 p-5 shadow-sm">
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <div className="w-10 h-10 bg-violet-100 rounded-xl flex items-center justify-center">
              <Users size={20} className="text-violet-600" />
            </div>
            <div>
              <h2 className="text-lg font-bold text-gray-900">{t('settings.marketing_group.title')}</h2>
              <p className="text-sm text-gray-500">{t('settings.marketing_group.description')}</p>
            </div>
          </div>
          <ProtectedGate permission={Permission.MARKETING_MANAGE}>
            <button
              onClick={handleAddGroup}
              className="inline-flex items-center gap-2 px-4 py-2.5 bg-violet-600 text-white rounded-xl text-sm font-semibold shadow-lg shadow-violet-600/20 hover:bg-violet-700 hover:shadow-violet-600/30 transition-all"
            >
              <Plus size={16} />
              <span>{t('settings.marketing_group.add')}</span>
            </button>
          </ProtectedGate>
        </div>
      </div>

      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder={t('common.hint.search_placeholder')}
        totalCount={groups.length}
        countUnit={t('settings.marketing_group.unit')}
        themeColor="purple"
      />

      {/* Master-Detail Layout */}
      <div className="flex bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden divide-x divide-gray-200 h-[calc(100vh-320px)] min-h-[400px]">
        {/* Left Panel */}
        <GroupListPanel
          groups={groups}
          selectedGroupId={selectedGroupId}
          onSelectGroup={setSelectedGroupId}
          searchQuery={searchQuery}
        />

        {/* Right Panel */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {detail ? (
            <div className="flex-1 overflow-y-auto">
              <GroupDetailPanel
                detail={detail}
                onEditGroup={() => { setEditingGroup(detail); setShowGroupForm(true); }}
                onDeleteGroup={() => setDeleteConfirm({ type: 'group', id: detail.id, name: detail.display_name })}
                onAddRule={() => { setEditingRule(null); setShowRuleForm(true); }}
                onEditRule={(rule) => { setEditingRule(rule); setShowRuleForm(true); }}
                onDeleteRule={(rule) => setDeleteConfirm({ type: 'rule', id: rule.id, name: rule.display_name })}
                onAddStamp={() => { setEditingStamp(null); setShowStampForm(true); }}
                onEditStamp={(activity) => { setEditingStamp(activity); }}
                onToggleStamp={async (activity) => {
                  try {
                    await toggleStampActivity(selectedGroupId!, activity.id, !activity.is_active);
                    await refreshDetail();
                    toast.success(t(activity.is_active ? 'common.message.disabled' : 'common.message.enabled'));
                  } catch (e) {
                    logger.error('Failed to toggle stamp activity', e);
                    toast.error(t('common.message.save_failed'));
                  }
                }}

              />
            </div>
          ) : (
            <div className="flex-1 flex items-center justify-center bg-white">
              <div className="text-center text-gray-400">
                <Users size={48} className="mx-auto mb-3 opacity-50" />
                <p>{t('settings.marketing_group.select_group')}</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* ── Modals ── */}

      {showGroupForm && (
        <GroupFormModal
          group={editingGroup}
          onSave={handleSaveGroup}
          onClose={() => { setShowGroupForm(false); setEditingGroup(null); }}
          t={t}
        />
      )}

      {selectedGroupId && showRuleForm && !editingRule && (
        <DiscountRuleWizard
          isOpen={showRuleForm}
          groupId={selectedGroupId}
          onClose={() => { setShowRuleForm(false); }}
          onSuccess={() => { setShowRuleForm(false); refreshDetail(); }}
        />
      )}

      {selectedGroupId && editingRule && (
        <DiscountRuleEditModal
          rule={editingRule}
          groupId={selectedGroupId}
          onClose={() => { setEditingRule(null); }}
          onSuccess={() => { setEditingRule(null); refreshDetail(); }}
        />
      )}

      {selectedGroupId && showStampForm && !editingStamp && (
        <StampActivityWizard
          isOpen={showStampForm}
          groupId={selectedGroupId}
          onClose={() => { setShowStampForm(false); }}
          onSuccess={() => { setShowStampForm(false); refreshDetail(); }}
        />
      )}

      {selectedGroupId && editingStamp && (
        <StampEditModal
          activity={editingStamp}
          groupId={selectedGroupId}
          onClose={() => { setEditingStamp(null); }}
          onSuccess={() => { setEditingStamp(null); refreshDetail(); }}
        />
      )}

      {/* Delete Confirmation */}
      {deleteConfirm && (
        <div className="fixed inset-0 z-90 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl max-w-sm w-full overflow-hidden animate-in zoom-in-95">
            <div className="p-6">
              <h3 className="text-lg font-bold text-gray-900 mb-2">
                {t('common.action.delete')}
              </h3>
              <p className="text-sm text-gray-600 mb-6">
                {t('common.confirm_delete', { name: deleteConfirm.name })}
              </p>
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
// Group Form Modal
// ============================================================================

const GroupFormModal: React.FC<{
  group: MarketingGroup | null;
  onSave: (data: { name: string; display_name: string; description?: string }) => void;
  onClose: () => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}> = ({ group, onSave, onClose, t }) => {
  const [name, setName] = useState(group?.name || '');
  const [displayName, setDisplayName] = useState(group?.display_name || '');
  const [description, setDescription] = useState(group?.description || '');

  return (
    <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-lg w-full overflow-hidden animate-in zoom-in-95">
        <div className="p-5 border-b border-gray-100 flex items-center justify-between">
          <h3 className="text-xl font-bold text-gray-800">
            {group ? t('settings.marketing_group.edit') : t('settings.marketing_group.add')}
          </h3>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-lg transition-colors">
            <X size={20} className="text-gray-400" />
          </button>
        </div>
        <div className="p-5 space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('settings.marketing_group.field.name')}
            </label>
            <input
              value={name}
              onChange={(e) => setName(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500"
              placeholder="vip"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('settings.marketing_group.field.display_name')}
            </label>
            <input
              value={displayName}
              onChange={(e) => setDisplayName(e.target.value)}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500"
              placeholder="VIP"
            />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('settings.marketing_group.field.description')}
            </label>
            <textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              rows={3}
              className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500"
            />
          </div>
        </div>
        <div className="p-5 border-t border-gray-100 flex gap-3">
          <button
            onClick={onClose}
            className="flex-1 px-4 py-3 bg-gray-200 text-gray-700 rounded-xl font-bold hover:bg-gray-300 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          <button
            onClick={() =>
              onSave({ name, display_name: displayName, description: description || undefined })
            }
            disabled={!name.trim() || !displayName.trim()}
            className="flex-1 px-4 py-3 bg-violet-500 text-white rounded-xl font-bold hover:bg-violet-600 disabled:opacity-50 transition-colors"
          >
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
};

