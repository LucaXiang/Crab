import React, { useMemo } from 'react';
import { createPortal } from 'react-dom';
import { X, Check } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { AttributeSelector } from './AttributeSelector';
import { ItemActionPanel } from '../ui/ItemActionPanel';
import { AttributeTemplate, AttributeOption, ProductAttribute, EmbeddedSpec } from '@/core/domain/types';
import { formatCurrency } from '@/utils/currency/formatCurrency';

interface ItemConfiguratorModalProps {
  isOpen: boolean;
  onClose: () => void;

  // Header Info
  title: string;       // e.g. "Select Options" or "Edit Item"
  productName: string; // e.g. "Cheeseburger"

  // Data & State
  isLoading?: boolean;
  attributes: AttributeTemplate[];
  allOptions: Map<string, AttributeOption[]>;
  bindings?: ProductAttribute[];
  selections: Map<string, string[]>;
  onAttributeSelect: (attributeId: string, optionIds: string[]) => void;

  // Specification Selection (embedded specs, use index as ID)
  specifications?: EmbeddedSpec[];
  hasMultiSpec?: boolean;
  selectedSpecId?: string | null;
  onSpecificationSelect?: (specId: string) => void;

  // Pricing & Quantity
  basePrice: number;
  quantity: number;
  discount: number;
  onQuantityChange: (val: number) => void;
  onDiscountChange: (val: number, authorizer?: { id: string; username: string }) => void;

  // Actions
  onConfirm: () => void;
  confirmLabel?: string;
  onDelete?: (authorizer?: { id: string; username: string }) => void;
  showDelete?: boolean;
  readOnlyAttributes?: boolean;
}

