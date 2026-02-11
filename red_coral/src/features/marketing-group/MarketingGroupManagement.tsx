import React, { useEffect, useState, useMemo, useCallback } from 'react';
import { Users, Plus, Pencil, Trash2, ChevronRight, ChevronDown, X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useMarketingGroupStore } from './store';
import {
  createMarketingGroup,
  updateMarketingGroup,
  deleteMarketingGroup,
  getMarketingGroupDetail,
  createDiscountRule,
  updateDiscountRule,
  deleteDiscountRule,
} from './mutations';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { formatCurrency } from '@/utils/currency';
import type { MarketingGroup, MarketingGroupDetail, MgDiscountRule, MgDiscountRuleCreate, ProductScope, AdjustmentType } from '@/core/domain/types/api';
import { ManagementHeader, FilterBar } from '@/screens/Settings/components';

// ============================================================================
// Main Management Component
// ============================================================================

export const MarketingGroupManagement: React.FC = React.memo(() => {
  const { t } = useI18n();
  const store = useMarketingGroupStore();
  const groups = store.items;
  const loading = store.isLoading;

  const [searchQuery, setSearchQuery] = useState('');
  const [selectedGroupId, setSelectedGroupId] = useState<number | null>(null);
  const [detail, setDetail] = useState<MarketingGroupDetail | null>(null);
  const [showGroupForm, setShowGroupForm] = useState(false);
  const [editingGroup, setEditingGroup] = useState<MarketingGroup | null>(null);
  const [showRuleForm, setShowRuleForm] = useState(false);
  const [editingRule, setEditingRule] = useState<MgDiscountRule | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<{ type: 'group' | 'rule'; id: number; name: string } | null>(null);

  useEffect(() => { store.fetchAll(); }, []);

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
    await store.fetchAll();
  }, [selectedGroupId]);

  const filteredGroups = useMemo(() => {
    if (!searchQuery.trim()) return groups;
    const q = searchQuery.toLowerCase();
    return groups.filter((g) => g.display_name.toLowerCase().includes(q) || g.name.toLowerCase().includes(q));
  }, [groups, searchQuery]);

  const handleSaveGroup = async (data: { name: string; display_name: string; description?: string }) => {
    try {
      if (editingGroup) {
        await updateMarketingGroup(editingGroup.id, data);
        toast.success(t('common.message.save_success'));
      } else {
        const created = await createMarketingGroup(data);
        setSelectedGroupId(created.id);
        toast.success(t('common.message.save_success'));
      }
      setShowGroupForm(false);
      setEditingGroup(null);
      await refreshDetail();
    } catch (e) {
      logger.error('Failed to save marketing group', e);
      toast.error(t('common.message.save_failed'));
    }
  };

  const handleDeleteGroup = async () => {
    if (!deleteConfirm || deleteConfirm.type !== 'group') return;
    try {
      await deleteMarketingGroup(deleteConfirm.id);
      toast.success(t('common.message.delete_success'));
      if (selectedGroupId === deleteConfirm.id) setSelectedGroupId(null);
      setDeleteConfirm(null);
      await store.fetchAll();
    } catch (e) {
      logger.error('Failed to delete marketing group', e);
      toast.error(t('common.message.delete_failed'));
    }
  };

  const handleSaveRule = async (data: MgDiscountRuleCreate) => {
    if (!selectedGroupId) return;
    try {
      if (editingRule) {
        await updateDiscountRule(selectedGroupId, editingRule.id, data);
      } else {
        await createDiscountRule(selectedGroupId, data);
      }
      toast.success(t('common.message.save_success'));
      setShowRuleForm(false);
      setEditingRule(null);
      await refreshDetail();
    } catch (e) {
      logger.error('Failed to save discount rule', e);
      toast.error(t('common.message.save_failed'));
    }
  };

  const handleDeleteRule = async () => {
    if (!deleteConfirm || deleteConfirm.type !== 'rule' || !selectedGroupId) return;
    try {
      await deleteDiscountRule(selectedGroupId, deleteConfirm.id);
      toast.success(t('common.message.delete_success'));
      setDeleteConfirm(null);
      await refreshDetail();
    } catch (e) {
      logger.error('Failed to delete discount rule', e);
      toast.error(t('common.message.delete_failed'));
    }
  };

  return (
    <div className="space-y-5">
      <ManagementHeader
        icon={Users}
        title={t('settings.marketing_group.title')}
        description={t('settings.marketing_group.description')}
        addButtonText={t('settings.marketing_group.add')}
        onAdd={() => { setEditingGroup(null); setShowGroupForm(true); }}
        themeColor="purple"
        permission={Permission.MARKETING_MANAGE}
      />

      <div className="flex gap-6">
        {/* Left: Group List */}
        <div className="w-80 shrink-0 space-y-3">
          <FilterBar
            searchQuery={searchQuery}
            onSearchChange={setSearchQuery}
            searchPlaceholder={t('common.hint.search_placeholder')}
            totalCount={filteredGroups.length}
            countUnit={t('settings.marketing_group.unit')}
            themeColor="purple"
          />
          <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
            {loading ? (
              <div className="p-8 text-center text-gray-400">{t('common.loading')}</div>
            ) : filteredGroups.length === 0 ? (
              <div className="p-8 text-center text-gray-400">{t('common.empty.no_data')}</div>
            ) : (
              <div className="divide-y divide-gray-100">
                {filteredGroups.map((g) => (
                  <button
                    key={g.id}
                    onClick={() => setSelectedGroupId(g.id)}
                    className={`w-full text-left p-4 transition-colors flex items-center justify-between ${
                      selectedGroupId === g.id ? 'bg-violet-50 border-l-4 border-l-violet-500' : 'hover:bg-gray-50'
                    }`}
                  >
                    <div>
                      <div className="font-medium text-gray-800">{g.display_name}</div>
                      <div className="text-xs text-gray-400 mt-0.5">{g.name}</div>
                    </div>
                    <div className="flex items-center gap-2">
                      {!g.is_active && (
                        <span className="text-xs bg-gray-100 text-gray-500 px-2 py-0.5 rounded">{t('common.status.inactive')}</span>
                      )}
                      <ChevronRight size={16} className="text-gray-400" />
                    </div>
                  </button>
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Right: Detail Panel */}
        <div className="flex-1 min-w-0">
          {detail ? (
            <div className="space-y-4">
              {/* Group Info */}
              <div className="bg-white rounded-xl border border-gray-200 p-5">
                <div className="flex items-center justify-between mb-3">
                  <h3 className="text-lg font-bold text-gray-800">{detail.display_name}</h3>
                  <div className="flex gap-2">
                    <button
                      onClick={() => { setEditingGroup(detail); setShowGroupForm(true); }}
                      className="p-2 text-gray-500 hover:text-violet-600 hover:bg-violet-50 rounded-lg transition-colors"
                    >
                      <Pencil size={16} />
                    </button>
                    <button
                      onClick={() => setDeleteConfirm({ type: 'group', id: detail.id, name: detail.display_name })}
                      className="p-2 text-gray-500 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                    >
                      <Trash2 size={16} />
                    </button>
                  </div>
                </div>
                {detail.description && <p className="text-sm text-gray-500">{detail.description}</p>}
              </div>

              {/* Discount Rules */}
              <div className="bg-white rounded-xl border border-gray-200 overflow-hidden">
                <div className="p-4 border-b border-gray-100 flex items-center justify-between">
                  <h4 className="font-bold text-gray-700">{t('settings.marketing_group.rules')}</h4>
                  <button
                    onClick={() => { setEditingRule(null); setShowRuleForm(true); }}
                    className="px-3 py-1.5 bg-violet-500 text-white rounded-lg text-sm font-medium hover:bg-violet-600 transition-colors flex items-center gap-1"
                  >
                    <Plus size={14} /> {t('settings.marketing_group.add_rule')}
                  </button>
                </div>
                {detail.discount_rules.length === 0 ? (
                  <div className="p-8 text-center text-gray-400">{t('settings.marketing_group.no_rules')}</div>
                ) : (
                  <div className="divide-y divide-gray-100">
                    {detail.discount_rules.map((rule) => (
                      <div key={rule.id} className="p-4 flex items-center justify-between hover:bg-gray-50 transition-colors">
                        <div className="flex-1 min-w-0">
                          <div className="font-medium text-gray-800 flex items-center gap-2">
                            {rule.display_name}
                            <span className={`text-xs px-2 py-0.5 rounded-full font-bold ${
                              rule.adjustment_type === 'PERCENTAGE'
                                ? 'bg-blue-100 text-blue-700'
                                : 'bg-green-100 text-green-700'
                            }`}>
                              {rule.adjustment_type === 'PERCENTAGE'
                                ? `-${rule.adjustment_value}%`
                                : `-${formatCurrency(rule.adjustment_value)}`}
                            </span>
                            <span className={`text-xs px-2 py-0.5 rounded ${
                              rule.product_scope === 'GLOBAL' ? 'bg-gray-100 text-gray-600'
                              : rule.product_scope === 'CATEGORY' ? 'bg-amber-100 text-amber-700'
                              : 'bg-violet-100 text-violet-700'
                            }`}>
                              {rule.product_scope}
                            </span>
                            {!rule.is_active && (
                              <span className="text-xs bg-gray-100 text-gray-500 px-2 py-0.5 rounded">{t('common.status.inactive')}</span>
                            )}
                          </div>
                          <div className="text-xs text-gray-400 mt-0.5">
                            {t('settings.marketing_group.receipt_name')}: {rule.receipt_name}
                          </div>
                        </div>
                        <div className="flex gap-1 shrink-0 ml-4">
                          <button
                            onClick={() => { setEditingRule(rule); setShowRuleForm(true); }}
                            className="p-2 text-gray-400 hover:text-violet-600 hover:bg-violet-50 rounded-lg transition-colors"
                          >
                            <Pencil size={14} />
                          </button>
                          <button
                            onClick={() => setDeleteConfirm({ type: 'rule', id: rule.id, name: rule.display_name })}
                            className="p-2 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                          >
                            <Trash2 size={14} />
                          </button>
                        </div>
                      </div>
                    ))}
                  </div>
                )}
              </div>
            </div>
          ) : (
            <div className="bg-white rounded-xl border border-gray-200 p-12 text-center text-gray-400">
              {t('settings.marketing_group.select_group')}
            </div>
          )}
        </div>
      </div>

      {/* Group Form Modal */}
      {showGroupForm && (
        <GroupFormModal
          group={editingGroup}
          onSave={handleSaveGroup}
          onClose={() => { setShowGroupForm(false); setEditingGroup(null); }}
          t={t}
        />
      )}

      {/* Rule Form Modal */}
      {showRuleForm && (
        <RuleFormModal
          rule={editingRule}
          onSave={handleSaveRule}
          onClose={() => { setShowRuleForm(false); setEditingRule(null); }}
          t={t}
        />
      )}

      {/* Delete Confirm */}
      {deleteConfirm && (
        <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl max-w-md w-full p-6">
            <h3 className="text-xl font-bold text-gray-800 mb-2">{t('common.action.delete')}</h3>
            <p className="text-gray-600 mb-6">
              {t('common.confirm_delete', { name: deleteConfirm.name })}
            </p>
            <div className="flex gap-3">
              <button onClick={() => setDeleteConfirm(null)} className="flex-1 px-4 py-3 bg-gray-200 text-gray-700 rounded-xl font-bold">
                {t('common.action.cancel')}
              </button>
              <button
                onClick={deleteConfirm.type === 'group' ? handleDeleteGroup : handleDeleteRule}
                className="flex-1 px-4 py-3 bg-red-500 text-white rounded-xl font-bold hover:bg-red-600"
              >
                {t('common.action.delete')}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
});

// Need Permission import
import { Permission } from '@/core/domain/types';

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
    <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-lg w-full p-6">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-xl font-bold text-gray-800">
            {group ? t('settings.marketing_group.edit') : t('settings.marketing_group.add')}
          </h3>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-full"><X size={20} /></button>
        </div>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.marketing_group.field.name')}</label>
            <input value={name} onChange={(e) => setName(e.target.value)} className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500" placeholder="vip" />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.marketing_group.field.display_name')}</label>
            <input value={displayName} onChange={(e) => setDisplayName(e.target.value)} className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500" placeholder="VIP" />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.marketing_group.field.description')}</label>
            <textarea value={description} onChange={(e) => setDescription(e.target.value)} rows={3} className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-violet-500 focus:border-violet-500" />
          </div>
        </div>
        <div className="flex gap-3 mt-6">
          <button onClick={onClose} className="flex-1 px-4 py-3 bg-gray-200 text-gray-700 rounded-xl font-bold">{t('common.action.cancel')}</button>
          <button
            onClick={() => onSave({ name, display_name: displayName, description: description || undefined })}
            disabled={!name.trim() || !displayName.trim()}
            className="flex-1 px-4 py-3 bg-violet-500 text-white rounded-xl font-bold hover:bg-violet-600 disabled:opacity-50"
          >
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
};

// ============================================================================
// Rule Form Modal
// ============================================================================

const RuleFormModal: React.FC<{
  rule: MgDiscountRule | null;
  onSave: (data: any) => void;
  onClose: () => void;
  t: (key: string, params?: Record<string, string | number>) => string;
}> = ({ rule, onSave, onClose, t }) => {
  const [name, setName] = useState(rule?.name || '');
  const [displayName, setDisplayName] = useState(rule?.display_name || '');
  const [receiptName, setReceiptName] = useState(rule?.receipt_name || '');
  const [productScope, setProductScope] = useState<ProductScope>(rule?.product_scope || 'GLOBAL');
  const [targetId, setTargetId] = useState<string>(rule?.target_id?.toString() || '');
  const [adjustmentType, setAdjustmentType] = useState<AdjustmentType>(rule?.adjustment_type || 'PERCENTAGE');
  const [adjustmentValue, setAdjustmentValue] = useState(rule?.adjustment_value?.toString() || '');

  return (
    <div className="fixed inset-0 z-50 bg-black/50 flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl max-w-lg w-full p-6 max-h-[90vh] overflow-y-auto">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-xl font-bold text-gray-800">
            {rule ? t('settings.marketing_group.edit_rule') : t('settings.marketing_group.add_rule')}
          </h3>
          <button onClick={onClose} className="p-2 hover:bg-gray-100 rounded-full"><X size={20} /></button>
        </div>
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.marketing_group.field.name')}</label>
            <input value={name} onChange={(e) => setName(e.target.value)} className="w-full px-3 py-2 border border-gray-300 rounded-lg" placeholder="vip_discount" />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.marketing_group.field.display_name')}</label>
            <input value={displayName} onChange={(e) => setDisplayName(e.target.value)} className="w-full px-3 py-2 border border-gray-300 rounded-lg" placeholder="VIP 折扣" />
          </div>
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.marketing_group.receipt_name')}</label>
            <input value={receiptName} onChange={(e) => setReceiptName(e.target.value)} className="w-full px-3 py-2 border border-gray-300 rounded-lg" placeholder="VIP" />
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.marketing_group.field.scope')}</label>
              <select value={productScope} onChange={(e) => setProductScope(e.target.value as ProductScope)} className="w-full px-3 py-2 border border-gray-300 rounded-lg">
                <option value="GLOBAL">{t('settings.marketing_group.scope.global')}</option>
                <option value="CATEGORY">{t('settings.marketing_group.scope.category')}</option>
                <option value="PRODUCT">{t('settings.marketing_group.scope.product')}</option>
              </select>
            </div>
            {productScope !== 'GLOBAL' && (
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">
                  {productScope === 'CATEGORY' ? t('settings.marketing_group.field.category_id') : t('settings.marketing_group.field.product_id')}
                </label>
                <input value={targetId} onChange={(e) => setTargetId(e.target.value)} type="number" className="w-full px-3 py-2 border border-gray-300 rounded-lg" />
              </div>
            )}
          </div>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">{t('settings.marketing_group.field.adjustment_type')}</label>
              <select value={adjustmentType} onChange={(e) => setAdjustmentType(e.target.value as AdjustmentType)} className="w-full px-3 py-2 border border-gray-300 rounded-lg">
                <option value="PERCENTAGE">{t('settings.marketing_group.type.percentage')}</option>
                <option value="FIXED_AMOUNT">{t('settings.marketing_group.type.fixed')}</option>
              </select>
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">
                {adjustmentType === 'PERCENTAGE' ? t('settings.marketing_group.field.percent') : t('settings.marketing_group.field.amount')}
              </label>
              <input value={adjustmentValue} onChange={(e) => setAdjustmentValue(e.target.value)} type="number" step="0.01" className="w-full px-3 py-2 border border-gray-300 rounded-lg" />
            </div>
          </div>
        </div>
        <div className="flex gap-3 mt-6">
          <button onClick={onClose} className="flex-1 px-4 py-3 bg-gray-200 text-gray-700 rounded-xl font-bold">{t('common.action.cancel')}</button>
          <button
            onClick={() => onSave({
              name,
              display_name: displayName,
              receipt_name: receiptName,
              product_scope: productScope,
              target_id: targetId ? Number(targetId) : null,
              adjustment_type: adjustmentType,
              adjustment_value: Number(adjustmentValue),
            })}
            disabled={!name.trim() || !displayName.trim() || !receiptName.trim() || !adjustmentValue}
            className="flex-1 px-4 py-3 bg-violet-500 text-white rounded-xl font-bold hover:bg-violet-600 disabled:opacity-50"
          >
            {t('common.action.save')}
          </button>
        </div>
      </div>
    </div>
  );
};
