import React, { useState, useMemo, useEffect } from 'react';
import { Package, DollarSign, Printer, Tag, Plus, Trash2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { Wizard, WizardStep } from '@/shared/components/Wizard';
import { FormField, CheckboxField, inputClass, selectClass } from '@/shared/components/FormField';
import { ImageUpload } from '@/shared/components/ImageUpload';
import { TagPicker } from '@/shared/components/TagPicker/TagPicker';
import type { StoreCategory, StoreTag, ProductCreate, ProductSpecInput } from '@/core/types/store';

interface ProductWizardProps {
  categories: StoreCategory[];
  tags: StoreTag[];
  initialCategoryId?: number;
  onFinish: (data: ProductCreate) => void;
  onCancel: () => void;
  isSubmitting?: boolean;
}

interface FormSpec {
  name: string;
  price: number;
  receipt_name: string;
  is_default: boolean;
  is_active: boolean;
}

export const ProductWizard: React.FC<ProductWizardProps> = ({
  categories,
  tags,
  initialCategoryId,
  onFinish,
  onCancel,
  isSubmitting,
}) => {
  const { t } = useI18n();

  // ── Form State ──
  const [name, setName] = useState('');
  const [categoryId, setCategoryId] = useState<number | ''>(initialCategoryId || '');
  const [image, setImage] = useState('');
  
  const [specs, setSpecs] = useState<FormSpec[]>([
    { name: 'Standard', price: 0, receipt_name: '', is_default: true, is_active: true }
  ]);

  const [taxRate, setTaxRate] = useState<number>(0);
  const [sortOrder, setSortOrder] = useState<number>(0);
  const [receiptName, setReceiptName] = useState('');
  const [kitchenPrintName, setKitchenPrintName] = useState('');
  const [isKitchenPrint, setIsKitchenPrint] = useState(false);
  const [isLabelPrint, setIsLabelPrint] = useState(false);
  
  const [externalId, setExternalId] = useState('');
  const [tagIds, setTagIds] = useState<number[]>([]);

  // ── Helpers ──
  const addSpec = () => {
    setSpecs([...specs, { name: '', price: 0, receipt_name: '', is_default: false, is_active: true }]);
  };

  const removeSpec = (index: number) => {
    setSpecs(specs.filter((_, i) => i !== index));
  };

  const updateSpec = (index: number, field: keyof FormSpec, value: string | number | boolean) => {
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

  const handleFinish = () => {
    const specInputs: ProductSpecInput[] = specs.map((s, i) => ({
      name: s.name.trim() || 'Standard',
      price: s.price,
      display_order: i,
      is_default: s.is_default,
      is_active: s.is_active,
      is_root: specs.length === 1,
      receipt_name: s.receipt_name.trim() || undefined,
    }));

    onFinish({
      name: name.trim(),
      image: image || undefined,
      category_id: Number(categoryId),
      tax_rate: taxRate,
      sort_order: sortOrder,
      receipt_name: receiptName.trim() || name.trim(),
      kitchen_print_name: kitchenPrintName.trim() || name.trim(),
      is_kitchen_print_enabled: isKitchenPrint ? 1 : 0,
      is_label_print_enabled: isLabelPrint ? 1 : 0,
      external_id: externalId ? Number(externalId) : undefined,
      tags: tagIds.length > 0 ? tagIds : undefined,
      specs: specInputs,
    });
  };

  // ── Steps ──

  const step1: WizardStep = {
    id: 'basics',
    title: t('settings.product.step_basics'),
    description: t('settings.product.step_basics_desc'),
    isValid: !!name.trim() && categoryId !== '',
    component: (
      <div className="space-y-6">
        <div className="flex justify-center">
           <ImageUpload value={image} onChange={setImage} />
        </div>
        
        <FormField label={t('settings.product.name')} required>
          <input value={name} onChange={e => setName(e.target.value)} className={inputClass} autoFocus placeholder="e.g. Cheese Burger" />
        </FormField>
        
        <FormField label={t('settings.product.category')} required>
          <select 
            value={categoryId} 
            onChange={e => setCategoryId(Number(e.target.value))} 
            className={selectClass}
          >
            <option value="" disabled>{t('common.hint.select_option')}</option>
            {categories.map(c => (
              <option key={c.source_id} value={c.source_id}>{c.name}</option>
            ))}
          </select>
          {initialCategoryId && categoryId === initialCategoryId && (
             <p className="text-xs text-primary-600 mt-1.5 flex items-center gap-1">
               <Package className="w-3 h-3" />
               {t('settings.product.last_selected_category')}
             </p>
          )}
        </FormField>
      </div>
    ),
  };

  const step2: WizardStep = {
    id: 'pricing',
    title: t('settings.product.step_pricing'),
    description: t('settings.product.step_pricing_desc'),
    isValid: specs.length > 0 && specs.every(s => s.price >= 0),
    component: (
      <div className="space-y-4">
        {specs.map((spec, idx) => (
          <div key={idx} className="p-4 bg-slate-50 rounded-xl border border-slate-200 relative group animate-in slide-in-from-bottom-2 duration-300">
            {specs.length > 1 && (
              <button 
                onClick={() => removeSpec(idx)}
                className="absolute top-2 right-2 p-1.5 text-slate-400 hover:text-red-500 rounded-lg hover:bg-red-50 opacity-0 group-hover:opacity-100 transition-all"
              >
                <Trash2 className="w-4 h-4" />
              </button>
            )}
            
            <div className="grid grid-cols-12 gap-3 items-end">
              <div className="col-span-7">
                <FormField label={t('settings.product.spec_name')}>
                  <input 
                    value={spec.name} 
                    onChange={e => updateSpec(idx, 'name', e.target.value)} 
                    className={inputClass} 
                    placeholder="Standard" 
                  />
                </FormField>
              </div>
              <div className="col-span-5">
                <FormField label={t('settings.product.price')} required>
                  <div className="relative">
                     <input 
                      type="number" 
                      value={spec.price} 
                      onChange={e => updateSpec(idx, 'price', Number(e.target.value))} 
                      className={`${inputClass} pr-8`} 
                      min={0} 
                      step="0.01" 
                    />
                     <div className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 font-medium">$</div>
                  </div>
                </FormField>
              </div>
            </div>
            
            {specs.length > 1 && (
               <div className="mt-3 flex items-center gap-4">
                 <label className="flex items-center gap-2 text-xs text-slate-600 cursor-pointer">
                   <input 
                     type="radio" 
                     checked={spec.is_default} 
                     onChange={() => updateSpec(idx, 'is_default', true)} 
                     className="text-primary-600 focus:ring-primary-500"
                   />
                   {t('settings.product.is_default')}
                 </label>
               </div>
            )}
          </div>
        ))}
        
        <button 
          onClick={addSpec}
          className="w-full py-3 border-2 border-dashed border-slate-200 rounded-xl text-slate-500 font-medium hover:border-primary-300 hover:text-primary-600 hover:bg-primary-50 transition-all flex items-center justify-center gap-2"
        >
          <Plus className="w-5 h-5" />
          {t('settings.product.add_variant')}
        </button>
      </div>
    ),
  };

  const step3: WizardStep = {
    id: 'operations',
    title: t('settings.product.step_operations'),
    description: t('settings.product.step_operations_desc'),
    isValid: true,
    component: (
      <div className="space-y-6">
        <div className="grid grid-cols-2 gap-4">
          <FormField label={t('settings.product.tax_rate')}>
            <div className="relative">
              <input 
                type="number" 
                value={taxRate} 
                onChange={e => setTaxRate(Number(e.target.value))} 
                className={`${inputClass} pr-8`} 
              />
              <div className="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400">%</div>
            </div>
          </FormField>
          <FormField label={t('settings.product.sort_order')}>
            <input 
              type="number" 
              value={sortOrder} 
              onChange={e => setSortOrder(Number(e.target.value))} 
              className={inputClass} 
            />
          </FormField>
        </div>
        
        <FormField label={t('settings.product.receipt_name')}>
          <input 
            value={receiptName} 
            onChange={e => setReceiptName(e.target.value)} 
            className={inputClass} 
            placeholder={name || 'Receipt Name'} 
          />
        </FormField>
        
        <div className="p-4 bg-slate-50 rounded-xl border border-slate-200 space-y-4">
           <h3 className="text-sm font-semibold text-slate-900 flex items-center gap-2">
             <Printer className="w-4 h-4" />
             {t('settings.product.printing')}
           </h3>
           <div className="space-y-3">
             <CheckboxField 
               id="is_kitchen" 
               label={t('settings.product.is_kitchen_print')} 
               checked={isKitchenPrint} 
               onChange={setIsKitchenPrint} 
             />
             {isKitchenPrint && (
               <div className="pl-7 animate-in slide-in-from-top-1 fade-in">
                 <input 
                   value={kitchenPrintName} 
                   onChange={e => setKitchenPrintName(e.target.value)} 
                   className={inputClass} 
                   placeholder={t('settings.product.kitchen_print_name')} 
                 />
               </div>
             )}
             <CheckboxField 
               id="is_label" 
               label={t('settings.product.is_label_print')} 
               checked={isLabelPrint} 
               onChange={setIsLabelPrint} 
             />
           </div>
        </div>
      </div>
    ),
  };

  const step4: WizardStep = {
    id: 'org',
    title: t('settings.product.step_org'),
    description: t('settings.product.step_org_desc'),
    isValid: true,
    component: (
      <div className="space-y-6">
        <div className="space-y-2">
          <label className="text-sm font-medium text-slate-700 flex items-center gap-2">
            <Tag className="w-4 h-4 text-slate-400" />
            {t('settings.product.tags')}
          </label>
          <TagPicker 
            tags={tags} 
            selectedIds={tagIds} 
            onToggle={toggleTag} 
          />
        </div>
        
        <FormField label={t('settings.product.external_id')}>
          <input 
            type="number" 
            value={externalId} 
            onChange={e => setExternalId(e.target.value)} 
            className={inputClass} 
            placeholder="e.g. POS ID" 
          />
        </FormField>
      </div>
    ),
  };

  const steps = [step1, step2, step3, step4];

  return (
    <Wizard
      steps={steps}
      onFinish={handleFinish}
      onCancel={onCancel}
      isSubmitting={isSubmitting}
      title={t('settings.product.create_title')}
      finishLabel={t('common.action.create')}
    />
  );
};
