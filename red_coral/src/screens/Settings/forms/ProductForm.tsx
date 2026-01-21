import React, { useEffect, useState } from 'react';
import { Image as ImageIcon, Tag, Hash, FileText, Layers, MoreHorizontal, Printer } from 'lucide-react';
import { FormField, inputClass, selectClass } from './FormField';
import { AttributeSelectionModal } from './AttributeSelectionModal';
import { ProductImage } from '@/presentation/components/ProductImage';
import { useAttributeStore, useAttributes, useAttributeActions, useOptionActions } from '@/core/stores/resources';
import { useIsKitchenPrintEnabled, useIsLabelPrintEnabled } from '@/core/stores/ui';
import { usePriceInput } from '@/hooks/usePriceInput';
import { SelectField } from '@/presentation/components/form/FormField/SelectField';
import { KitchenPrinterSelector } from '@/presentation/components/form/FormField/KitchenPrinterSelector';
import { AttributeDisplayTag } from '@/presentation/components/form/FormField/AttributeDisplayTag';
import { Category, EmbeddedSpec } from '@/core/domain/types';

interface ProductFormProps {
  formData: {
    id?: string; // Product ID (for editing existing product)
    name: string;
    receiptName?: string;
    price: number;
    categoryId?: string | number;
    image: string;
    externalId?: number;
    taxRate: number;
    selectedAttributeIds?: string[];
    attributeDefaultOptions?: Record<string, string[]>; // Product-level default options (array for multi-select)
    kitchenPrinterId?: number | null;
    kitchenPrintName?: string;
    isKitchenPrintEnabled?: number | null;
    isLabelPrintEnabled?: number | null;
    specs?: EmbeddedSpec[]; // Embedded specifications
    selectedTagIds?: string[]; // Tag IDs loaded from getProductFull API
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

  const TAX_RATES = [
    { value: 21, label: t('settings.product.form.taxRateGeneral') },
    { value: 10, label: t('settings.product.form.taxRateReduced') },
    { value: 4, label: t('settings.product.form.taxRateSuperReduced') },
    { value: 0, label: t('settings.product.form.taxRateExempt') },
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
          <span className="text-xs font-normal text-gray-400 ml-auto">{t('common.label.required')}</span>
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
            onChange={(value) => onFieldChange('categoryId', value)}
            options={categories.map(c => ({ value: c.id ?? '', label: c.name }))}
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
          </FormField>

          <SelectField
            label={t('settings.product.form.taxRate')}
            value={formData.taxRate?.toString() || '10'}
            onChange={(value) => {
              const val = parseInt(value as string, 10);
              onFieldChange('taxRate', isNaN(val) ? 10 : val);
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
          <span className="text-xs font-normal text-gray-400 ml-auto">{t('common.label.optional')}</span>
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
                    <option value="-1">{t('common.label.default')}</option>
                    <option value="1">{t('common.status.enabled')}</option>
                    <option value="0">{t('common.status.disabled')}</option>
                  </select>
                  {(formData.isKitchenPrintEnabled === undefined || formData.isKitchenPrintEnabled === null || formData.isKitchenPrintEnabled === -1) && (
                    <div className="mt-1.5 text-xs text-gray-500 flex items-center gap-1.5">
                      <div className="w-1.5 h-1.5 rounded-full bg-blue-400"></div>
                      <span>
                        {t('settings.product.print.effectiveState')}: {
                          (() => {
                            if (!isGlobalKitchenEnabled) return (t('common.status.disabledGlobal'));
                            const cat = categories.find(c => String(c.id) === String(formData.categoryId));
                            // Kitchen printing is enabled if category has print_destinations
                            const isEnabled = cat ? (cat.print_destinations && cat.print_destinations.length > 0) : false;
                            return isEnabled ? (t('common.status.enabled')) : (t('common.status.disabled'));
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
                    <option value="-1">{t('common.label.default')}</option>
                    <option value="1">{t('common.status.enabled')}</option>
                    <option value="0">{t('common.status.disabled')}</option>
                  </select>
                  {(formData.isLabelPrintEnabled === undefined || formData.isLabelPrintEnabled === null || formData.isLabelPrintEnabled === -1) && (
                    <div className="mt-1.5 text-xs text-gray-500 flex items-center gap-1.5">
                      <div className="w-1.5 h-1.5 rounded-full bg-blue-400"></div>
                      <span>
                        {t('settings.product.print.effectiveState')}: {
                          (() => {
                            if (!isGlobalLabelEnabled) return (t('common.status.disabledGlobal'));
                            const cat = categories.find(c => String(c.id) === String(formData.categoryId));
                            const isEnabled = cat ? (cat.is_label_print_enabled !== false) : true;
                            return isEnabled ? (t('common.status.enabled')) : (t('common.status.disabled'));
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
          <span className="text-xs font-normal text-gray-400 ml-auto">{t('common.label.optional')}</span>
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
                    {formData.image ? (t('common.action.change')) : (t('common.action.uploadImage'))}
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
                  {t('settings.product.form.imageHint')}
                </p>
              </div>
            </div>
          </div>
        </div>
      </section>

      {/* Block 4: Attributes (Optional) */}
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
