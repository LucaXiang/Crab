import React, { useEffect, useState } from 'react';
import { Sliders, Check, Layers, Plus, X, Search, AlertCircle, Lock } from 'lucide-react';
import { useAttributeStore, useAttributes, useAttributeActions, useOptionActions, attributeHelpers } from '@/core/stores/resources';

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
  const { loadAttributes } = useAttributeActions();
  const { loadOptions } = useOptionActions();
  const [showAddPanel, setShowAddPanel] = useState(false);
  const [searchTerm, setSearchTerm] = useState('');

  // Use stable helper directly
  const getOptionsByAttributeId = attributeHelpers.getOptionsByAttributeId;

  // Load attributes on mount
  useEffect(() => {
    if (attributes.length === 0) {
      loadAttributes();
    }
  }, []);

  // Load options for selected attributes
  useEffect(() => {
    selectedAttributeIds.forEach(id => {
      const options = getOptionsByAttributeId(id);
      if (options.length === 0) {
        loadOptions(id);
      }
    });
  }, [selectedAttributeIds]);

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
      SINGLE_REQUIRED: t('settings.attribute.type.singleRequired'),
      SINGLE_OPTIONAL: t('settings.attribute.type.singleOptional'),
      MULTI_REQUIRED: t('settings.attribute.type.multiRequired'),
      MULTI_OPTIONAL: t('settings.attribute.type.multiOptional'),
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
  const selectedAttributes = activeAttributes.filter(attr => selectedAttributeIds.includes(String(attr.id)));
  const unselectedAttributes = activeAttributes.filter(attr => !selectedAttributeIds.includes(String(attr.id)));

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
                placeholder={t('common.search')}
                value={searchTerm}
                onChange={(e) => setSearchTerm(e.target.value)}
                autoFocus
                className="w-full pl-8 pr-3 py-1.5 text-sm border border-gray-200 rounded-lg focus:outline-none focus:ring-2 focus:ring-teal-500/20 focus:border-teal-500"
              />
            </div>

            <div className="max-h-[240px] overflow-y-auto custom-scrollbar space-y-1">
              {filteredUnselected.length === 0 ? (
                <div className="text-center py-4 text-xs text-gray-400">
                  {unselectedAttributes.length === 0 ? (t('settings.allAttributesSelected')) : (t('common.noResults'))}
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
             <p className="text-sm">{t('settings.product.attribute.noSelected')}</p>
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
            const isMulti = (attr.attr_type || '').startsWith('MULTI');
            const defaultOptions = getDefaultOptions(attrId);
            const options = optionsMap.get(attrId) || [];
            const hasDefault = defaultOptions.length > 0;

            return (
              <div key={attrId} className="bg-white rounded-xl border border-gray-200 shadow-sm overflow-hidden group hover:border-teal-200 hover:shadow-md transition-all duration-200">
                {/* Header */}
                <div className="px-4 py-3 border-b border-gray-50 flex items-center justify-between bg-gray-50/50">
                  <div className="flex items-center gap-2">
                    <span className="font-bold text-gray-800">{attr.name}</span>
                    <span className={`text-[10px] px-1.5 py-0.5 rounded border ${
                      (attr.attr_type || '').includes('REQUIRED')
                        ? 'bg-red-50 text-red-600 border-red-100'
                        : 'bg-blue-50 text-blue-600 border-blue-100'
                    }`}>
                      {getAttributeTypeLabel(attr.attr_type)}
                    </span>
                  </div>
                  {inheritedAttributeIds.includes(attrId) ? (
                    <div className="text-gray-400 p-1" title={t('settings.product.attribute.inherited')}>
                      <Lock size={16} />
                    </div>
                  ) : (
                    <button
                      onClick={() => handleRemoveAttribute(attrId)}
                      className="text-gray-400 hover:text-red-500 transition-colors p-1 rounded-md hover:bg-red-50 opacity-0 group-hover:opacity-100"
                      title={t('common.remove')}
                    >
                      <X size={16} />
                    </button>
                  )}
                </div>

                {/* Options Area */}
                <div className="p-4">
                  {options.length === 0 ? (
                    <div className="text-sm text-gray-400 italic py-2 text-center">{t('settings.attribute.option.noData')}</div>
                  ) : (
                    <div className="flex flex-wrap gap-2 max-h-[220px] overflow-y-auto custom-scrollbar content-start">
                      {options.map((opt) => {
                        // Use index as option identifier since AttributeOption doesn't have id
                        const optionKey = String(opt.index);
                        const isSelected = defaultOptions.includes(optionKey);
                        return (
                          <button
                            key={optionKey}
                            onClick={() => {
                              if (!onDefaultOptionChange) return;
                              let newDefaults: string[];
                              if (isMulti) {
                                // Multi-select toggle
                                newDefaults = isSelected
                                  ? defaultOptions.filter(id => id !== optionKey)
                                  : [...defaultOptions, optionKey];
                              } else {
                                // Single-select toggle (click selected to unselect)
                                newDefaults = isSelected ? [] : [optionKey];
                              }
                              onDefaultOptionChange(attrId, newDefaults);
                            }}
                            className={`
                              relative px-3 py-1.5 rounded-lg text-sm border font-medium transition-all duration-200 flex items-center gap-1.5
                              ${isSelected
                                ? 'bg-teal-500 text-white border-teal-600 shadow-sm ring-2 ring-teal-200 ring-offset-1'
                                : 'bg-white text-gray-600 border-gray-200 hover:border-teal-300 hover:bg-teal-50'
                              }
                            `}
                          >
                            {isSelected && <Check size={12} strokeWidth={3} />}
                            {opt.name}
                            {opt.price_modifier !== 0 && (
                              <span className={`text-[10px] ml-0.5 ${isSelected ? 'text-teal-100' : 'text-gray-400'}`}>
                                {opt.price_modifier > 0 ? '+' : ''}{opt.price_modifier}
                              </span>
                            )}
                          </button>
                        );
                      })}
                    </div>
                  )}
                  
                  {/* Validation/Hint Status */}
                  <div className="mt-3 flex items-center justify-between text-xs">
                     <span className="text-gray-400">
                        {isMulti ? (t('settings.product.attribute.hint.multiSelect')) : (t('settings.product.attribute.hint.singleSelect'))}
                     </span>
                     {!hasDefault && (
                       <span className="flex items-center gap-1 text-amber-600 bg-amber-50 px-2 py-0.5 rounded-full border border-amber-100">
                         <AlertCircle size={10} />
                         {t('settings.product.attribute.hint.noDefault')}
                       </span>
                     )}
                     {hasDefault && (
                        <span className="flex items-center gap-1 text-green-600 bg-green-50 px-2 py-0.5 rounded-full border border-green-100">
                          <Check size={10} />
                          {t('settings.product.attribute.hint.defaultSet')}
                        </span>
                     )}
                  </div>
                </div>
              </div>
            );
          })
        )}
      </div>
    </div>
  );
});
