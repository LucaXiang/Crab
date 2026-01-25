import React, { useEffect, useState } from 'react';
import { Image as ImageIcon, Tag, Hash, FileText, Layers, ImagePlus, Printer, List, Star, Check } from 'lucide-react';
import { FormField, FormSection, inputClass, selectClass, SelectField, KitchenPrinterSelector, AttributeDisplayTag } from '@/shared/components/FormField';
import { AttributeSelectionModal } from '@/features/attribute';
import { ProductImage } from './ProductImage';
import { useAttributeStore, useAttributes, useAttributeActions, useOptionActions, usePrintDestinationStore } from '@/core/stores/resources';
import { useIsKitchenPrintEnabled, useIsLabelPrintEnabled } from '@/core/stores/ui';
import { usePriceInput } from '@/hooks/usePriceInput';
import { Category, EmbeddedSpec, PrintState } from '@/core/domain/types';

interface ProductFormProps {
  formData: {
    id?: string; // Product ID (for editing existing product)
    name: string;
    receipt_name?: string;
    price: number;
    category?: string | number;
    image: string;
    externalId?: number;
    tax_rate: number;
    selected_attribute_ids?: string[];
    attribute_default_options?: Record<string, string[]>; // Product-level default options (array for multi-select)
    print_destinations?: string[];
    kitchen_print_name?: string;
    is_kitchen_print_enabled?: PrintState; // Kitchen print state: -1=inherit, 0=disabled, 1=enabled
    is_label_print_enabled?: PrintState;
    label_print_destinations?: string[]; // Label printer destinations
    is_active?: boolean;
    specs?: EmbeddedSpec[]; // Embedded specifications
    selected_tag_ids?: string[]; // Tag IDs loaded from getProductFull API
  };
  categories: Category[];
  onFieldChange: (field: string, value: any) => void;
  onSelectImage: () => void;
  t: (key: string) => string;
  inheritedAttributeIds?: string[];
}

