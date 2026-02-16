import React, { useEffect, useState, useMemo } from 'react';
import { Settings, Plus, Edit, Trash2, List, Star, ReceiptText, ChefHat, Hash, DollarSign, Search } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { ConfirmDialog } from '@/shared/components/ConfirmDialog';
import { useConfirmDialog } from '@/shared/hooks/useConfirmDialog';
import { useModalState } from '@/shared/hooks/useModalState';
import { useShallow } from 'zustand/react/shallow';
import { getErrorMessage } from '@/utils/error';
import { logger } from '@/utils/logger';
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
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { ManagementHeader } from '@/screens/Settings/components';
import { formatCurrency } from '@/utils/currency';

// Extended option type with index for UI (matches store type)
interface AttributeOptionWithIndex extends AttributeOption {
  index: number;
  attributeId: number;
}

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
  const attributeForm = useModalState<Attribute>();
  const optionForm = useModalState<AttributeOptionWithIndex>();

  // Search state
  const [searchQuery, setSearchQuery] = useState('');

  // Selected attribute (Master-Detail)
  const [selectedAttributeId, setSelectedAttributeId] = useState<number | null>(null);

  // Confirm dialog state
  const confirmDialog = useConfirmDialog();

  // Get all options for all attributes
  const allOptions = useAttributeStore(
    useShallow((state) => state.options)
  );

  const filteredAttributes = useMemo(() => {
    let list = [...attributes].sort((a, b) => a.display_order - b.display_order);
    if (searchQuery.trim()) {
      const q = searchQuery.toLowerCase();
      list = list.filter(attr => attr.name.toLowerCase().includes(q));
    }
    return list;
  }, [attributes, searchQuery]);

  // Get selected attribute
  const selectedAttribute = useMemo(() => {
    if (!selectedAttributeId) return null;
    return attributes.find(a => a.id === selectedAttributeId) ?? null;
  }, [attributes, selectedAttributeId]);

  // Get options for selected attribute
  const selectedOptions = useMemo(() => {
    if (!selectedAttributeId) return [];
    return [...(allOptions.get(selectedAttributeId) || [])].sort((a, b) => a.display_order - b.display_order);
  }, [allOptions, selectedAttributeId]);

  // Load attributes on mount
  useEffect(() => {
    fetchAll();
  }, []);

  // Load options when attribute is selected
  useEffect(() => {
    if (selectedAttributeId && !allOptions.has(selectedAttributeId)) {
      loadOptions(selectedAttributeId);
    }
  }, [selectedAttributeId]);

  // Auto-select first attribute if none selected
  useEffect(() => {
    if (!selectedAttributeId && filteredAttributes.length > 0) {
      setSelectedAttributeId(filteredAttributes[0].id);
    }
  }, [filteredAttributes, selectedAttributeId]);

  // Clear selection if selected attribute is deleted
  useEffect(() => {
    if (selectedAttributeId && !attributes.find(a => a.id === selectedAttributeId)) {
      setSelectedAttributeId(filteredAttributes.length > 0 ? filteredAttributes[0].id : null);
    }
  }, [attributes]);

  // Handlers for Attributes
  const handleAddAttribute = () => {
    attributeForm.open();
  };

  const handleEditAttribute = (attr: Attribute) => {
    attributeForm.open(attr);
  };

  const handleDeleteAttribute = (attr: Attribute) => {
    confirmDialog.show(
      t('settings.attribute.delete_attribute'),
      t('settings.attribute.confirm.delete', { name: attr.name }),
      async () => {
        confirmDialog.close();
        try {
          await deleteAttribute(attr.id);
          toast.success(t('settings.user.message.delete_success'));
        } catch (error) {
          logger.error('Failed to delete attribute', error);
          toast.error(getErrorMessage(error));
        }
      },
    );
  };

  // Handlers for Options
  const handleAddOption = () => {
    if (!selectedAttributeId) return;
    optionForm.open();
  };

  const handleEditOption = (option: AttributeOptionWithIndex) => {
    optionForm.open(option);
  };

  const handleDeleteOption = (option: AttributeOptionWithIndex) => {
    confirmDialog.show(
      t('settings.attribute.option.delete_option'),
      t('settings.attribute.confirm.deleteOption', { name: option.name }),
      async () => {
        confirmDialog.close();
        try {
          await deleteOption(option.attributeId, option.index);
          toast.success(t('settings.user.message.delete_success'));
        } catch (error) {
          logger.error('Failed to delete option', error);
          toast.error(getErrorMessage(error));
        }
      },
    );
  };

  const handleToggleDefault = async (attr: Attribute, optionId: number) => {
    const current = attr.default_option_ids ?? [];
    const isCurrentlyDefault = current.includes(optionId);

    let newDefaults: number[];
    if (attr.is_multi_select) {
      if (isCurrentlyDefault) {
        newDefaults = current.filter(id => id !== optionId);
      } else {
        if (attr.max_selections && current.length >= attr.max_selections) {
          toast.error(t('settings.attribute.error.max_defaults', { n: attr.max_selections }));
          return;
        }
        newDefaults = [...current, optionId];
      }
    } else {
      newDefaults = isCurrentlyDefault ? [] : [optionId];
    }

    try {
      await updateAttribute({
        id: attr.id,
        default_option_ids: newDefaults,
      });
    } catch (error) {
      logger.error('Failed to toggle default option', error);
      toast.error(getErrorMessage(error));
    }
  };

  // Check if attribute has special features (use attr.options directly, not lazy-loaded store)
  const getAttributeFeatures = (attr: Attribute) => {
    const options = attr.options ?? [];
    const hasPrice = options.some(o => o.price_modifier !== 0);
    const hasQuantity = options.some(o => o.enable_quantity);
    return { hasPrice, hasQuantity };
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
        permission={Permission.MENU_MANAGE}
      />

      <div className="bg-white rounded-xl border border-gray-200 overflow-hidden shadow-sm flex" style={{ minHeight: '28rem' }}>
        {isLoading && attributes.length === 0 ? (
          <div className="flex-1 flex items-center justify-center">
            <div className="text-gray-400 text-sm text-center flex flex-col items-center gap-3">
              <div className="w-8 h-8 border-4 border-gray-200 border-t-teal-500 rounded-full animate-spin" />
              <span>{t('common.message.loading')}</span>
            </div>
          </div>
        ) : attributes.length === 0 ? (
          <div className="flex-1 flex flex-col items-center justify-center py-16 text-center">
            <div className="w-16 h-16 bg-gray-50 rounded-full flex items-center justify-center mb-4">
              <Settings className="text-gray-300" size={32} />
            </div>
            <p className="text-gray-500 font-medium">{t('common.empty.no_data')}</p>
            <p className="text-sm text-gray-400 mt-1">{t('settings.attribute.hint.add_first')}</p>
          </div>
        ) : (
          <>
            {/* Left Panel - Attribute List */}
            <div className="w-72 border-r border-gray-100 bg-gray-50/50 flex flex-col shrink-0">
              {/* Search */}
              <div className="p-3 border-b border-gray-100">
                <div className="relative">
                  <Search size={16} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
                  <input
                    type="text"
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    placeholder={t('common.hint.search_placeholder')}
                    className="w-full pl-9 pr-3 py-2 text-sm border border-gray-200 rounded-lg bg-white focus:outline-none focus:ring-2 focus:ring-teal-500/20 focus:border-teal-400"
                  />
                </div>
              </div>

              {/* Attribute List */}
              <div className="flex-1 overflow-y-auto">
                {filteredAttributes.length === 0 ? (
                  <div className="p-4 text-center text-sm text-gray-400">
                    {t('common.empty.no_results')}
                  </div>
                ) : (
                  <div className="py-1">
                    {filteredAttributes.map((attr) => {
                      const isSelected = selectedAttributeId === attr.id;
                      const { hasPrice, hasQuantity } = getAttributeFeatures(attr);
                      const optionCount = attr.options?.length ?? 0;

                      return (
                        <div
                          key={attr.id}
                          onClick={() => setSelectedAttributeId(attr.id)}
                          className={`
                            mx-2 my-1 px-3 py-2.5 rounded-lg cursor-pointer transition-all
                            ${isSelected
                              ? 'bg-teal-50 border-l-3 border-l-teal-500 shadow-sm'
                              : 'hover:bg-white border-l-3 border-l-transparent'
                            }
                          `}
                        >
                          <div className="flex items-center justify-between gap-2">
                            <h3 className={`font-medium text-sm truncate ${isSelected ? 'text-teal-900' : 'text-gray-800'}`}>
                              {attr.name}
                            </h3>
                            <div className="flex items-center gap-1 shrink-0">
                              {hasQuantity && (
                                <Hash size={12} className="text-purple-500" />
                              )}
                              {hasPrice && (
                                <DollarSign size={12} className="text-orange-500" />
                              )}
                              {attr.show_on_kitchen_print && (
                                <ChefHat size={12} className="text-purple-400" />
                              )}
                              {attr.show_on_receipt && (
                                <ReceiptText size={12} className="text-blue-400" />
                              )}
                            </div>
                          </div>
                          <div className="flex items-center gap-2 mt-1 text-xs text-gray-500">
                            <span className={`px-1.5 py-0.5 rounded ${isSelected ? 'bg-teal-100 text-teal-700' : 'bg-gray-100 text-gray-600'}`}>
                              {attr.is_multi_select ? t('settings.attribute.type.multi_select') : t('settings.attribute.type.single_select')}
                            </span>
                            <span>· {optionCount} {t('settings.attribute.option.title')}</span>
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>

            {/* Right Panel - Option Detail */}
            <div className="flex-1 flex flex-col min-w-0">
              {!selectedAttribute ? (
                <div className="flex-1 flex flex-col items-center justify-center text-gray-400">
                  <List size={32} className="mb-2 text-gray-300" />
                  <p className="text-sm">{t('settings.attribute.hint.select_attribute')}</p>
                </div>
              ) : (
                <>
                  {/* Attribute Header */}
                  <div className="p-4 border-b border-gray-100">
                    <div className="flex items-center justify-between">
                      <div>
                        <h2 className="text-lg font-semibold text-gray-900">{selectedAttribute.name}</h2>
                        <div className="flex items-center gap-3 mt-1 text-sm text-gray-500">
                          <span className="px-2 py-0.5 bg-teal-50 text-teal-700 rounded font-medium text-xs">
                            {selectedAttribute.is_multi_select ? t('settings.attribute.type.multi_select') : t('settings.attribute.type.single_select')}
                          </span>
                          {selectedAttribute.show_on_receipt && (
                            <span className="flex items-center gap-1 text-blue-600">
                              <ReceiptText size={12} />
                              {t('settings.attribute.show_on_receipt')}
                              {selectedAttribute.receipt_name && (
                                <span className="text-gray-400">({selectedAttribute.receipt_name})</span>
                              )}
                            </span>
                          )}
                          {selectedAttribute.show_on_kitchen_print && (
                            <span className="flex items-center gap-1 text-purple-600">
                              <ChefHat size={12} />
                              {t('settings.attribute.show_on_kitchen_print')}
                              {selectedAttribute.kitchen_print_name && (
                                <span className="text-gray-400">({selectedAttribute.kitchen_print_name})</span>
                              )}
                            </span>
                          )}
                        </div>
                      </div>
                      <ProtectedGate permission={Permission.MENU_MANAGE}>
                        <div className="flex items-center gap-2">
                          <button
                            onClick={() => handleEditAttribute(selectedAttribute)}
                            className="px-3 py-1.5 text-sm text-gray-600 hover:text-teal-600 hover:bg-teal-50 rounded-lg transition-colors flex items-center gap-1"
                          >
                            <Edit size={14} />
                            {t('common.action.edit')}
                          </button>
                          <button
                            onClick={() => handleDeleteAttribute(selectedAttribute)}
                            className="px-3 py-1.5 text-sm text-gray-600 hover:text-red-600 hover:bg-red-50 rounded-lg transition-colors flex items-center gap-1"
                          >
                            <Trash2 size={14} />
                            {t('common.action.delete')}
                          </button>
                        </div>
                      </ProtectedGate>
                    </div>
                  </div>

                  {/* Options List Header */}
                  <div className="px-4 py-3 border-b border-gray-50 flex items-center justify-between bg-gray-50/30">
                    <h3 className="font-medium text-gray-700 text-sm">{t('settings.attribute.option.title')}</h3>
                    <ProtectedGate permission={Permission.MENU_MANAGE}>
                      <button
                        onClick={handleAddOption}
                        className="px-3 py-1.5 text-sm bg-teal-500 text-white rounded-lg hover:bg-teal-600 transition-colors flex items-center gap-1"
                      >
                        <Plus size={14} />
                        {t('settings.attribute.option.add_option')}
                      </button>
                    </ProtectedGate>
                  </div>

                  {/* Options List */}
                  <div className="flex-1 overflow-y-auto p-4">
                    {selectedOptions.length === 0 ? (
                      <div className="h-full flex flex-col items-center justify-center border-2 border-dashed border-gray-200 rounded-xl text-gray-400">
                        <List size={24} className="mb-2 text-gray-300" />
                        <p className="text-sm">{t('common.empty.no_data')}</p>
                        <ProtectedGate permission={Permission.MENU_MANAGE}>
                          <button
                            onClick={handleAddOption}
                            className="mt-2 text-teal-600 hover:text-teal-700 text-sm font-medium hover:underline"
                          >
                            {t('settings.attribute.option.hint.add_first')}
                          </button>
                        </ProtectedGate>
                      </div>
                    ) : (
                      <div className="space-y-2">
                        {selectedOptions.map((option) => {
                          const isDefault = selectedAttribute.default_option_ids?.includes(option.id) ?? false;
                          const hasPriceMod = option.price_modifier !== 0;
                          const hasQuantityControl = option.enable_quantity;

                          return (
                            <div
                              key={option.index}
                              className={`
                                p-3 rounded-lg border transition-colors group
                                ${isDefault ? 'bg-amber-50/50 border-amber-200' : 'bg-white border-gray-200 hover:border-gray-300'}
                              `}
                            >
                              {/* Row 1: Name + Price + Actions */}
                              <div className="flex items-center gap-2">
                                <ProtectedGate permission={Permission.MENU_MANAGE}>
                                  <button
                                    onClick={() => handleToggleDefault(selectedAttribute, option.id)}
                                    className={`shrink-0 p-1 rounded transition-colors ${
                                      isDefault ? 'text-amber-500 hover:text-amber-600' : 'text-gray-300 hover:text-amber-400'
                                    }`}
                                    title={isDefault ? t('settings.attribute.option.unset_default') : t('settings.attribute.option.set_default')}
                                  >
                                    <Star size={16} fill={isDefault ? 'currentColor' : 'none'} />
                                  </button>
                                </ProtectedGate>

                                <span className={`font-medium ${isDefault ? 'text-gray-900' : 'text-gray-800'}`}>
                                  {option.name}
                                </span>

                                <div className="flex-1" />

                                {hasPriceMod && (
                                  <span className={`text-sm font-semibold px-2 py-0.5 rounded ${
                                    option.price_modifier > 0
                                      ? 'bg-orange-50 text-orange-600 border border-orange-100'
                                      : 'bg-green-50 text-green-600 border border-green-100'
                                  }`}>
                                    {option.price_modifier > 0 ? '+' : ''}{formatCurrency(option.price_modifier)}
                                  </span>
                                )}

                                <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                                  <ProtectedGate permission={Permission.MENU_MANAGE}>
                                    <button
                                      onClick={() => handleEditOption(option)}
                                      className="p-1.5 text-gray-400 hover:text-teal-600 hover:bg-teal-50 rounded-md transition-colors"
                                    >
                                      <Edit size={14} />
                                    </button>
                                  </ProtectedGate>
                                  <ProtectedGate permission={Permission.MENU_MANAGE}>
                                    <button
                                      onClick={() => handleDeleteOption(option)}
                                      className="p-1.5 text-gray-400 hover:text-red-600 hover:bg-red-50 rounded-md transition-colors"
                                    >
                                      <Trash2 size={14} />
                                    </button>
                                  </ProtectedGate>
                                </div>
                              </div>

                              {/* Row 2: Quantity Control (if enabled) */}
                              {hasQuantityControl && (
                                <div className="mt-2 flex items-center gap-2 text-xs text-purple-600">
                                  <Hash size={12} />
                                  <span>{t('settings.attribute.option.quantity_control')}: 1~{option.max_quantity ?? 99}</span>
                                </div>
                              )}

                              {/* Row 3: Receipt/Kitchen names (if any) */}
                              {(option.receipt_name || option.kitchen_print_name) && (
                                <div className="mt-2 flex items-center gap-4 text-xs text-gray-500">
                                  {option.receipt_name && (
                                    <span className="flex items-center gap-1 text-blue-500">
                                      <ReceiptText size={11} />
                                      {option.receipt_name}
                                    </span>
                                  )}
                                  {option.kitchen_print_name && (
                                    <span className="flex items-center gap-1 text-purple-500">
                                      <ChefHat size={11} />
                                      {option.kitchen_print_name}
                                    </span>
                                  )}
                                </div>
                              )}
                            </div>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </>
              )}
            </div>
          </>
        )}
      </div>

      {/* Modals */}
      {attributeForm.isOpen && (
        <AttributeForm
          isOpen={attributeForm.isOpen}
          onClose={attributeForm.close}
          editingAttribute={attributeForm.editing}
        />
      )}

      {optionForm.isOpen && selectedAttributeId && (
        <OptionForm
          isOpen={optionForm.isOpen}
          onClose={optionForm.close}
          attributeId={selectedAttributeId}
          editingOption={optionForm.editing}
        />
      )}

      {/* Confirm Dialog */}
      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={confirmDialog.onConfirm}
        onCancel={confirmDialog.close}
      />
    </div>
  );
});
