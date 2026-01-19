import React, { useEffect, useState } from 'react';
import { Image as ImageIcon, Tag, Hash, FileText, Layers, MoreHorizontal, PackagePlus, Printer } from 'lucide-react';
import { FormField, inputClass, selectClass } from './FormField';
import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { createApiClient } from '@/infrastructure/api';
import { AttributeSelectionModal } from './AttributeSelectionModal';
import { useAttributeStore, useAttributes, useAttributeActions, useOptionActions } from '@/core/stores/resources';
import { useIsKitchenPrintEnabled, useIsLabelPrintEnabled } from '@/core/stores/ui';
import { usePriceInput } from '@/hooks/usePriceInput';
import { SelectField } from '@/presentation/components/form/FormField/SelectField';
import { KitchenPrinterSelector } from '@/presentation/components/form/FormField/KitchenPrinterSelector';
import { AttributeDisplayTag } from '@/presentation/components/form/FormField/AttributeDisplayTag';
import { SpecificationManagementModal } from '../components/SpecificationManagementModal';
import { Category, ProductSpecification } from '@/core/domain/types';
import { formatCurrency } from '@/utils/currency';

// API client for fetching specs
const api = createApiClient();

interface ProductFormProps {
  formData: {
    id?: string; // Product ID (for editing existing product)
    name: string;
    receiptName?: string;
    price: number;
    categoryId?: number;
    image: string;
    externalId?: number;
    taxRate: number;
    selectedAttributeIds?: string[];
    attributeDefaultOptions?: Record<string, string[]>; // Product-level default options (array for multi-select)
    kitchenPrinterId?: number | null;
    kitchenPrintName?: string;
    isKitchenPrintEnabled?: number | null;
    isLabelPrintEnabled?: number | null;
    hasMultiSpec?: boolean; // Whether this product has multiple specifications
    tempSpecifications?: ProductSpecification[];
  };
  categories: Category[];
  onFieldChange: (field: string, value: any) => void;
  onSelectImage: () => void;
  t: (key: string) => string;
  inheritedAttributeIds?: string[];
}

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
  const [showSpecModal, setShowSpecModal] = useState(false);
  const allAttributes = useAttributes();
  const optionsMap = useAttributeStore(state => state.options);
  const { loadAttributes } = useAttributeActions();
  const { loadOptions } = useOptionActions();

  const { priceInput, handlePriceChange, commitPrice, handlePriceKeyDown } = usePriceInput(
    formData.price || 0,
    {
      minValue: 0,
      onCommit: (value) => onFieldChange('price', value)
    }
  );

  useEffect(() => {
  }, [formData.isKitchenPrintEnabled]);

  useEffect(() => {
    if (allAttributes.length === 0) {
      loadAttributes();
    }
  }, []);

  // Ensure options are loaded for selected attributes so we can display default values
  useEffect(() => {
    if (formData.selectedAttributeIds) {
      formData.selectedAttributeIds.forEach(id => {
        const options = optionsMap.get(id);
        if (!options || options.length === 0) {
          loadOptions(id);
        }
      });
    }
  }, [formData.selectedAttributeIds]);

  const [previewSpecs, setPreviewSpecs] = useState<ProductSpecification[]>([]);

  // Fetch specs for preview
  useEffect(() => {
    const fetchSpecs = async () => {
      if (formData.id) {
        try {
          const response = await api.listProductSpecs(Number(formData.id));
          setPreviewSpecs(response.data?.specs || []);
        } catch (e) {
          console.error('Failed to fetch specs for preview:', e);
        }
      } else {
        setPreviewSpecs(formData.tempSpecifications || []);
      }
    };

    // Fetch on mount and when modal closes to ensure latest data
    if (!showSpecModal) {
      fetchSpecs();
    }
  }, [formData.id, showSpecModal, formData.tempSpecifications]);

  const TAX_RATES = [
    { value: 0.21, label: t('settings.product.form.taxRateGeneral') },
    { value: 0.10, label: t('settings.product.form.taxRateReduced') },
    { value: 0.04, label: t('settings.product.form.taxRateSuperReduced') },
    { value: 0.00, label: t('settings.product.form.taxRateExempt') },
  ];

  // Get selected attribute objects for display
  const selectedAttributes = allAttributes.filter(attr =>
    formData.selectedAttributeIds?.includes(attr.id)
  );

  return (
    <div className="space-y-6">
      {/* Block 1: Basic Info (Required) */}
      <section className="bg-white rounded-xl border border-gray-100 p-4 space-y-4 shadow-sm">
        <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900 pb-2 border-b border-gray-100">
          <Tag size={16} className="text-orange-500" />
          {t('settings.form.basicInfo')}
          <span className="text-xs font-normal text-gray-400 ml-auto">{t('common.required')}</span>
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="col-span-1 md:col-span-2">
            <FormField label={t('settings.product.form.name')} required>
              <input
                value={formData.name}
                onChange={(e) => onFieldChange('name', e.target.value)}
                placeholder={t('settings.product.form.namePlaceholder')}
                className={inputClass}
              />
            </FormField>
          </div>

          <SelectField
            label={t('settings.product.form.category')}
            value={formData.categoryId ?? ''}
            onChange={(value) => onFieldChange('categoryId', Number(value))}
            options={categories.map(c => ({ value: c.id, label: c.name }))}
            placeholder={t('settings.product.form.selectCategory')}
            required
          />

          <FormField
            label={t('settings.product.form.price')}
            required
          >
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
            {formData.hasMultiSpec && (
              <p className="text-xs text-orange-500 mt-1">
                {t('settings.product.form.priceBaseHint')}
              </p>
            )}
          </FormField>

          <SelectField
            label={t('settings.product.form.taxRate')}
            value={formData.taxRate?.toString() || '0.1'}
            onChange={(value) => {
              const val = parseFloat(value as string);
              onFieldChange('taxRate', isNaN(val) ? 0.10 : val);
            }}
            options={TAX_RATES.map(rate => ({ value: rate.value.toString(), label: rate.label }))}
            required
          />

          <FormField label={t('settings.product.form.externalId')} required>
            <div className="relative">
              <div className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400">
                <Hash size={14} />
              </div>
              <input
                type="number"
                value={formData.externalId ?? ''}
                onChange={(e) => {
                  const val = e.target.value;
                  onFieldChange('externalId', val ? parseInt(val, 10) : undefined);
                }}
                placeholder={t('settings.product.form.externalIdPlaceholder')}
                className={`${inputClass} pl-9`}
              />
            </div>
          </FormField>
        </div>
      </section>

      {/* Block 2: Print Settings */}
      <section className="bg-white rounded-xl border border-gray-100 p-4 space-y-6 shadow-sm">
        <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900 pb-2 border-b border-gray-100">
          <Printer size={16} className="text-teal-500" />
          {t('settings.product.print.settings')}
          <span className="text-xs font-normal text-gray-400 ml-auto">{t('common.optional')}</span>
        </h3>

        <div className="space-y-4">
          {/* Kitchen Printing Group */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold text-gray-500 uppercase tracking-wider flex items-center gap-2">
              {t('settings.product.print.kitchenPrinting')}
            </h4>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <FormField label={t('settings.product.print.isKitchenPrintEnabled')}>
                <div className="relative">
                  <select
                    value={
                      formData.isKitchenPrintEnabled === undefined || formData.isKitchenPrintEnabled === null || formData.isKitchenPrintEnabled === -1
                        ? '-1'
                        : String(formData.isKitchenPrintEnabled)
                    }
                    onChange={(e) => {
                      const raw = e.target.value;
                      const num = parseInt(raw, 10);
                      const next = isNaN(num) ? -1 : (num === 1 ? 1 : num === 0 ? 0 : -1);
                      onFieldChange('isKitchenPrintEnabled', next as any);
                    }}
                    className={selectClass}
                  >
                    <option value="-1">{t('common.default')}</option>
                    <option value="1">{t('common.enabled')}</option>
                    <option value="0">{t('common.disabled')}</option>
                  </select>
                  {(formData.isKitchenPrintEnabled === undefined || formData.isKitchenPrintEnabled === null || formData.isKitchenPrintEnabled === -1) && (
                    <div className="mt-1.5 text-xs text-gray-500 flex items-center gap-1.5">
                      <div className="w-1.5 h-1.5 rounded-full bg-blue-400"></div>
                      <span>
                        {t('settings.product.print.effectiveState')}: {
                          (() => {
                            if (!isGlobalKitchenEnabled) return (t('common.disabledGlobal'));
                            const cat = categories.find(c => String(c.id) === String(formData.categoryId));
                            const isEnabled = cat ? (cat.is_kitchen_print_enabled !== false) : true;
                            return isEnabled ? (t('common.enabled')) : (t('common.disabled'));
                          })()
                        }
                      </span>
                    </div>
                  )}
                </div>
              </FormField>

              <KitchenPrinterSelector
                value={formData.kitchenPrinterId}
                onChange={(value) => onFieldChange('kitchenPrinterId', value)}
                t={t}
              />

              <div className="col-span-1 md:col-span-2">
                <FormField label={t('settings.product.print.kitchenPrintName')}>
                  <input
                    value={formData.kitchenPrintName || ''}
                    onChange={(e) => onFieldChange('kitchenPrintName', e.target.value)}
                    placeholder={t('settings.product.print.kitchenPrintNamePlaceholder')}
                    className={inputClass}
                  />
                </FormField>
              </div>
            </div>
          </div>

          <div className="border-t border-gray-100"></div>

          {/* Label Printing Group */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold text-gray-500 uppercase tracking-wider flex items-center gap-2">
              {t('settings.product.print.labelPrinting')}
            </h4>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <FormField label={t('settings.product.print.isLabelPrintEnabled')}>
                <div className="relative">
                  <select
                    value={
                      formData.isLabelPrintEnabled === undefined || formData.isLabelPrintEnabled === null || formData.isLabelPrintEnabled === -1
                        ? '-1'
                        : String(formData.isLabelPrintEnabled)
                    }
                    onChange={(e) => {
                      const raw = e.target.value;
                      const num = parseInt(raw, 10);
                      const next = isNaN(num) ? -1 : (num === 1 ? 1 : num === 0 ? 0 : -1);
                      onFieldChange('isLabelPrintEnabled', next as any);
                    }}
                    className={selectClass}
                  >
                    <option value="-1">{t('common.default')}</option>
                    <option value="1">{t('common.enabled')}</option>
                    <option value="0">{t('common.disabled')}</option>
                  </select>
                  {(formData.isLabelPrintEnabled === undefined || formData.isLabelPrintEnabled === null || formData.isLabelPrintEnabled === -1) && (
                    <div className="mt-1.5 text-xs text-gray-500 flex items-center gap-1.5">
                      <div className="w-1.5 h-1.5 rounded-full bg-blue-400"></div>
                      <span>
                        {t('settings.product.print.effectiveState')}: {
                          (() => {
                            if (!isGlobalLabelEnabled) return (t('common.disabledGlobal'));
                            const cat = categories.find(c => String(c.id) === String(formData.categoryId));
                            const isEnabled = cat ? (cat.is_label_print_enabled !== false) : true;
                            return isEnabled ? (t('common.enabled')) : (t('common.disabled'));
                          })()
                        }
                      </span>
                    </div>
                  )}
                </div>
              </FormField>
            </div>
          </div>

          <div className="border-t border-gray-100"></div>

          {/* Receipt Printing Group */}
          <div className="space-y-3">
            <h4 className="text-xs font-bold text-gray-500 uppercase tracking-wider flex items-center gap-2">
              {t('settings.product.print.receiptPrinting')}
            </h4>
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="col-span-1 md:col-span-2">
                <FormField label={t('settings.product.print.receiptName')}>
                  <div className="relative">
                    <div className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400">
                      <FileText size={14} />
                    </div>
                    <input
                      value={formData.receiptName || ''}
                      onChange={(e) => onFieldChange('receiptName', e.target.value)}
                      placeholder={t('settings.product.print.receiptNamePlaceholder')}
                      className={`${inputClass} pl-9`}
                    />
                  </div>
                </FormField>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Block 3: Extended Info (Optional) */}
      <section className="bg-white rounded-xl border border-gray-100 p-4 space-y-4 shadow-sm">
        <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900 pb-2 border-b border-gray-100">
          <MoreHorizontal size={16} className="text-blue-500" />
          {t('settings.form.extendedInfo')}
          <span className="text-xs font-normal text-gray-400 ml-auto">{t('common.optional')}</span>
        </h3>

        <div className="grid grid-cols-1 md:grid-cols-2 gap-4 items-start">
          {/* Image Upload - Compact Row Style */}
          <div className="col-span-1 md:col-span-2">
            <label className="block text-sm font-medium text-gray-700 mb-1">
              {t('settings.product.form.image')}
            </label>
            <div className="flex items-center gap-4 p-3 bg-gray-50 rounded-xl border border-dashed border-gray-200">
              <div
                className="w-16 h-16 shrink-0 bg-white rounded-lg border border-gray-200 flex items-center justify-center overflow-hidden cursor-pointer hover:border-orange-300 transition-colors"
                onClick={onSelectImage}
              >
                {formData.image ? (
                  <img
                    src={
                      /^https?:\/\//.test(formData.image)
                        ? formData.image
                        : convertFileSrc(formData.image)
                    }
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
                    {formData.image ? (t('common.change')) : (t('common.uploadImage'))}
                  </button>
                  {formData.image && (
                    <button
                      type="button"
                      onClick={() => onFieldChange('image', '')}
                      className="px-3 py-1.5 text-xs font-medium text-red-600 bg-red-50 border border-red-100 rounded-lg hover:bg-red-100 transition-colors"
                    >
                      {t('common.remove')}
                    </button>
                  )}
                </div>
                <p className="mt-1 text-xs text-gray-400 truncate">
                  {t('settings.product.form.imageHint')}
                </p>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Block 4: Product Specifications (Optional) - Only show in EDIT mode */}
      {formData.id && (
      <section className="bg-white rounded-xl border border-gray-100 p-4 space-y-4 shadow-sm">
        <div className="flex items-center justify-between pb-2 border-b border-gray-100">
          <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900">
            <PackagePlus size={16} className="text-teal-500" />
            {t('settings.product.specification.title')}
          </h3>
          <button
            type="button"
            onClick={() => setShowSpecModal(true)}
            className="px-3 py-1.5 text-xs font-medium text-gray-700 bg-white border border-gray-200 rounded-lg hover:bg-gray-50 hover:text-gray-900 transition-all shadow-sm"
          >
            {t('settings.product.specification.manage')}
          </button>
        </div>

        <div className={`rounded-xl border ${formData.hasMultiSpec && previewSpecs.length > 0 ? 'border-gray-100 bg-white' : 'border-dashed border-gray-200 bg-gray-50/50'} min-h-[80px] p-4 transition-all`}>
          {formData.hasMultiSpec ? (
            previewSpecs.length > 0 ? (
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-2">
                {previewSpecs.map(spec => (
                  <div key={spec.id} className="flex justify-between items-center bg-white p-3 rounded-lg border border-gray-100 shadow-sm hover:shadow-md transition-shadow group">
                    <div className="flex items-center gap-2 min-w-0">
                      <div className="w-1.5 h-1.5 rounded-full bg-teal-400 shrink-0"></div>
                      <span className="text-sm font-medium text-gray-900 truncate group-hover:text-teal-700 transition-colors">
                        {spec.is_root && !spec.name ? t('settings.product.specification.label.default') : spec.name}
                      </span>
                      {spec.is_default && (
                        <span className="shrink-0 px-1.5 py-0.5 text-[10px] font-bold text-teal-600 bg-teal-50 rounded border border-teal-100">
                          {t('common.default')}
                        </span>
                      )}
                    </div>
                    <span className="text-sm font-mono font-medium text-gray-600 shrink-0">
                      {formatCurrency(spec.price)}
                    </span>
                  </div>
                ))}
              </div>
            ) : (
              <div className="flex flex-col items-center justify-center py-2 text-gray-400 gap-2">
                <PackagePlus size={20} className="text-gray-300" />
                <span className="text-sm">{t('settings.product.specification.noConfigured')}</span>
              </div>
            )
          ) : (
            <div className="flex flex-col items-center justify-center py-2 text-gray-400 gap-2">
              <div className="flex items-center gap-2">
                <div className="w-2 h-2 rounded-full bg-gray-300"></div>
                <span className="text-sm font-medium text-gray-500">{t('settings.product.specification.disabled')}</span>
              </div>
              <span className="text-xs text-gray-400">
                {t('settings.product.specification.hint')}
              </span>
            </div>
          )}
        </div>
      </section>
      )}

      <SpecificationManagementModal
        isOpen={showSpecModal}
        onClose={() => setShowSpecModal(false)}
        productId={formData.id || null}
        basePrice={formData.price}
        baseExternalId={formData.externalId}
        initialSpecifications={formData.tempSpecifications}
        hasMultiSpec={formData.hasMultiSpec}
        onEnableMultiSpec={(enabled) => {
          onFieldChange('hasMultiSpec', enabled);
          // If editing an existing product, update the backend immediately
          // to match SpecificationManager's behavior (which saves specs immediately)
          if (formData.id) {
            invoke('update_product', {
              params: {
                id: formData.id,
                hasMultiSpec: enabled
              }
            }).catch(e => console.error('Failed to update hasMultiSpec:', e));
          }
        }}
        onSpecificationsChange={(specs) => {
          // Store temp specifications in formData (only relevant for new products or local preview)
          if (!formData.id && specs) {
            onFieldChange('tempSpecifications', specs);
          }
        }}
        t={t}
      />

      {/* Block 5: Attributes (Optional) */}
      <section className="bg-white rounded-xl border border-gray-100 p-4 space-y-4 shadow-sm">
        <div className="flex items-center justify-between pb-2 border-b border-gray-100">
          <h3 className="flex items-center gap-2 text-sm font-bold text-gray-900">
            <Layers size={16} className="text-teal-500" />
            {t('settings.product.attribute.title')}
          </h3>
          <button
            type="button"
            onClick={() => setShowAttributeModal(true)}
            className="px-3 py-1.5 text-xs font-medium text-gray-700 bg-white border border-gray-200 rounded-lg hover:bg-gray-50 hover:text-gray-900 transition-all shadow-sm"
          >
            {t('settings.product.attribute.manage')}
          </button>
        </div>

        <div className={`rounded-xl border ${selectedAttributes.length > 0 ? 'border-gray-100 bg-white' : 'border-dashed border-gray-200 bg-gray-50/50'} min-h-[80px] p-4 transition-all`}>
          {selectedAttributes.length > 0 ? (
            <div className="flex flex-wrap gap-2">
              {selectedAttributes.map((attr) => {
                const rawDefaults = formData.attributeDefaultOptions?.[attr.id];
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
            <div className="flex flex-col items-center justify-center py-2 text-gray-400 gap-2">
              <Layers size={20} className="text-gray-300" />
              <p className="text-sm">{t('settings.product.attribute.noSelected')}</p>
            </div>
          )}
        </div>
      </section>

      <AttributeSelectionModal
        isOpen={showAttributeModal}
        onClose={() => setShowAttributeModal(false)}
        selectedAttributeIds={formData.selectedAttributeIds || []}
        attributeDefaultOptions={formData.attributeDefaultOptions || {}}
        onChange={(ids) => onFieldChange('selectedAttributeIds', ids)}
        onDefaultOptionChange={(attrId, optionIds) => {
          const newDefaults = { ...formData.attributeDefaultOptions, [attrId]: optionIds };
          if (!optionIds || optionIds.length === 0) delete newDefaults[attrId];
          onFieldChange('attributeDefaultOptions', newDefaults);
        }}
        t={t}
        inheritedAttributeIds={inheritedAttributeIds}
      />
    </div>
  );
};
