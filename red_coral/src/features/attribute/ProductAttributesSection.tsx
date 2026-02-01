import React, { useEffect, useState } from 'react';
import { Sliders, Check, Layers, Plus, X, Search, AlertCircle, Lock } from 'lucide-react';
import { useAttributeStore, useAttributes, useAttributeActions, useOptionActions, attributeHelpers } from './store';
import { formatCurrency } from '@/utils/currency';

interface ProductAttributesSectionProps {
  selectedAttributeIds: string[];
  attributeDefaultOptions?: Record<string, string | string[]>;
  onChange: (attributeIds: string[]) => void;
  onDefaultOptionChange?: (attributeId: string, optionIds: string[]) => void;
  t: (key: string) => string;
  hideHeader?: boolean;
  inheritedAttributeIds?: string[];
}

export const ProductAttributesSection: React.FC<ProductAttributesSectionProps> = React.memo(({
  selectedAttributeIds,
  attributeDefaultOptions = {},
  onChange,
  onDefaultOptionChange,
  t,
  hideHeader = false,
  inheritedAttributeIds = [],
}) => {
  const attributes = useAttributes();
  const optionsMap = useAttributeStore(state => state.options);
  const { fetchAll } = useAttributeActions();
  const { loadOptions } = useOptionActions();
  const [showAddPanel, setShowAddPanel] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');

  // Use stable helper directly
  const getOptionsByAttributeId = attributeHelpers.getOptionsByAttributeId;

  // Load attributes on mount
  useEffect(() => {
    if (attributes.length === 0) {
      fetchAll();
    }
  }, []);

  // Load options for selected + inherited attributes
  useEffect(() => {
    [...selectedAttributeIds, ...inheritedAttributeIds].forEach(id => {
      const options = getOptionsByAttributeId(id);
      if (options.length === 0) {
        loadOptions(id);
      }
    });
  }, [selectedAttributeIds, inheritedAttributeIds]);

  const handleAddAttribute = (attributeId: string) => {
    onChange([...selectedAttributeIds, attributeId]);
    loadOptions(attributeId);
    setSearchTerm(''); // Clear search after adding
    // If only one match, it auto-selects, so maybe keep panel open or close?
    // Let's keep panel open if user wants to add more
  };

  const handleRemoveAttribute = (attributeId: string) => {
    if (inheritedAttributeIds.includes(attributeId)) return;
    onChange(selectedAttributeIds.filter(id => id !== attributeId));
    if (onDefaultOptionChange) {
      onDefaultOptionChange(attributeId, []);
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

  const getDefaultOptions = (attrId: string): string[] => {
    const val = attributeDefaultOptions[attrId];
    if (Array.isArray(val)) return val;
    if (val) return [val];
    return [];
  };

  const activeAttributes = attributes.filter(attr => attr.is_active);
  const allSelectedIds = [...selectedAttributeIds, ...inheritedAttributeIds];
  const selectedAttributes = activeAttributes.filter(attr => allSelectedIds.includes(String(attr.id)));
  const unselectedAttributes = activeAttributes.filter(attr =>
    !allSelectedIds.includes(String(attr.id))
  );

  const filteredUnselected = unselectedAttributes.filter(attr =>
    attr.name.toLowerCase().includes(searchTerm.toLowerCase())
  );

  return (
    <div className="flex flex-col h-full bg-gray-50/30">
      {!hideHeader && (
        <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900 pb-3 border-b border-gray-100 shrink-0 px-1">
          <Layers size={16} className="text-teal-500" />
          {t('settings.product.attribute.title')}
        </h3>
      )}

      {/* Add Attribute Header/Panel */}
      <div className="mb-2 pb-3 border-b border-gray-200 bg-white shrink-0 -mx-1 px-1">
        {!showAddPanel ? (
          <button
            onClick={() => setShowAddPanel(true)}
            className="w-full py-2.5 border-2 border-dashed border-gray-300 rounded-xl text-gray-500 font-medium hover:border-teal-400 hover:text-teal-600 hover:bg-teal-50 transition-all flex items-center justify-center gap-2"
          >
            <Plus size={18} />
            {t('settings.product.attribute.add')}
          </button>
        ) : (
          <div className="bg-white rounded-xl border border-gray-200 shadow-lg p-3 animate-in slide-in-from-top-2">
            <div className="flex items-center justify-between mb-2">
              <span className="text-sm font-bold text-gray-800">{t('settings.product.attribute.select')}</span>
              <button onClick={() => setShowAddPanel(false)} className="text-gray-400 hover:text-gray-600">
                <X size={16} />
              </button>
            </div>

            <div className="relative mb-2">
              <Search size={14} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-gray-400" />
              <input
                type="text"
                placeholder={t('common.action.search')}
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                autoFocus
                className="w-full pl-8 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-teal-500/20 focus:border-teal-500"
              />
            </div>

            <div className="max-h-[15rem] overflow-y-auto custom-scrollbar space-y-1">
              {filteredUnselected.length === 0 ? (
                <div className="text-center py-4 text-xs text-gray-400">
                  {unselectedAttributes.length === 0 ? (t('settings.all_attributes_selected')) : (t('common.empty.no_results'))}
                </div>
              ) : (
                filteredUnselected.map(attr => (
                  <button
                    key={String(attr.id)}
                    onClick={() => handleAddAttribute(String(attr.id))}
                    className="w-full text-left px-3 py-2 rounded-lg hover:bg-teal-50 hover:text-teal-700 text-sm text-gray-700 flex items-center justify-between group transition-colors"
                  >
                    <span>{attr.name}</span>
                    <Plus size={14} className="opacity-0 group-hover:opacity-100 transition-opacity" />
                  </button>
                ))
              )}
            </div>
          </div>
        )}
      </div>

      {/* Main Content Area */}
      <div className="flex-1 overflow-y-auto custom-scrollbar pr-1 py-2 space-y-3">
        {selectedAttributes.length === 0 ? (
           <div className="flex flex-col items-center justify-center py-10 text-gray-400 bg-white rounded-xl border border-dashed border-gray-200 mx-1">
             <Sliders size={32} className="opacity-20 mb-2" />
             <p className="text-sm">{t('settings.product.attribute.no_selected')}</p>
             <button
               onClick={() => setShowAddPanel(true)}
               className="mt-3 text-teal-600 text-sm font-medium hover:underline"
             >
               {t('settings.product.attribute.add')}
             </button>
           </div>
        ) : (
          selectedAttributes.map((attr) => {
            const attrId = String(attr.id);
            const isMulti = attr.is_multi_select;
            const defaultOptions = getDefaultOptions(attrId);
            const options = optionsMap.get(attrId) || [];
            const isInherited = inheritedAttributeIds.includes(attrId);

            return (
              <div key={attrId} className="bg-white rounded-xl border border-gray-100 overflow-hidden group">
                {/* Header */}
                <div className="px-3 py-2.5 flex items-center justify-between">
                  <div className="flex items-center gap-2 min-w-0">
                    <span className="font-semibold text-sm text-gray-800 truncate">{attr.name}</span>
                    <span className="text-[0.6rem] px-1.5 py-0.5 rounded bg-gray-100 text-gray-500 shrink-0">
                      {isMulti ? t('settings.attribute.type.multi_select') : t('settings.attribute.type.single_select')}
                    </span>
                  </div>
                  {isInherited ? (
                    <div className="text-gray-300 p-0.5" title={t('settings.product.attribute.inherited')}>
                      <Lock size={14} />
                    </div>
                  ) : (
                    <button
                      onClick={() => handleRemoveAttribute(attrId)}
                      className="text-gray-300 hover:text-red-500 transition-colors p-0.5 rounded hover:bg-red-50 opacity-0 group-hover:opacity-100"
                      title={t('common.action.remove')}
                    >
                      <X size={14} />
                    </button>
                  )}
                </div>

                {/* Options */}
                <div className="px-3 pb-3">
                  {options.length === 0 ? (
                    <div className="text-xs text-gray-400 italic py-1.5 text-center">{t('common.empty.no_data')}</div>
                  ) : (
                    <div className="flex flex-wrap gap-1.5">
                      {options.map((opt) => {
                        const optionKey = String(opt.index);
                        const isDefault = defaultOptions.includes(optionKey);
                        return (
                          <button
                            key={optionKey}
                            onClick={() => {
                              if (!onDefaultOptionChange) return;
                              let newDefaults: string[];
                              if (isMulti) {
                                newDefaults = isDefault
                                  ? defaultOptions.filter(id => id !== optionKey)
                                  : [...defaultOptions, optionKey];
                              } else {
                                newDefaults = isDefault ? [] : [optionKey];
                              }
                              onDefaultOptionChange(attrId, newDefaults);
                            }}
                            className={`
                              px-2.5 py-1 rounded-md text-xs font-medium transition-all flex items-center gap-1
                              ${isDefault
                                ? 'bg-teal-50 text-teal-700 border border-teal-300 ring-1 ring-teal-100'
                                : 'bg-gray-50 text-gray-600 border border-gray-200 hover:border-teal-200 hover:bg-teal-50/50'
                              }
                            `}
                          >
                            {opt.name}
                            {opt.price_modifier !== 0 && (
                              <span className={`text-[0.6rem] ${isDefault ? 'text-teal-500' : 'text-gray-400'}`}>
                                {opt.price_modifier > 0 ? '+' : ''}{formatCurrency(opt.price_modifier)}
                              </span>
                            )}
                          </button>
                        );
                      })}
                    </div>
                  )}
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
});
