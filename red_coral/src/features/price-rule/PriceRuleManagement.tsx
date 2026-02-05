import React, { useEffect, useState } from 'react';
import { Percent, Settings } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { usePriceRuleStore } from './store';
import { createTauriClient } from '@/infrastructure/api';
import { toast } from '@/presentation/components/Toast';
import type { PriceRule } from '@/core/domain/types';
import { Permission } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';
import { ManagementHeader } from '@/screens/Settings/components';
import { FilterBar } from '@/shared/components/FilterBar';
import { PriceRuleWizard } from './PriceRuleWizard';
import {
  RuleListPanel,
  RuleDetailPanel,
  RulePreviewTester,
  RuleConflictAnalysis,
} from './components';

const getApi = () => createTauriClient();

export const PriceRuleManagement: React.FC = React.memo(() => {
  const { t } = useI18n();
  const { hasPermission } = usePermission();
  const canManage = hasPermission(Permission.SETTINGS_MANAGE);

  const priceRuleStore = usePriceRuleStore();
  const rules = priceRuleStore.items;
  const loading = priceRuleStore.isLoading;

  const [searchQuery, setSearchQuery] = useState('');
  const [selectedRuleId, setSelectedRuleId] = useState<string | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<PriceRule | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<PriceRule | null>(null);

  useEffect(() => {
    priceRuleStore.fetchAll();
  }, []);

  // Auto-select first rule if none selected
  useEffect(() => {
    if (!selectedRuleId && rules.length > 0) {
      setSelectedRuleId(rules[0].id);
    }
  }, [rules, selectedRuleId]);

  const selectedRule = rules.find(r => r.id === selectedRuleId) || null;

  const handleAdd = () => {
    setEditingRule(null);
    setWizardOpen(true);
  };

  const handleEdit = (rule: PriceRule) => {
    setEditingRule(rule);
    setWizardOpen(true);
  };

  const handleDelete = async () => {
    if (!deleteConfirm?.id) return;
    try {
      await getApi().deletePriceRule(deleteConfirm.id);
      await priceRuleStore.fetchAll(true);
      toast.success(t('settings.price_rule.message.deleted'));
      // Clear selection if deleted rule was selected
      if (selectedRuleId === deleteConfirm.id) {
        setSelectedRuleId(null);
      }
    } catch (e) {
      console.error(e);
      toast.error(t('common.message.delete_failed'));
    } finally {
      setDeleteConfirm(null);
    }
  };

  const handleWizardClose = () => {
    setWizardOpen(false);
    setEditingRule(null);
  };

  const handleWizardSuccess = async () => {
    await priceRuleStore.fetchAll(true);
    handleWizardClose();
  };

  const handleRuleUpdated = async () => {
    await priceRuleStore.fetchAll(true);
  };

  // Empty state
  if (!loading && rules.length === 0) {
    return (
      <div className="space-y-5">
        <ManagementHeader
          icon={Percent}
          title={t('settings.price_rule.title')}
          description={t('settings.price_rule.description')}
          addButtonText={t('settings.price_rule.add_rule')}
          onAdd={handleAdd}
          themeColor="teal"
          permission={Permission.SETTINGS_MANAGE}
        />

        <div className="flex flex-col items-center justify-center py-20 text-gray-400">
          <Settings size={64} className="mb-4 opacity-50" />
          <p className="text-lg mb-4">{t('settings.price_rule.empty')}</p>
          {canManage && (
            <button
              onClick={handleAdd}
              className="px-6 py-3 bg-teal-600 text-white rounded-xl font-medium hover:bg-teal-700 transition-colors"
            >
              {t('settings.price_rule.add_rule')}
            </button>
          )}
        </div>

        {wizardOpen && (
          <PriceRuleWizard
            isOpen={wizardOpen}
            onClose={handleWizardClose}
            onSuccess={handleWizardSuccess}
            editingRule={editingRule}
          />
        )}
      </div>
    );
  }

  return (
    <div className="space-y-5 h-full flex flex-col">
      <ManagementHeader
        icon={Percent}
        title={t('settings.price_rule.title')}
        description={t('settings.price_rule.description')}
        addButtonText={t('settings.price_rule.add_rule')}
        onAdd={handleAdd}
        themeColor="teal"
        permission={Permission.SETTINGS_MANAGE}
      />

      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder={t('settings.price_rule.search_placeholder')}
        totalCount={rules.length}
        countUnit={t('settings.price_rule.unit')}
        themeColor="teal"
      />

      {/* Master-Detail Layout */}
      <div className="flex-1 flex bg-white rounded-2xl shadow-sm border border-gray-200 overflow-hidden min-h-0">
        {/* Left Panel - Rule List */}
        <RuleListPanel
          rules={rules}
          selectedRuleId={selectedRuleId}
          onSelectRule={setSelectedRuleId}
          searchQuery={searchQuery}
        />

        {/* Right Panel - Rule Details */}
        <div className="flex-1 flex flex-col overflow-hidden">
          {selectedRule ? (
            <div className="flex-1 overflow-y-auto">
              <div className="max-w-2xl mx-auto p-6 space-y-6">
                {/* Rule Detail Panel */}
                <RuleDetailPanel
                  rule={selectedRule}
                  onRuleUpdated={handleRuleUpdated}
                  onDeleteRule={setDeleteConfirm}
                />

                {/* Rule Preview Tester */}
                <RulePreviewTester
                  rules={rules}
                  currentRuleId={selectedRuleId}
                />

                {/* Rule Conflict Analysis */}
                <RuleConflictAnalysis
                  currentRule={selectedRule}
                  allRules={rules}
                />
              </div>
            </div>
          ) : (
            <div className="flex-1 flex items-center justify-center bg-white">
              <div className="text-center text-gray-400">
                <Settings size={48} className="mx-auto mb-3 opacity-50" />
                <p>{t('settings.price_rule.hint.select_rule')}</p>
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Wizard Modal */}
      {wizardOpen && (
        <PriceRuleWizard
          isOpen={wizardOpen}
          onClose={handleWizardClose}
          onSuccess={handleWizardSuccess}
          editingRule={editingRule}
        />
      )}

      {/* Delete Confirmation */}
      {deleteConfirm && (
        <div className="fixed inset-0 z-90 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl max-w-sm w-full overflow-hidden animate-in zoom-in-95">
            <div className="p-6">
              <h3 className="text-lg font-bold text-gray-900 mb-2">
                {t('settings.price_rule.delete_confirm_title')}
              </h3>
              <p className="text-sm text-gray-600 mb-6">
                {t('settings.price_rule.delete_confirm_message', {
                  name: deleteConfirm.display_name,
                })}
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
