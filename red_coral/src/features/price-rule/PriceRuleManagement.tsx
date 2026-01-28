import React, { useEffect, useMemo, useState } from 'react';
import { Percent, Tag, Package, LayoutGrid, Clock, Check, X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { usePriceRuleStore } from './store';
import { createTauriClient } from '@/infrastructure/api';
import { DataTable, Column } from '@/shared/components/DataTable';
import { FilterBar } from '@/shared/components/FilterBar';
import { toast } from '@/presentation/components/Toast';
import type { PriceRule } from '@/core/domain/types';
import { Permission } from '@/core/domain/types';
import { usePermission } from '@/hooks/usePermission';
import { ManagementHeader } from '@/screens/Settings/components';
import { PriceRuleWizard } from './PriceRuleWizard';

const api = createTauriClient();

export const PriceRuleManagement: React.FC = React.memo(() => {
  const { t } = useI18n();
  const { hasPermission } = usePermission();
  const canManage = hasPermission(Permission.SETTINGS_MANAGE);

  const priceRuleStore = usePriceRuleStore();
  const rules = priceRuleStore.items;
  const loading = priceRuleStore.isLoading;

  const [searchQuery, setSearchQuery] = useState('');
  const [wizardOpen, setWizardOpen] = useState(false);
  const [editingRule, setEditingRule] = useState<PriceRule | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<PriceRule | null>(null);

  useEffect(() => {
    priceRuleStore.fetchAll();
  }, []);

  const filteredItems = useMemo(() => {
    if (!searchQuery.trim()) return rules;
    const q = searchQuery.toLowerCase();
    return rules.filter(
      (rule) =>
        rule.name.toLowerCase().includes(q) ||
        rule.display_name.toLowerCase().includes(q)
    );
  }, [rules, searchQuery]);

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
      await api.deletePriceRule(deleteConfirm.id);
      await priceRuleStore.fetchAll(true);
      toast.success(t('settings.price_rule.message.deleted'));
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

  // Render scope badge
  const renderScopeBadge = (rule: PriceRule) => {
    const scopeConfig: Record<string, { icon: React.ElementType; color: string; label: string }> = {
      GLOBAL: { icon: LayoutGrid, color: 'bg-purple-100 text-purple-700', label: t('settings.price_rule.scope.global') },
      CATEGORY: { icon: LayoutGrid, color: 'bg-blue-100 text-blue-700', label: t('settings.price_rule.scope.category') },
      TAG: { icon: Tag, color: 'bg-indigo-100 text-indigo-700', label: t('settings.price_rule.scope.tag') },
      PRODUCT: { icon: Package, color: 'bg-orange-100 text-orange-700', label: t('settings.price_rule.scope.product') },
    };
    const config = scopeConfig[rule.product_scope] || scopeConfig.GLOBAL;
    const Icon = config.icon;
    return (
      <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${config.color}`}>
        <Icon size={12} />
        {config.label}
      </span>
    );
  };

  // Render time mode badge
  const renderTimeBadge = (rule: PriceRule) => {
    const timeConfig: Record<string, { color: string; label: string }> = {
      ALWAYS: { color: 'bg-green-100 text-green-700', label: t('settings.price_rule.time.always') },
      SCHEDULE: { color: 'bg-amber-100 text-amber-700', label: t('settings.price_rule.time.schedule') },
      ONETIME: { color: 'bg-rose-100 text-rose-700', label: t('settings.price_rule.time.onetime') },
    };
    // Determine time mode from fields
    let mode = 'ALWAYS';
    if (rule.valid_from || rule.valid_until) {
      mode = 'ONETIME';
    } else if (rule.active_days?.length || rule.active_start_time || rule.active_end_time) {
      mode = 'SCHEDULE';
    }
    const config = timeConfig[mode];
    return (
      <span className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${config.color}`}>
        <Clock size={12} />
        {config.label}
      </span>
    );
  };

  const columns: Column<PriceRule>[] = useMemo(
    () => [
      {
        key: 'name',
        header: t('settings.price_rule.column.name'),
        render: (item) => (
          <div className="flex flex-col gap-0.5 min-w-0">
            <span className="font-medium text-gray-900 truncate">{item.display_name}</span>
            <span className="text-xs text-gray-400 truncate">{item.name}</span>
          </div>
        ),
      },
      {
        key: 'type',
        header: t('settings.price_rule.column.type'),
        width: '120px',
        render: (item) => (
          <span
            className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-xs font-medium ${
              item.rule_type === 'DISCOUNT'
                ? 'bg-green-100 text-green-700'
                : 'bg-red-100 text-red-700'
            }`}
          >
            <Percent size={12} />
            {item.rule_type === 'DISCOUNT'
              ? t('settings.price_rule.type.discount')
              : t('settings.price_rule.type.surcharge')}
          </span>
        ),
      },
      {
        key: 'value',
        header: t('settings.price_rule.column.value'),
        width: '120px',
        align: 'right',
        render: (item) => (
          <span className="font-mono text-sm">
            {item.adjustment_type === 'PERCENTAGE'
              ? `${item.adjustment_value}%`
              : `€${item.adjustment_value.toFixed(2)}`}
          </span>
        ),
      },
      {
        key: 'scope',
        header: t('settings.price_rule.column.scope'),
        width: '130px',
        render: renderScopeBadge,
      },
      {
        key: 'time',
        header: t('settings.price_rule.column.time'),
        width: '120px',
        render: renderTimeBadge,
      },
      {
        key: 'status',
        header: t('settings.price_rule.column.status'),
        width: '100px',
        align: 'center',
        render: (item) =>
          item.is_active ? (
            <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-green-100">
              <Check size={14} className="text-green-600" />
            </span>
          ) : (
            <span className="inline-flex items-center justify-center w-6 h-6 rounded-full bg-gray-100">
              <X size={14} className="text-gray-400" />
            </span>
          ),
      },
    ],
    [t]
  );

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

      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder={t('settings.price_rule.search_placeholder')}
        totalCount={filteredItems.length}
        countUnit={t('settings.price_rule.unit')}
        themeColor="teal"
      />

      <DataTable
        data={filteredItems}
        columns={columns}
        loading={loading}
        getRowKey={(item) => item.id || item.name}
        onEdit={canManage ? handleEdit : undefined}
        onDelete={canManage ? (item) => setDeleteConfirm(item) : undefined}
        emptyText={t('common.empty.no_data')}
        themeColor="teal"
      />

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
                {t('settings.price_rule.delete_confirm_message', { name: deleteConfirm.display_name })}
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