export const ItemConfiguratorModal: React.FC<ItemConfiguratorModalProps> = ({
  isOpen,
  onClose,
  title,
  productName,
  isLoading = false,
  attributes,
  allOptions,
  bindings = [],
  selections,
  onAttributeSelect,
  specifications,
  hasMultiSpec,
  selectedSpecId,
  onSpecificationSelect,
  basePrice,
  quantity,
  discount,
  onQuantityChange,
  onDiscountChange,
  onConfirm,
  confirmLabel,
  onDelete,
  showDelete = false,
  readOnlyAttributes = false,
}) => {
  const { t } = useI18n();

  // Calculate options modifier locally since we have all the data
  const optionsModifier = useMemo(() => {
    let mod = 0;
    selections.forEach((idxs, attrId) => {
      const opts = allOptions.get(attrId) || [];
      idxs.forEach(idxStr => {
        const idx = parseInt(idxStr, 10);
        const opt = opts[idx];
        if (opt) mod += opt.price_modifier;
      });
    });
    return mod;
  }, [selections, allOptions]);

  if (!isOpen) return null;

  const hasAttributes = attributes.length > 0;
  const hasSpecs = hasMultiSpec && specifications && specifications.length > 0;
  // If loading, we assume we might have attributes/specs, so keep layout wide or show spinner
  // If not loading and no attributes/specs, narrow layout
  const showAttributesColumn = isLoading || hasAttributes || hasSpecs;

  return createPortal(
    <div className="fixed inset-0 z-100 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4 animate-in fade-in duration-200">
      <div 
        className={`bg-white rounded-3xl w-full ${showAttributesColumn ? 'max-w-4xl' : 'max-w-md'} h-[70vh] overflow-hidden shadow-2xl flex flex-col transition-all duration-300`}
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="px-6 py-4 border-b border-gray-100 bg-white shrink-0 z-10 flex items-center justify-between">
          <div>
            <div className="text-xs font-bold text-gray-400 uppercase tracking-wider mb-1">{title}</div>
            <h2 className="text-xl font-bold text-gray-800 line-clamp-1">{productName}</h2>
          </div>
          <button
            onClick={onClose}
            className="w-12 h-12 flex items-center justify-center bg-gray-50 hover:bg-gray-100 rounded-full transition-colors active:scale-95"
          >
            <X size={24} className="text-gray-600" />
          </button>
        </div>

        {/* Content - Two Column Layout */}
        <div className="flex flex-1 overflow-hidden">
          {/* Left: Attributes Section */}
          {showAttributesColumn && (
            <div className="flex-1 overflow-y-auto p-6 custom-scrollbar bg-gray-50/50">
              {isLoading ? (
                <div className="flex justify-center py-20">
                  <div className="animate-spin rounded-full h-10 w-10 border-b-2 border-orange-500"></div>
                </div>
              ) : (
                <div className="max-w-3xl mx-auto space-y-8">
                  {/* Specification Selection */}
                  {hasMultiSpec && specifications && specifications.length > 0 && (
                    <div className="bg-white p-5 rounded-2xl shadow-sm border border-gray-100">
                      <div className="mb-3">
                        <h3 className="text-sm font-bold text-gray-900 flex items-center gap-2">
                          <span>{t('pos.product.select_specification')}</span>
                          <span className="text-xs text-red-500">*</span>
                        </h3>
                      </div>
                      <div className="grid grid-cols-3 gap-2">
                        {specifications
                          .map((spec, specIdx) => ({ spec, specIdx }))
                          .filter(({ spec }) => spec.is_active)
                          .map(({ spec, specIdx }) => {
                            const isSelected = selectedSpecId === String(specIdx);
                            return (
                              <button
                                key={specIdx}
                                type="button"
                                onClick={() => onSpecificationSelect?.(String(specIdx))}
                                className={`
                                  relative p-3 rounded-xl border-2 transition-all text-left flex flex-col items-start min-h-[4.375rem] justify-center
                                  ${isSelected
                                    ? 'border-orange-500 bg-orange-50 ring-2 ring-orange-200'
                                    : 'border-gray-200 hover:border-orange-300 bg-white hover:bg-orange-50/30'
                                  }
                                `}
                              >
                                <span className={`text-sm font-semibold mb-1 ${isSelected ? 'text-orange-900' : 'text-gray-900'}`}>
                                  {spec.is_default && !spec.name ? t('settings.product.specification.label.default') : spec.name}
                                </span>
                                {spec.is_default && !isSelected && (
                                  <span className="absolute top-2 right-2 w-2 h-2 bg-blue-500 rounded-full border border-white" title={t('common.label.default')} />
                                )}
                                <div className={`text-sm font-bold ${isSelected ? 'text-orange-600' : 'text-gray-700'}`}>
                                  {formatCurrency(spec.price)}
                                </div>
                                {isSelected && (
                                  <div className="absolute top-2 right-2">
                                    <div className="w-4 h-4 bg-orange-500 rounded-full flex items-center justify-center">
                                      <Check size={10} className="text-white" strokeWidth={3} />
                                    </div>
                                  </div>
                                )}
                              </button>
                            );
                          })}
                      </div>
                    </div>
                  )}

                  {/* Attribute Selectors */}
                  {hasAttributes && attributes.map((attr) => {
                    const attrId = String(attr.id);
                    const options = allOptions.get(attrId) || [];
                    const selectedOptionIds = selections.get(attrId) || [];
                    // binding.to is the attribute ID in AttributeBinding relation
                    const binding = bindings?.find(b => b.to === attr.id);

                    // Logic to find defaults for display (visual cues in AttributeSelector)
                    // default_option_idx is stored at the attribute level
                    const attrDefaultIdx = attr.default_option_idx;
                    let defaultOptionIds = (attrDefaultIdx !== null && attrDefaultIdx !== undefined && attrDefaultIdx >= 0) ? [String(attrDefaultIdx)] : [];

                    return (
                      <div key={attrId} className="bg-white p-5 rounded-2xl shadow-sm border border-gray-100">
                        <AttributeSelector
                          attribute={attr}
                          options={options}
                          selectedOptionIds={selectedOptionIds}
                          defaultOptionIds={defaultOptionIds}
                          onSelect={(optionIds) => onAttributeSelect(attrId, optionIds)}
                        />
                      </div>
                    );
                  })}

                  {/* Show message only if no specifications and no attributes */}
                  {!hasMultiSpec && !hasAttributes && (
                    <div className="h-full flex flex-col items-center justify-center text-gray-400 space-y-4">
                      <div className="p-4 bg-gray-50 rounded-full">
                        <Check size={32} className="text-gray-300" />
                      </div>
                      <p>{t('pos.product.no_options')}</p>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}

          {/* Right: Action Panel */}
          <div className={`${showAttributesColumn ? 'w-[27.5rem] border-l border-gray-100' : 'w-full'} bg-white shadow-xl z-20 flex flex-col h-full transition-all duration-300`}>
            <ItemActionPanel
              t={t}
              quantity={quantity}
              discount={discount}
              basePrice={basePrice}
              optionsModifier={optionsModifier}
              onQuantityChange={onQuantityChange}
              onDiscountChange={onDiscountChange}
              onConfirm={onConfirm}
              onCancel={onClose}
              onDelete={onDelete}
              confirmLabel={confirmLabel}
              showDelete={showDelete}
            />
          </div>
        </div>
      </div>
    </div>,
    document.body
  );
};