// Label Printer Selector component (similar to KitchenPrinterSelector but for label printers)
const LabelPrinterSelector: React.FC<{
  value: string | null;
  onChange: (value: string | null) => void;
  t: (key: string) => string;
}> = ({ value, onChange, t }) => {
  const items = usePrintDestinationStore((state) => state.items);

  return (
    <FormField label={t('settings.label_printer')}>
      <div className="relative">
        <select
          value={value || ''}
          onChange={(e) => onChange(e.target.value || null)}
          className={selectClass}
        >
          <option value="">{t('common.label.default')}</option>
          {items.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
        <div className="pointer-events-none absolute inset-y-0 right-0 flex items-center px-3">
          <svg className="h-4 w-4 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </div>
    </FormField>
  );
};

export const ProductForm: React.FC<ProductFormProps> = ({
  formData,
  categories,
  onFieldChange,
  onSelectImage,
  t,
  inheritedAttributeIds = [],
}) => {
  const isGlobalKitchenEnabled = useIsKitchenPrintEnabled();
  const isGlobalLabelEnabled = useIsLabelPrintEnabled();
  const [showAttributeModal, setShowAttributeModal] = useState(false);
  const allAttributes = useAttributes();
  const optionsMap = useAttributeStore(state => state.options);
  const { loadAttributes } = useAttributeActions();
  const { loadOptions } = useOptionActions();

  const { priceInput, handlePriceChange, commitPrice, handlePriceKeyDown } = usePriceInput(
    formData.price || 0,
    {
      minValue: 0,
      onCommit: (value) => {
        // Update price in root spec (specs is the source of truth)
        const currentSpecs = formData.specs || [];
        if (currentSpecs.length === 0) {
          onFieldChange('specs', [{
            name: formData.name,
            price: value,
            display_order: 0,
            is_default: true,
            is_active: true,
            external_id: formData.externalId ?? null,
            receipt_name: null,
            is_root: true,
          }]);
        } else {
          const newSpecs = currentSpecs.map(s =>
            s.is_root ? { ...s, price: value } : s
          );
          onFieldChange('specs', newSpecs);
        }
      }
    }
  );

  useEffect(() => {
    if (allAttributes.length === 0) {
      loadAttributes();
    }
  }, []);

  // Ensure options are loaded for selected attributes so we can display default values
  useEffect(() => {
    if (formData.selected_attribute_ids) {
      formData.selected_attribute_ids.forEach(id => {
        const options = optionsMap.get(id);
        if (!options || options.length === 0) {
          loadOptions(id);
        }
      });
    }
  }, [formData.selected_attribute_ids]);

  const TAX_RATES = [
    { value: 21, label: t('settings.product.form.tax_rate_general') },
    { value: 10, label: t('settings.product.form.tax_rate_reduced') },
    { value: 4, label: t('settings.product.form.tax_rate_super_reduced') },
    { value: 0, label: t('settings.product.form.tax_rate_exempt') },
  ];

  // Get selected attribute objects for display
  const selectedAttributes = allAttributes.filter(attr =>
    formData.selected_attribute_ids?.includes(attr.id)
  );

  return (
    <div className="space-y-4">
      {/* Basic Info */}
      <FormSection title={t('settings.attribute.section.basic')} icon={Tag}>
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="col-span-1 md:col-span-2">
            <FormField label={t('settings.product.form.name')} required>
              <input
                value={formData.name}
                onChange={(e) => onFieldChange('name', e.target.value)}
                placeholder={t('settings.product.form.name_placeholder')}
                className={inputClass}
              />
            </FormField>
          </div>

          <SelectField
            label={t('settings.product.form.category')}
            value={formData.category ?? ''}
            onChange={(value) => onFieldChange('category', value)}
            options={categories.map(c => ({ value: c.id ?? '', label: c.name }))}
            placeholder={t('settings.product.form.select_category')}
            required
          />

          <FormField label={t('settings.product.form.price')} required>
            <div className="relative">
              <span className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-500 font-medium">€</span>
              <input
                type="text"
                inputMode="decimal"
                value={priceInput}
                onChange={handlePriceChange}
                onBlur={commitPrice}
                onFocus={(e) => e.currentTarget.select()}
                onKeyDown={handlePriceKeyDown}
                placeholder={t('settings.form.placeholder.price')}
                className={`${inputClass} pl-8 font-mono font-medium`}
              />
            </div>
          </FormField>

          <SelectField
            label={t('settings.product.form.tax_rate')}
            value={formData.tax_rate ?? 10}
            onChange={(value) => {
              const val = typeof value === 'number' ? value : parseInt(value as string, 10);
              onFieldChange('tax_rate', isNaN(val) ? 10 : val);
            }}
            options={TAX_RATES.map(rate => ({ value: rate.value, label: rate.label }))}
            required
          />

          <FormField label={t('settings.product.form.external_id')}>
            <div className="relative">
              <div className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400">
                <Hash size={14} />
              </div>
              <input
                type="number"
                value={formData.externalId ?? ''}
                onChange={(e) => {
                  const val = e.target.value;
                  const newExternalId = val ? parseInt(val, 10) : null;
                  // Update external_id in root spec (specs is the source of truth)
                  const currentSpecs = formData.specs || [];
                  if (currentSpecs.length === 0) {
                    onFieldChange('specs', [{
                      name: formData.name,
                      price: formData.price ?? 0,
                      display_order: 0,
                      is_default: true,
                      is_active: true,
                      external_id: newExternalId,
                      receipt_name: null,
                      is_root: true,
                    }]);
                  } else {
                    const newSpecs = currentSpecs.map(s =>
                      s.is_root ? { ...s, external_id: newExternalId } : s
                    );
                    onFieldChange('specs', newSpecs);
                  }
                }}
                placeholder={t('settings.product.form.external_id_placeholder')}
                className={`${inputClass} pl-9`}
              />
            </div>
          </FormField>
        </div>
      </FormSection>

      {/* Print Settings */}
      <FormSection title={t('settings.attribute.section.print')} icon={Printer}>
        {/* Kitchen Printing */}
        <div className="space-y-3">
          <h4 className="text-xs font-bold text-gray-500 uppercase tracking-wider">
            {t('settings.product.print.kitchen_printing')}
          </h4>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <FormField label={t('settings.product.print.is_kitchen_print_enabled')}>
              <div className="relative">
                <select
                  value={
                    formData.is_kitchen_print_enabled === undefined || formData.is_kitchen_print_enabled === null || formData.is_kitchen_print_enabled === -1
                      ? '-1'
                      : String(formData.is_kitchen_print_enabled)
                  }
                  onChange={(e) => {
                    const raw = e.target.value;
                    const num = parseInt(raw, 10);
                    const next: PrintState = isNaN(num) ? -1 : (num === 1 ? 1 : num === 0 ? 0 : -1);
                    onFieldChange('is_kitchen_print_enabled', next);
                  }}
                  className={selectClass}
                >
                  <option value="-1">{t('common.label.default')}</option>
                  <option value="1">{t('common.status.enabled')}</option>
                  <option value="0">{t('common.status.disabled')}</option>
                </select>
                {(formData.is_kitchen_print_enabled === undefined || formData.is_kitchen_print_enabled === null || formData.is_kitchen_print_enabled === -1) && (
                  <div className="mt-1.5 text-xs text-gray-500 flex items-center gap-1.5">
                    <div className="w-1.5 h-1.5 rounded-full bg-blue-400"></div>
                    <span>
                      {t('settings.product.print.effective_state')}: {
                        (() => {
                          if (!isGlobalKitchenEnabled) return t('common.status.disabled_global');
                          const cat = categories.find(c => String(c.id) === String(formData.category));
                          const isEnabled = cat ? (cat.kitchen_print_destinations && cat.kitchen_print_destinations.length > 0) : false;
                          return isEnabled ? t('common.status.enabled') : t('common.status.disabled');
                        })()
                      }
                    </span>
                  </div>
                )}
              </div>
            </FormField>

            <KitchenPrinterSelector
              value={formData.print_destinations?.[0] ?? null}
              onChange={(value) => {
                onFieldChange('print_destinations', value === null ? [] : [value]);
              }}
              t={t}
            />

            <div className="col-span-1 md:col-span-2">
              <FormField label={t('settings.product.print.kitchen_print_name')}>
                <input
                  value={formData.kitchen_print_name || ''}
                  onChange={(e) => onFieldChange('kitchen_print_name', e.target.value)}
                  placeholder={t('settings.product.print.kitchen_print_name_placeholder')}
                  className={inputClass}
                />
              </FormField>
            </div>
          </div>
        </div>

        <div className="border-t border-gray-100 pt-3 mt-3" />

        {/* Label Printing */}
        <div className="space-y-3">
          <h4 className="text-xs font-bold text-gray-500 uppercase tracking-wider">
            {t('settings.product.print.label_printing')}
          </h4>
          <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
            <FormField label={t('settings.product.print.is_label_print_enabled')}>
              <div className="relative">
                <select
                  value={
                    formData.is_label_print_enabled === undefined || formData.is_label_print_enabled === null || formData.is_label_print_enabled === -1
                      ? '-1'
                      : String(formData.is_label_print_enabled)
                  }
                  onChange={(e) => {
                    const raw = e.target.value;
                    const num = parseInt(raw, 10);
                    const next: PrintState = isNaN(num) ? -1 : (num === 1 ? 1 : num === 0 ? 0 : -1);
                    onFieldChange('is_label_print_enabled', next);
                  }}
                  className={selectClass}
                >
                  <option value="-1">{t('common.label.default')}</option>
                  <option value="1">{t('common.status.enabled')}</option>
                  <option value="0">{t('common.status.disabled')}</option>
                </select>
                {(formData.is_label_print_enabled === undefined || formData.is_label_print_enabled === null || formData.is_label_print_enabled === -1) && (
                  <div className="mt-1.5 text-xs text-gray-500 flex items-center gap-1.5">
                    <div className="w-1.5 h-1.5 rounded-full bg-blue-400"></div>
                    <span>
                      {t('settings.product.print.effective_state')}: {
                        (() => {
                          if (!isGlobalLabelEnabled) return t('common.status.disabled_global');
                          const cat = categories.find(c => String(c.id) === String(formData.category));
                          const isEnabled = cat ? (cat.is_label_print_enabled !== false) : true;
                          return isEnabled ? t('common.status.enabled') : t('common.status.disabled');
                        })()
                      }
                    </span>
                  </div>
                )}
              </div>
            </FormField>

            <LabelPrinterSelector
              value={formData.label_print_destinations?.[0] ?? null}
              onChange={(value) => {
                onFieldChange('label_print_destinations', value === null ? [] : [value]);
              }}
              t={t}
            />
          </div>
        </div>

        <div className="border-t border-gray-100 pt-3 mt-3" />

        {/* Receipt Printing */}
        <div className="space-y-3">
          <h4 className="text-xs font-bold text-gray-500 uppercase tracking-wider">
            {t('settings.product.print.receipt_printing')}
          </h4>
          <FormField label={t('settings.product.print.receipt_name')}>
            <div className="relative">
              <div className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400">
                <FileText size={14} />
              </div>
              <input
                value={formData.receipt_name || ''}
                onChange={(e) => onFieldChange('receipt_name', e.target.value)}
                placeholder={t('settings.product.print.receipt_name_placeholder')}
                className={`${inputClass} pl-9`}
              />
            </div>
          </FormField>
        </div>
      </FormSection>

      {/* Image */}
      <FormSection title={t('settings.product.form.image')} icon={ImagePlus} defaultCollapsed>
        <div className="flex items-center gap-4 p-3 bg-gray-50 rounded-xl border border-dashed border-gray-200">
          <div
            className="w-16 h-16 shrink-0 bg-white rounded-lg border border-gray-200 flex items-center justify-center overflow-hidden cursor-pointer hover:border-teal-300 transition-colors"
            onClick={onSelectImage}
          >
            {formData.image ? (
              <ProductImage
                src={formData.image}
                alt="preview"
                className="w-full h-full object-cover"
              />
            ) : (
              <ImageIcon size={24} className="text-gray-300" />
            )}
          </div>

          <div className="flex-1 min-w-0">
            <div className="flex gap-2">
              <button
                type="button"
                onClick={onSelectImage}
                className="px-3 py-1.5 text-xs font-medium text-gray-700 bg-white border border-gray-200 rounded-lg hover:bg-gray-50 transition-colors"
              >
                {formData.image ? t('common.action.change') : t('common.action.upload_image')}
              </button>
              {formData.image && (
                <button
                  type="button"
                  onClick={() => onFieldChange('image', '')}
                  className="px-3 py-1.5 text-xs font-medium text-red-600 bg-red-50 border border-red-100 rounded-lg hover:bg-red-100 transition-colors"
                >
                  {t('common.action.remove')}
                </button>
              )}
            </div>
            <p className="mt-1 text-xs text-gray-400 truncate">
              {t('settings.product.form.image_hint')}
            </p>
          </div>
        </div>
      </FormSection>

      {/* Attributes */}
      <FormSection title={t('settings.product.attribute.title')} icon={Layers}>
        <div className="flex items-center justify-between mb-3">
          <p className="text-xs text-gray-500">{t('settings.product.attribute.description')}</p>
          <button
            type="button"
            onClick={() => setShowAttributeModal(true)}
            className="text-xs font-bold text-teal-600 hover:text-teal-700 hover:underline"
          >
            {t('settings.product.attribute.manage')}
          </button>
        </div>

        <div className="min-h-[3.75rem]">
          {selectedAttributes.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {selectedAttributes.map((attr) => {
                const rawDefaults = formData.attribute_default_options?.[attr.id];
                const defaultOptionIds = Array.isArray(rawDefaults)
                  ? rawDefaults
                  : (rawDefaults ? [rawDefaults] : []);

                return (
                  <AttributeDisplayTag
                    key={attr.id}
                    attribute={attr}
                    defaultOptionIds={defaultOptionIds}
                    isInherited={inheritedAttributeIds.includes(attr.id)}
                    t={t}
                  />
                );
              })}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-4 text-gray-400 bg-gray-50 rounded-lg border border-dashed border-gray-200">
              <p className="text-sm">{t('settings.product.attribute.no_selected')}</p>
            </div>
          )}
        </div>
      </FormSection>

      <AttributeSelectionModal
        isOpen={showAttributeModal}
        onClose={() => setShowAttributeModal(false)}
        selectedAttributeIds={formData.selected_attribute_ids || []}
        attributeDefaultOptions={formData.attribute_default_options || {}}
        onChange={(ids) => onFieldChange('selected_attribute_ids', ids)}
        onDefaultOptionChange={(attrId, optionIds) => {
          const newDefaults = { ...formData.attribute_default_options, [attrId]: optionIds };
          if (!optionIds || optionIds.length === 0) delete newDefaults[attrId];
          onFieldChange('attribute_default_options', newDefaults);
        }}
        t={t}
        inheritedAttributeIds={inheritedAttributeIds}
      />

      {/* Specifications - only show when multiple specs exist */}
      {formData.specs && formData.specs.length > 1 && (
        <FormSection title={t('specification.list')} icon={List} defaultCollapsed>
          <div className="space-y-3">
            {formData.specs.map((spec, index) => (
              <div
                key={index}
                className={`p-3 rounded-lg border ${spec.is_root ? 'bg-amber-50 border-amber-200' : 'bg-gray-50 border-gray-200'}`}
              >
                <div className="flex items-center justify-between mb-2">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-gray-900">{spec.name}</span>
                    {spec.is_root && (
                      <span className="px-2 py-0.5 text-xs font-medium bg-amber-100 text-amber-700 rounded">
                        {t('specification.label.root')}
                      </span>
                    )}
                    {spec.is_default && (
                      <span className="px-2 py-0.5 text-xs font-medium bg-teal-100 text-teal-700 rounded">
                        {t('specification.label.default')}
                      </span>
                    )}
                  </div>
                  <span className="text-sm font-mono text-gray-600">EUR{spec.price.toFixed(2)}</span>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-3 mt-2">
                  {/* Receipt Name */}
                  <FormField label={t('specification.form.receipt_name')}>
                    <input
                      value={spec.receipt_name || ''}
                      onChange={(e) => {
                        const newSpecs = [...formData.specs!];
                        newSpecs[index] = { ...newSpecs[index], receipt_name: e.target.value || null };
                        onFieldChange('specs', newSpecs);
                      }}
                      placeholder={t('specification.form.receipt_name_placeholder')}
                      className={inputClass}
                    />
                  </FormField>

                  {/* Is Root Toggle */}
                  <FormField label={t('specification.form.is_root')}>
                    <button
                      type="button"
                      onClick={() => {
                        const newSpecs = formData.specs!.map((s, i) => ({
                          ...s,
                          is_root: i === index
                        }));
                        onFieldChange('specs', newSpecs);
                      }}
                      className={`flex items-center gap-2 px-3 py-2 rounded-lg border transition-colors ${
                        spec.is_root
                          ? 'bg-amber-100 border-amber-300 text-amber-700'
                          : 'bg-white border-gray-200 text-gray-600 hover:bg-gray-50'
                      }`}
                    >
                      {spec.is_root ? <Check size={16} /> : <Star size={16} />}
                      <span className="text-sm">
                        {spec.is_root ? t('specification.label.root') : t('specification.form.is_root')}
                      </span>
                    </button>
                    <p className="mt-1 text-xs text-gray-500">{t('specification.form.is_root_hint')}</p>
                  </FormField>
                </div>
              </div>
            ))}
          </div>
        </FormSection>
      )}

    </div>
  );
};
