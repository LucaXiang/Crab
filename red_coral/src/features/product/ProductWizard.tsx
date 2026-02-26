import React, { useState } from 'react';
import { Printer, Tag, Plus, Trash2, Image as ImageIcon } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { Wizard, WizardStep } from '@/shared/components/Wizard';
import { FormField, CheckboxField, inputClass, selectClass } from '@/shared/components/FormField';
import { ProductImage } from './ProductImage';
import type { Category, Tag as TagType, ProductCreate, ProductSpecInput } from '@/core/domain/types/api/models';

interface ProductWizardProps {
  categories: Category[];
  tags: TagType[];
  initialCategoryId?: number;
  onFinish: (data: ProductCreate) => void;
  onCancel: () => void;
  onSelectImage: () => Promise<string | null>;
  isSubmitting?: boolean;
}

interface FormSpec {
  name: string;
  price: string;
  is_default: boolean;
  is_root: boolean;
}

export const ProductWizard: React.FC<ProductWizardProps> = ({
  categories,
  tags,
  initialCategoryId,
  onFinish,
  onCancel,
  onSelectImage,
  isSubmitting,
}) => {
  const { t } = useI18n();

  // ── Form State ──
  const [name, setName] = useState('');
  const [categoryId, setCategoryId] = useState<number | ''>(initialCategoryId || '');
  const [image, setImage] = useState('');
  const [price, setPrice] = useState('');
  const [rootSpecName, setRootSpecName] = useState('');

  const [specs, setSpecs] = useState<FormSpec[]>([]);

  const [taxRate, setTaxRate] = useState('10');
  const [isKitchenPrint, setIsKitchenPrint] = useState(true);
  const [isLabelPrint, setIsLabelPrint] = useState(false);

  const [externalId, setExternalId] = useState('');
  const [tagIds, setTagIds] = useState<number[]>([]);

  // ── Spec Helpers ──
  const addSpec = () => {
    setSpecs([...specs, { name: '', price: '', is_default: false, is_root: false }]);
  };

  const removeSpec = (index: number) => {
    setSpecs(specs.filter((_, i) => i !== index));
  };

  const updateSpec = (index: number, field: keyof FormSpec, value: string | boolean) => {
    setSpecs(specs.map((s, i) => {
      if (i !== index) {
        if (field === 'is_default' && value === true) return { ...s, is_default: false };
        return s;
      }
      return { ...s, [field]: value };
    }));
  };

  const toggleTag = (id: number) => {
    setTagIds(prev => prev.includes(id) ? prev.filter(t => t !== id) : [...prev, id]);
  };

  const handleImageSelect = async () => {
    try {
      const result = await onSelectImage();
      if (result) setImage(result);
    } catch (error) {
      console.error('Failed to select image:', error);
    }
  };

  const handleFinish = () => {
    if (!name.trim() || categoryId === '' || !externalId.trim()) return;

    // Build specs: root spec + additional specs
    const allSpecs: ProductSpecInput[] = [
      {
        name: rootSpecName.trim(),
        price: Number(price) || 0,
        display_order: 0,
        is_default: specs.length === 0 || !specs.some(s => s.is_default),
        is_active: true,
        is_root: true,
      },
      ...specs.map((s, i) => ({
        name: s.name.trim(),
        price: Number(s.price) || 0,
        display_order: i + 1,
        is_default: s.is_default,
        is_active: true,
        is_root: false,
      })),
    ];

    onFinish({
      name: name.trim(),
      image: image || undefined,
      category_id: Number(categoryId),
      tax_rate: Number(taxRate) || 0,
      sort_order: 0,
      receipt_name: name.trim(),
      kitchen_print_name: name.trim(),
      is_kitchen_print_enabled: isKitchenPrint ? 1 : 0,
      is_label_print_enabled: isLabelPrint ? 1 : 0,
      external_id: externalId ? Number(externalId) : undefined,
      tags: tagIds.length > 0 ? tagIds : undefined,
      specs: allSpecs,
    });
  };

  // ── Step 1 Validation ──
  const step1Valid = !!name.trim() && categoryId !== ''
    && specs.every(s => !!s.name.trim())
    && (price === '' || Number(price) >= 0)
    && specs.every(s => s.price === '' || Number(s.price) >= 0);

  const getStep1Hint = (): string | undefined => {
    if (!name.trim()) return t('settings.product.wizard.hint_name_required');
    if (categoryId === '') return t('settings.product.wizard.hint_category_required');
    if (specs.some(s => !s.name.trim())) return t('settings.product.wizard.hint_variant_name_required');
    return undefined;
  };

  // ── Step 1: Basics ──

  const step1: WizardStep = {
    id: 'basics',
    title: t('settings.product.wizard.step_basics'),
    description: t('settings.product.wizard.step_basics_desc'),
    isValid: step1Valid,
    validationHint: getStep1Hint(),
    component: (
      <div className="space-y-4">
        {/* Image + Name + Category */}
        <div className="flex gap-4">
          <div
            className="w-28 aspect-square shrink-0 bg-white rounded-2xl border-2 border-dashed border-gray-200 flex items-center justify-center overflow-hidden cursor-pointer hover:border-primary-400 hover:bg-primary-50/50 transition-all shadow-sm self-start"
            onClick={handleImageSelect}
          >
            {image ? (
              <ProductImage src={image} alt="preview" className="w-full h-full object-cover" />
            ) : (
              <div className="flex flex-col items-center gap-1.5 text-slate-400">
                <ImageIcon size={28} strokeWidth={1.5} />
                <span className="text-[10px] font-medium">{t('common.action.upload')}</span>
              </div>
            )}
          </div>
          <div className="flex-1 space-y-3">
            <FormField label={t('settings.product.form.name')} required>
              <input value={name} onChange={e => setName(e.target.value)} className={inputClass} autoFocus placeholder={t('settings.product.wizard.name_placeholder')} />
            </FormField>
            <FormField label={t('settings.product.form.category')} required>
              <select value={categoryId} onChange={e => setCategoryId(Number(e.target.value))} className={selectClass}>
                <option value="" disabled>{t('common.hint.select')}</option>
                {categories.map(c => (
                  <option key={c.id} value={c.id}>{c.name}</option>
                ))}
              </select>
            </FormField>
          </div>
        </div>

        {/* Specs: root (always) + additional */}
        <div className="space-y-3">
          {/* Root spec — always visible, not deletable */}
          <div className="p-3 bg-amber-50/50 rounded-xl border border-amber-200">
            <div className="grid grid-cols-12 gap-3 items-end">
              <div className="col-span-7">
                <FormField label={t('settings.product.wizard.variant_name')}>
                  <input
                    value={rootSpecName}
                    onChange={e => setRootSpecName(e.target.value)}
                    className={inputClass}
                    placeholder={t('settings.product.specification.label.default')}
                  />
                </FormField>
              </div>
              <div className="col-span-5">
                <FormField label={t('settings.product.form.price')}>
                  <div className="relative">
                    <input
                      type="text"
                      inputMode="decimal"
                      value={price}
                      onChange={e => {
                        const v = e.target.value;
                        if (v === '' || /^\d*\.?\d{0,2}$/.test(v)) setPrice(v);
                      }}
                      className={`${inputClass} pr-8`}
                      placeholder="0.00"
                    />
                    <div className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 font-medium">€</div>
                  </div>
                </FormField>
              </div>
            </div>
          </div>

          {/* Additional specs */}
          {specs.map((spec, idx) => (
            <div key={idx} className="p-3 bg-slate-50 rounded-xl border border-slate-200 relative group">
              <button
                onClick={() => removeSpec(idx)}
                className="absolute top-2 right-2 p-1 text-slate-400 hover:text-red-500 rounded-lg hover:bg-red-50 opacity-0 group-hover:opacity-100 transition-all"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
              <div className="grid grid-cols-12 gap-3 items-end">
                <div className="col-span-7">
                  <FormField label={t('settings.product.wizard.variant_name')} required>
                    <input
                      value={spec.name}
                      onChange={e => updateSpec(idx, 'name', e.target.value)}
                      className={inputClass}
                      placeholder={t('settings.product.wizard.variant_name_placeholder')}
                    />
                  </FormField>
                </div>
                <div className="col-span-5">
                  <FormField label={t('settings.product.form.price')}>
                    <div className="relative">
                      <input
                        type="text"
                        inputMode="decimal"
                        value={spec.price}
                        onChange={e => {
                          const v = e.target.value;
                          if (v === '' || /^\d*\.?\d{0,2}$/.test(v)) updateSpec(idx, 'price', v);
                        }}
                        className={`${inputClass} pr-8`}
                        placeholder="0.00"
                      />
                      <div className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 font-medium">€</div>
                    </div>
                  </FormField>
                </div>
              </div>
              <div className="mt-2 flex items-center">
                <label className="flex items-center gap-2 text-xs text-slate-600 cursor-pointer">
                  <input
                    type="radio"
                    checked={spec.is_default}
                    onChange={() => updateSpec(idx, 'is_default', true)}
                    className="text-primary-600 focus:ring-primary-500"
                  />
                  {t('settings.product.wizard.is_default')}
                </label>
              </div>
            </div>
          ))}

          {/* Add spec button */}
          <button
            onClick={addSpec}
            className="w-full py-2.5 border-2 border-dashed border-slate-200 rounded-xl text-slate-500 text-sm font-medium hover:border-primary-300 hover:text-primary-600 hover:bg-primary-50 transition-all flex items-center justify-center gap-2"
          >
            <Plus className="w-4 h-4" />
            {t('settings.product.wizard.add_variant')}
          </button>
        </div>
      </div>
    ),
  };

  // ── Step 2 ──

  const userTags = tags.filter(t => !t.is_system);

  const step2Valid = !!externalId.trim();

  const getStep2Hint = (): string | undefined => {
    if (!externalId.trim()) return t('settings.product.wizard.hint_external_id_required');
    return undefined;
  };

  const step2: WizardStep = {
    id: 'more',
    title: t('settings.product.wizard.step_operations'),
    description: t('settings.product.wizard.step_operations_desc'),
    isValid: step2Valid,
    validationHint: getStep2Hint(),
    component: (
      <div className="space-y-5">
        <FormField label={t('settings.product.form.external_id')} required>
          <input
            type="text"
            inputMode="numeric"
            value={externalId}
            onChange={e => {
              const v = e.target.value;
              if (v === '' || /^\d+$/.test(v)) setExternalId(v);
            }}
            className={inputClass}
            placeholder={t('settings.product.wizard.external_id_placeholder')}
            autoFocus
          />
          <p className="mt-1 text-xs text-slate-400">{t('settings.product.wizard.external_id_hint')}</p>
        </FormField>

        <FormField label={t('settings.product.form.tax_rate')}>
          <div className="relative">
            <input
              type="text"
              inputMode="decimal"
              value={taxRate}
              onChange={e => {
                const v = e.target.value;
                if (v === '' || /^\d*\.?\d{0,2}$/.test(v)) setTaxRate(v);
              }}
              className={`${inputClass} pr-8`}
              placeholder="10"
            />
            <div className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400">%</div>
          </div>
          <p className="mt-1 text-xs text-slate-400">{t('settings.product.wizard.tax_rate_hint')}</p>
        </FormField>

        <div className="p-4 bg-slate-50 rounded-xl border border-slate-200 space-y-3">
          <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2">
            <Printer className="w-4 h-4" />
            {t('settings.product.print.settings')}
          </h3>
          <CheckboxField
            id="is_kitchen"
            label={t('settings.product.print.is_kitchen_print_enabled')}
            checked={isKitchenPrint}
            onChange={setIsKitchenPrint}
          />
          <p className="text-xs text-slate-400 ml-6 -mt-1">{t('settings.product.wizard.kitchen_print_hint')}</p>
          <CheckboxField
            id="is_label"
            label={t('settings.product.print.is_label_print_enabled')}
            checked={isLabelPrint}
            onChange={setIsLabelPrint}
          />
        </div>

        {userTags.length > 0 && (
          <div className="space-y-2">
            <label className="text-sm font-medium text-slate-700 flex items-center gap-2">
              <Tag className="w-4 h-4 text-slate-400" />
              {t('settings.product.tags.title')}
            </label>
            <p className="text-xs text-slate-400">{t('settings.product.wizard.tags_hint')}</p>
            <div className="flex flex-wrap gap-2">
              {userTags.map(tag => (
                <button
                  key={tag.id}
                  onClick={() => toggleTag(tag.id)}
                  className={`px-3 py-1.5 rounded-full text-sm font-medium transition-all ${
                    tagIds.includes(tag.id)
                      ? 'text-white shadow-md scale-105'
                      : 'bg-white text-slate-500 border border-slate-200 hover:border-primary-300'
                  }`}
                  style={{
                    backgroundColor: tagIds.includes(tag.id) ? (tag.color || '#14b8a6') : undefined,
                    borderColor: !tagIds.includes(tag.id) && tag.color ? tag.color : undefined
                  }}
                >
                  {tag.name}
                </button>
              ))}
            </div>
          </div>
        )}
      </div>
    ),
  };

  return (
    <Wizard
      steps={[step1, step2]}
      onFinish={handleFinish}
      onCancel={onCancel}
      isSubmitting={isSubmitting}
      title={t('settings.product.wizard.create_title')}
      finishLabel={t('common.action.create')}
    />
  );
};
