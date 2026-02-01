import React, { useEffect, useState } from 'react';
import { Settings, Plus, Edit, Trash2, ChevronRight, List, Star } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { useShallow } from 'zustand/react/shallow';
import { getErrorMessage } from '@/utils/error';
import {
  useAttributes,
  useAttributesLoading,
  useAttributeActions,
  useOptionActions,
  useAttributeStore,
} from './store';
import { AttributeForm } from './AttributeForm';
import { OptionForm } from './OptionForm';
import { Permission } from '@/core/domain/types';
import type { Attribute, AttributeOption } from '@/core/domain/types/api';

// Extended option type with index for UI (matches store type)
interface AttributeOptionWithIndex extends AttributeOption {
  index: number;
  attributeId: string;
}
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { ManagementHeader, FilterBar } from '@/screens/Settings/components';
import { formatCurrency } from '@/utils/currency';

export const AttributeManagement: React.FC = React.memo(() => {
  const { t } = useI18n();

  const attributes = useAttributes();
  const isLoading = useAttributesLoading();
  const {
    fetchAll,
    deleteAttribute,
    updateAttribute,
  } = useAttributeActions();
  const { loadOptions, deleteOption } = useOptionActions();

  // Modal states
  const [attributeFormOpen, setAttributeFormOpen] = useState(false);
  const [optionFormOpen, setOptionFormOpen] = useState(false);
  const [editingAttribute, setEditingAttribute] = useState<Attribute | null>(null);
  const [editingOption, setEditingOption] = useState<AttributeOptionWithIndex | null>(null);
  const [selectedAttributeForOption, setSelectedAttributeForOption] = useState<string | null>(null);

  // Search state
  const [searchQuery, setSearchQuery] = useState('');

  // Expanded attributes (track which attributes are expanded)
  const [expandedAttributes, setExpandedAttributes] = useState<Set<string>>(new Set());

  // Confirm dialog state
  const [confirmDialog, setConfirmDialog] = useState({
    isOpen: false,
    title: '',
    description: '',
    onConfirm: () => {},
  });

  // Get all options for all attributes
  const allOptions = useAttributeStore(
    useShallow((state) => state.options)
  );

  const filteredAttributes = React.useMemo(() => {
    if (!searchQuery.trim()) return attributes;
    const q = searchQuery.toLowerCase();
    return attributes.filter(attr => attr.name.toLowerCase().includes(q));
  }, [attributes, searchQuery]);

  // Load attributes on mount
  useEffect(() => {
    fetchAll();
  }, []);

  // Load options for expanded attributes
  useEffect(() => {
    expandedAttributes.forEach((attrId) => {
      if (!allOptions.has(attrId)) {
        loadOptions(attrId);
      }
    });
  }, [expandedAttributes]);

  // Toggle attribute expansion
  const toggleAttribute = (attributeId: string) => {
    const newExpanded = new Set(expandedAttributes);
    if (newExpanded.has(attributeId)) {
      newExpanded.delete(attributeId);
    } else {
      newExpanded.add(attributeId);
      // Load options if not already loaded
      if (!allOptions.has(attributeId)) {
        loadOptions(attributeId);
      }
    }
    setExpandedAttributes(newExpanded);
  };

  // Handlers for Attributes
  const handleAddAttribute = () => {
    setEditingAttribute(null);
    setAttributeFormOpen(true);
  };

  const handleEditAttribute = (attr: Attribute, e: React.MouseEvent) => {
    e.stopPropagation();
    setEditingAttribute(attr);
    setAttributeFormOpen(true);
  };

  const handleDeleteAttribute = (attr: Attribute, e: React.MouseEvent) => {
    e.stopPropagation();
    setConfirmDialog({
      isOpen: true,
      title: t('settings.attribute.delete_attribute'),
      description:
        t('settings.attribute.confirm.delete', { name: attr.name }),
      onConfirm: async () => {
        setConfirmDialog((prev) => ({ ...prev, isOpen: false }));
        try {
          await deleteAttribute(String(attr.id));
          toast.success(t('settings.user.message.delete_success'));
        } catch (error) {
          console.error('Delete attribute error:', error);
          toast.error(getErrorMessage(error));
        }
      },
    });
  };

  // Handlers for Options
  const handleAddOption = (attributeId: string, e: React.MouseEvent) => {
    e.stopPropagation();
    setSelectedAttributeForOption(attributeId);
    setEditingOption(null);
    setOptionFormOpen(true);
  };

  const handleEditOption = (option: AttributeOptionWithIndex, e: React.MouseEvent) => {
    e.stopPropagation();
    setSelectedAttributeForOption(option.attributeId);
    setEditingOption(option);
    setOptionFormOpen(true);
  };

  const handleDeleteOption = (option: AttributeOptionWithIndex, e: React.MouseEvent) => {
    e.stopPropagation();
    setConfirmDialog({
      isOpen: true,
      title: t('settings.attribute.option.delete_option'),
      description:
        t('settings.attribute.confirm.deleteOption', { name: option.name }),
      onConfirm: async () => {
        setConfirmDialog((prev) => ({ ...prev, isOpen: false }));
        try {
          await deleteOption(option.attributeId, option.index);
          toast.success(t('settings.user.message.delete_success'));
        } catch (error) {
          console.error('Delete option error:', error);
          toast.error(getErrorMessage(error));
        }
      },
    });
  };

  const handleToggleDefault = async (attr: Attribute, optionIndex: number, e: React.MouseEvent) => {
    e.stopPropagation();
    const current = attr.default_option_indices ?? [];
    const isCurrentlyDefault = current.includes(optionIndex);

    let newDefaults: number[];
    if (attr.is_multi_select) {
      // Multi-select: toggle this index in/out of the array
      newDefaults = isCurrentlyDefault
        ? current.filter(i => i !== optionIndex)
        : [...current, optionIndex];
    } else {
      // Single-select: set or clear
      newDefaults = isCurrentlyDefault ? [] : [optionIndex];
    }

    try {
      await updateAttribute({
        id: attr.id,
        default_option_indices: newDefaults.length > 0 ? newDefaults : null,
      });
    } catch (error) {
      console.error('Toggle default error:', error);
      toast.error(getErrorMessage(error));
    }
  };

  const getAttributeTypeLabel = (type: string) => {
    const labels: Record<string, string> = {
      SINGLE_REQUIRED: t('settings.attribute.type.single_required'),
      SINGLE_OPTIONAL: t('settings.attribute.type.single_optional'),
      MULTI_REQUIRED: t('settings.attribute.type.multi_required'),
      MULTI_OPTIONAL: t('settings.attribute.type.multi_optional'),
    };
    return labels[type] || type;
  };

  return (
    <div className="space-y-5">
      <ManagementHeader
        icon={Settings}
        title={t('settings.attribute.title')}
        description={t('settings.attribute.description')}
        addButtonText={t('settings.attribute.add_attribute')}
        onAdd={handleAddAttribute}
        themeColor="teal"
        permission={Permission.ATTRIBUTES_MANAGE}
      />

      <FilterBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        searchPlaceholder={t('common.hint.search_placeholder')}
        totalCount={filteredAttributes.length}
        countUnit={t('settings.attribute.unit')}
        themeColor="teal"
      />

      <div className="bg-white rounded-xl border border-gray-200 overflow-hidden min-h-[25rem] shadow-sm">
        {isLoading && attributes.length > 0 && (
          <div className="absolute inset-0 bg-white/60 z-10 flex items-center justify-center backdrop-blur-[1px]">
            <div className="w-8 h-8 border-4 border-gray-200 border-t-teal-500 rounded-full animate-spin" />
          </div>
        )}

        {isLoading && attributes.length === 0 ? (
          <div className="text-gray-400 text-sm text-center py-16 flex flex-col items-center gap-3">
            <div className="w-8 h-8 border-4 border-gray-200 border-t-teal-500 rounded-full animate-spin" />
            <span>{t('common.message.loading')}</span>
          </div>
        ) : filteredAttributes.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-16 text-center">
            <div className="w-16 h-16 bg-gray-50 rounded-full flex items-center justify-center mb-4">
              <Settings className="text-gray-300" size={32} />
            </div>
            <p className="text-gray-500 font-medium">
              {searchQuery ? t('common.empty.no_results') : t('common.empty.no_data')}
            </p>
            {!searchQuery && (
              <p className="text-sm text-gray-400 mt-1">
                {t('settings.attribute.hint.add_first')}
              </p>
            )}
          </div>
        ) : (
          <div className="divide-y divide-gray-100">
            {filteredAttributes.map((attr) => {
              const attrId = String(attr.id);
              const isExpanded = expandedAttributes.has(attrId);
              const options = allOptions.get(attrId) || [];

              return (
                <div
                  key={attrId}
                  className="transition-all hover:bg-teal-50/30 group"
                >
                  {/* Attribute Header */}
                  <div
                    onClick={() => toggleAttribute(attrId)}
                    className={`p-4 cursor-pointer transition-colors ${isExpanded ? 'bg-teal-50/50' : ''}`}
                  >
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-3 flex-1">
                        {/* Expand Icon */}
                        <div className={`transition-transform duration-200 ${isExpanded ? 'rotate-90' : ''}`}>
                           <ChevronRight size={18} className={`shrink-0 ${isExpanded ? 'text-teal-500' : 'text-gray-400'}`} />
                        </div>

                        {/* Attribute Info */}
                        <div className="flex-1 min-w-0">
                          <div className="flex items-center gap-3 flex-wrap">
                            <h3 className={`font-medium text-sm md:text-base ${isExpanded ? 'text-teal-900' : 'text-gray-900'}`}>
                              {attr.name}
                            </h3>
                            <span className="text-xs bg-teal-100 text-teal-700 px-2 py-0.5 rounded-full font-medium">
                              {attr.is_multi_select ? t('settings.attribute.type.multi_select') : t('settings.attribute.type.single_select')}
                            </span>
                            {!attr.is_active && (
                              <span className="text-xs bg-gray-100 text-gray-500 px-2 py-0.5 rounded-full">
                                {t('common.status.inactive')}
                              </span>
                            )}
                            <span className="text-xs text-gray-400 flex items-center gap-1">
                              <span className="w-1 h-1 rounded-full bg-gray-300"></span>
                              {attr.options?.length ?? 0} {t('settings.attribute.option.title')}
                            </span>
                          </div>
                        </div>

                        {/* Action Buttons */}
                        <div className="flex items-center gap-1 shrink-0 opacity-0 group-hover:opacity-100 transition-opacity">
                          <ProtectedGate permission={Permission.ATTRIBUTES_MANAGE}>
                            <button
                              onClick={(e) => handleAddOption(attrId, e)}
                              className="p-2 text-teal-600 hover:bg-teal-100 rounded-lg transition-colors"
                              title={t('settings.attribute.option.add_option')}
                            >
                              <Plus size={16} />
                            </button>
                          </ProtectedGate>
                          <ProtectedGate permission={Permission.ATTRIBUTES_MANAGE}>
                            <button
                              onClick={(e) => handleEditAttribute(attr, e)}
                              className="p-2 text-gray-400 hover:text-teal-600 hover:bg-teal-50 rounded-lg transition-colors"
                            >
                              <Edit size={16} />
                            </button>
                          </ProtectedGate>
                          <ProtectedGate permission={Permission.ATTRIBUTES_MANAGE}>
                            <button
                              onClick={(e) => handleDeleteAttribute(attr, e)}
                              className="p-2 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors"
                            >
                              <Trash2 size={16} />
                            </button>
                          </ProtectedGate>
                        </div>
                      </div>
                    </div>
                  </div>

                  {/* Options List (Expanded) */}
                  {isExpanded && (
                    <div className="border-t border-gray-100 bg-gray-50/30 shadow-inner">
                      {options.length === 0 ? (
                        <div className="p-8 text-sm text-gray-400 text-center flex flex-col items-center justify-center border-dashed border-2 border-gray-100 m-4 rounded-xl">
                          <span className="mb-2 block text-gray-300"><List size={24} /></span>
                          {t('common.empty.no_data')}
                          <button
                            onClick={(e) => handleAddOption(attrId, e)}
                            className="mt-2 text-teal-600 hover:text-teal-700 font-medium text-xs hover:underline"
                          >
                            {t('settings.attribute.option.hint.add_first')}
                          </button>
                        </div>
                      ) : (
                        <div className="divide-y divide-gray-100/50">
                          {options.map((option) => {
                            // Check if this option is one of the defaults
                            const isDefault = attr.default_option_indices?.includes(option.index) ?? false;
                            return (
                            <div
                              key={option.index}
                              className="p-3 pl-12 hover:bg-white transition-colors group/opt relative"
                            >
                              <div className="absolute left-0 top-0 bottom-0 w-[3px] bg-transparent group-hover/opt:bg-teal-400 transition-colors"></div>
                              <div className="flex items-center justify-between">
                                <div className="flex-1 min-w-0">
                                  <div className="flex items-center gap-2 flex-wrap">
                                    <span className="font-medium text-gray-700 text-sm">{option.name}</span>
                                    {isDefault && (
                                      <span className="text-[0.625rem] uppercase tracking-wider bg-teal-100 text-teal-700 px-1.5 py-0.5 rounded border border-teal-200/50">
                                        {t('common.label.default')}
                                      </span>
                                    )}
                                    {!option.is_active && (
                                      <span className="text-[0.625rem] bg-gray-100 text-gray-500 px-1.5 py-0.5 rounded border border-gray-200">
                                        {t('common.status.inactive')}
                                      </span>
                                    )}
                                  </div>
                                  <div className="flex items-center gap-3 mt-1.5 text-xs text-gray-500">
                                  <span
                                    className={`text-xs font-bold px-2 py-0.5 rounded-full border ${
                                      option.price_modifier > 0
                                        ? 'bg-orange-50 text-orange-700 border-orange-100'
                                        : option.price_modifier < 0
                                        ? 'bg-green-50 text-green-700 border-green-100'
                                        : 'bg-gray-50 text-gray-500 border-gray-100'
                                    }`}
                                  >
                                    {option.price_modifier > 0 && '+'}
                                    {formatCurrency(option.price_modifier)}
                                  </span>
                                </div>
                                </div>
                                <div className="flex items-center gap-1 shrink-0">
                                  <ProtectedGate permission={Permission.ATTRIBUTES_MANAGE}>
                                    <button
                                      onClick={(e) => handleToggleDefault(attr, option.index, e)}
                                      className={`p-1.5 rounded-lg transition-colors ${
                                        isDefault
                                          ? 'text-amber-500 hover:text-amber-600 hover:bg-amber-50'
                                          : 'text-gray-300 hover:text-amber-500 hover:bg-amber-50 opacity-0 group-hover/opt:opacity-100'
                                      }`}
                                      title={isDefault ? t('settings.attribute.option.unset_default') : t('settings.attribute.option.set_default')}
                                    >
                                      <Star size={14} fill={isDefault ? 'currentColor' : 'none'} />
                                    </button>
                                  </ProtectedGate>
                                  <ProtectedGate permission={Permission.ATTRIBUTES_MANAGE}>
                                    <button
                                      onClick={(e) => handleEditOption(option, e)}
                                      className="p-1.5 text-gray-400 hover:text-teal-600 hover:bg-teal-50 rounded-lg transition-colors opacity-0 group-hover/opt:opacity-100"
                                    >
                                      <Edit size={14} />
                                    </button>
                                  </ProtectedGate>
                                  <ProtectedGate permission={Permission.ATTRIBUTES_MANAGE}>
                                    <button
                                      onClick={(e) => handleDeleteOption(option, e)}
                                      className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors opacity-0 group-hover/opt:opacity-100"
                                    >
                                      <Trash2 size={14} />
                                    </button>
                                  </ProtectedGate>
                                </div>
                              </div>
                            </div>
                          );
                          })}
                        </div>
                      )}
                    </div>
                  )}
                </div>
              );
            })}
          </div>
        )}
      </div>

      {/* Modals */}
      {attributeFormOpen && (
        <AttributeForm
          isOpen={attributeFormOpen}
          onClose={() => {
            setAttributeFormOpen(false);
            setEditingAttribute(null);
          }}
          editingAttribute={editingAttribute}
        />
      )}

      {optionFormOpen && selectedAttributeForOption && (
        <OptionForm
          isOpen={optionFormOpen}
          onClose={() => {
            setOptionFormOpen(false);
            setEditingOption(null);
            setSelectedAttributeForOption(null);
          }}
          attributeId={selectedAttributeForOption}
          editingOption={editingOption}
        />
      )}

      {/* Confirm Dialog */}
      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={confirmDialog.onConfirm}
        onCancel={() => setConfirmDialog((prev) => ({ ...prev, isOpen: false }))}
      />
    </div>
  );
});
