import React, { useState } from 'react';
import { X, RotateCcw } from 'lucide-react';
import type { AuditAction } from '@/core/domain/types/api';

/**
 * 资源类型 → 该资源下所有 action
 */
const RESOURCE_ACTIONS: Record<string, AuditAction[]> = {
  system: ['system_startup', 'system_shutdown', 'system_abnormal_shutdown', 'system_long_downtime'],
  auth: ['login_success', 'login_failed', 'logout'],
  system_issue: ['resolve_system_issue'],
  order: ['order_completed', 'order_voided', 'order_merged', 'order_moved'],
  employee: ['employee_created', 'employee_updated', 'employee_deleted'],
  role: ['role_created', 'role_updated', 'role_deleted'],
  product: ['product_created', 'product_updated', 'product_deleted'],
  category: ['category_created', 'category_updated', 'category_deleted'],
  tag: ['tag_created', 'tag_updated', 'tag_deleted'],
  attribute: ['attribute_created', 'attribute_updated', 'attribute_deleted'],
  price_rule: ['price_rule_created', 'price_rule_updated', 'price_rule_deleted'],
  zone: ['zone_created', 'zone_updated', 'zone_deleted'],
  dining_table: ['table_created', 'table_updated', 'table_deleted'],
  shift: ['shift_opened', 'shift_closed'],
  print_config: ['print_config_changed'],
  print_destination: ['print_destination_created', 'print_destination_updated', 'print_destination_deleted'],
  label_template: ['label_template_created', 'label_template_updated', 'label_template_deleted'],
  store_info: ['store_info_changed'],
};

/**
 * 资源分类 — 将资源类型分组显示，减少视觉噪音
 */
const RESOURCE_CATEGORIES: { group: string; resources: string[] }[] = [
  { group: 'system', resources: ['system', 'auth', 'system_issue'] },
  { group: 'order', resources: ['order'] },
  { group: 'management', resources: ['employee', 'role'] },
  { group: 'catalog', resources: ['product', 'category', 'tag', 'attribute'] },
  { group: 'venue', resources: ['zone', 'dining_table'] },
  { group: 'config', resources: ['price_rule', 'shift', 'print_config', 'print_destination', 'label_template', 'store_info'] },
];

interface AuditFilterModalProps {
  isOpen: boolean;
  onClose: () => void;
  actionFilter: string;
  resourceTypeFilter: string;
  onApply: (action: string, resourceType: string) => void;
  t: (key: string) => string;
}

export const AuditFilterModal: React.FC<AuditFilterModalProps> = ({
  isOpen,
  onClose,
  actionFilter,
  resourceTypeFilter,
  onApply,
  t,
}) => {
  const [localResource, setLocalResource] = useState(resourceTypeFilter);
  const [localAction, setLocalAction] = useState(actionFilter);

  // 同步外部值
  React.useEffect(() => {
    if (isOpen) {
      setLocalResource(resourceTypeFilter);
      setLocalAction(actionFilter);
    }
  }, [isOpen, actionFilter, resourceTypeFilter]);

  if (!isOpen) return null;

  const handleReset = () => {
    setLocalResource('');
    setLocalAction('');
  };

  const handleApply = () => {
    // action 和 resource 互斥: 选了具体 action 就不需要 resource_type
    if (localAction) {
      onApply(localAction, '');
    } else {
      onApply('', localResource);
    }
    onClose();
  };

  const handleResourceClick = (rt: string) => {
    if (localResource === rt) {
      // 取消选择
      setLocalResource('');
      setLocalAction('');
    } else {
      setLocalResource(rt);
      setLocalAction(''); // 切换资源时清除已选操作
    }
  };

  const handleActionClick = (action: string) => {
    setLocalAction(localAction === action ? '' : action);
  };

  const hasFilters = localAction || localResource;
  const actions = localResource ? (RESOURCE_ACTIONS[localResource] || []) : [];

  return (
    <div
      className="fixed inset-0 z-100 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200"
      onClick={onClose}
    >
      <div
        className="bg-white rounded-2xl shadow-2xl w-full max-w-lg flex flex-col max-h-[85vh] overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-6 py-4 border-b border-gray-100 bg-gray-50/50 shrink-0">
          <h3 className="text-lg font-bold text-gray-900">
            {t('audit.filter.title')}
          </h3>
          <button
            onClick={onClose}
            className="p-2 hover:bg-gray-200 rounded-full transition-colors"
          >
            <X size={20} className="text-gray-500" />
          </button>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto min-h-0 flex-1 space-y-5">
          {/* 资源类型 — 按分类显示 */}
          <div>
            <h4 className="text-sm font-semibold text-gray-700 mb-3">
              {t('audit.filter.resource_type_label')}
            </h4>
            <div className="space-y-3">
              {RESOURCE_CATEGORIES.map(({ group, resources }) => (
                <div key={group}>
                  <div className="text-[11px] text-gray-400 font-medium mb-1.5 uppercase tracking-wider">
                    {t(`audit.group.${group}`)}
                  </div>
                  <div className="flex flex-wrap gap-1.5">
                    {resources.map((rt) => {
                      const isSelected = localResource === rt;
                      return (
                        <button
                          key={rt}
                          onClick={() => handleResourceClick(rt)}
                          className={`px-2.5 py-1 text-xs rounded-lg border transition-colors ${
                            isSelected
                              ? 'bg-indigo-600 border-indigo-600 text-white font-medium'
                              : 'bg-white border-gray-200 text-gray-600 hover:bg-gray-50'
                          }`}
                        >
                          {t(`audit.resource_type.${rt}`)}
                        </button>
                      );
                    })}
                  </div>
                </div>
              ))}
            </div>
          </div>

          {/* 具体操作 — 仅在选中资源后显示 */}
          {localResource && actions.length > 1 && (
            <div>
              <h4 className="text-sm font-semibold text-gray-700 mb-2">
                {t('audit.filter.action_optional')}
              </h4>
              <div className="flex flex-wrap gap-1.5 pl-2 border-l-2 border-indigo-200 ml-1">
                {actions.map((action) => {
                  const isSelected = localAction === action;
                  return (
                    <button
                      key={action}
                      onClick={() => handleActionClick(action)}
                      className={`px-2.5 py-1 text-xs rounded-lg border transition-colors ${
                        isSelected
                          ? 'bg-indigo-600 border-indigo-600 text-white font-medium'
                          : 'bg-white border-gray-200 text-gray-600 hover:bg-gray-50'
                      }`}
                    >
                      {t(`audit.action.${action}`)}
                    </button>
                  );
                })}
              </div>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-6 py-4 border-t border-gray-100 bg-gray-50/50 flex items-center justify-between shrink-0">
          <button
            onClick={handleReset}
            disabled={!hasFilters}
            className="flex items-center gap-1.5 px-3 py-2 text-xs text-gray-500 hover:text-gray-700 disabled:opacity-30 disabled:cursor-not-allowed transition-colors"
          >
            <RotateCcw size={14} />
            {t('audit.filter.reset')}
          </button>
          <button
            onClick={handleApply}
            className="px-6 py-2 bg-indigo-600 text-white rounded-xl text-sm font-bold hover:bg-indigo-700 transition-colors shadow-lg shadow-indigo-600/20"
          >
            {t('common.action.confirm')}
          </button>
        </div>
      </div>
    </div>
  );
};
