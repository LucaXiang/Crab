import React, { useState, useMemo, useEffect } from 'react';
import {
  Plus,
  Trash2,
  Check,
  ShoppingBag,
  Star,
  Save,
  Lock
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { createTauriClient } from '@/infrastructure/api';
import { ProductSpecification } from '@/core/domain/types';
import { toast } from '@/presentation/components/Toast';
import { ConfirmDialog } from '@/presentation/components/ui/ConfirmDialog';
import { formatCurrency } from '@/utils/formatCurrency';
import { usePriceInput } from '@/hooks/usePriceInput';
import { useI18n } from '@/hooks/useI18n';

// API client for specs operations
const api = createTauriClient();

interface SpecificationManagerProps {
  productId: string | null; // null = new product (temp mode)
  onSpecificationsChange?: (specs?: ProductSpecification[]) => void;
  initialSpecifications?: ProductSpecification[];
  basePrice?: number; // Product base price
  baseExternalId?: number; // Product external_id
  t: (key: string, options?: any) => string;
}

interface SpecForm {
  name: string;
  price: string;
  external_id: string;
  is_default: boolean;
}

const NEW_SPEC_ID = 'NEW_SPEC';

export const SpecificationManager: React.FC<SpecificationManagerProps> = ({
  productId,
  onSpecificationsChange,
  initialSpecifications = [],
  basePrice = 0,
  baseExternalId,
}) => {
  const { t } = useI18n();
  
  const [specifications, setSpecifications] = useState<ProductSpecification[]>(initialSpecifications);
  const [loading, setLoading] = useState(false);
  const [hasLoaded, setHasLoaded] = useState(false);

  // Selection State
  const [selectedId, setSelectedId] = useState<string>(NEW_SPEC_ID);

  const [formData, setFormData] = useState<SpecForm>({
    name: '',
    price: '0.00',
    external_id: '',
    is_default: false,
  });

  const {
    priceInput: priceInputValue,
    setPriceInput,
    handlePriceChange,
    commitPrice,
    handlePriceKeyDown
  } = usePriceInput(parseFloat(formData.price) || 0, {
    minValue: 0,
    onCommit: (val) => setFormData((prev) => ({ ...prev, price: val.toFixed(2) }))
  });

  const [confirmDialog, setConfirmDialog] = useState({
    isOpen: false,
    title: '',
    description: '',
    onConfirm: () => { },
  });

  // Derived state for the currently selected spec (if any)
  const selectedSpec = useMemo(() =>
    specifications.find(s => s.id === selectedId) || null,
    [specifications, selectedId]
  );

  // Load specifications
  const loadSpecifications = async () => {
    if (!productId) {
      setSpecifications(initialSpecifications);
      setHasLoaded(true);
      return;
    }

    setLoading(true);
    try {
      const response = await api.listProductSpecs(productId!);
      const specs = response.data?.specs || [];
      setSpecifications(specs);
    } catch (error) {
      console.error('Failed to load specifications:', error);
      toast.error(t('settings.specification.message.loadFailed'));
    } finally {
      setLoading(false);
      setHasLoaded(true);
    }
  };

  // Initial Load
  useEffect(() => {
    loadSpecifications();
  }, [productId]);

  // Sync Root Spec with Base Product Info
  useEffect(() => {
    setSpecifications(prev => {
      const rootIndex = prev.findIndex(s => s.is_root);
      if (rootIndex === -1) return prev;

      const root = prev[rootIndex];
      const currentExternalId = root.external_id ?? null;
      const newExternalId = baseExternalId ?? null;

      if (Math.abs(root.price - basePrice) < 0.001 && currentExternalId === newExternalId) {
        return prev;
      }

      const newSpecs = [...prev];
      newSpecs[rootIndex] = {
        ...root,
        price: basePrice,
        external_id: baseExternalId
      };

      // Notify parent if needed (avoid loop if parent updates basePrice based on specs)
      // Actually we shouldn't notify parent here to avoid cycles if parent is source of truth for basePrice

      return newSpecs;
    });
  }, [basePrice, baseExternalId]);

  // Sync form when selection changes or when data updates
  useEffect(() => {
    if (selectedId === NEW_SPEC_ID) {
      setFormData({
        name: '',
        price: basePrice ? basePrice.toFixed(2) : '0.00',
        external_id: baseExternalId?.toString() || '',
        is_default: specifications.length === 0, // Auto default if first
      });
      setPriceInput(basePrice ? basePrice.toFixed(2) : '0.00');
    } else if (selectedSpec) {
      setFormData({
        name: selectedSpec.name,
        price: selectedSpec.price.toFixed(2),
        external_id: selectedSpec.external_id?.toString() || '',
        is_default: selectedSpec.is_default,
      });
      setPriceInput(selectedSpec.price.toFixed(2));
    }
  }, [selectedId, selectedSpec, basePrice, baseExternalId]);

  // Actions
  const handleSelectSpec = (id: string) => {
    setSelectedId(id);
  };

  const handleSave = async () => {
    if (!formData.name.trim()) {
      toast.error(t("settings.specification.form.nameRequired"));
      return;
    }

    const finalPrice = parseFloat(formData.price) || 0;
    const finalExternalId = formData.external_id ? parseInt(formData.external_id) : undefined;

    try {
      if (selectedId !== NEW_SPEC_ID && selectedSpec) {
        // Update
        if (productId) {
          await api.updateProductSpec(productId!, selectedSpec.id!, {
            name: formData.name.trim(),
            price: finalPrice,
            external_id: finalExternalId ?? null,
            is_default: formData.is_default,
          });
          toast.success(t("settings.specification.message.updated"));
          await loadSpecifications();
        } else {
          // Local update
          const updatedSpecs = specifications.map(spec => {
            if (spec.id === selectedSpec.id) {
              return {
                ...spec,
                name: formData.name.trim(),
                price: finalPrice,
                external_id: finalExternalId ?? null,
                is_default: formData.is_default,
              };
            }
            // If we are setting this to default, unset others
            if (formData.is_default && spec.is_default) {
              return { ...spec, is_default: false };
            }
            return spec;
          });
          setSpecifications(updatedSpecs);
          onSpecificationsChange?.(updatedSpecs);
        }
      } else {
        // Create
        if (productId) {
          await api.createProductSpec(productId!, {
            name: formData.name.trim(),
            price: finalPrice,
            external_id: finalExternalId?.toString(),
            is_default: formData.is_default,
            is_root: specifications.length === 0, // First one is technically root
            display_order: specifications.length,
          });
          toast.success(t("settings.specification.message.created"));
          await loadSpecifications();
          setSelectedId(NEW_SPEC_ID);
        } else {
          // Local Create
          const newSpec: ProductSpecification = {
            id: `temp-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
            product: productId || '',
            name: formData.name.trim(),
            price: finalPrice,
            external_id: finalExternalId ?? null,
            display_order: specifications.length,
            is_default: formData.is_default,
            is_root: specifications.length === 0,
            is_active: true,
            tags: [],
            created_at: null,
            updated_at: null,
          };

          let updatedSpecs = [...specifications, newSpec];
          // If new one is default, unset others
          if (formData.is_default) {
            updatedSpecs = updatedSpecs.map(s => s.id === newSpec.id ? s : { ...s, is_default: false });
          }

          setSpecifications(updatedSpecs);
          onSpecificationsChange?.(updatedSpecs);
          setSelectedId(newSpec.id);
        }
      }
      onSpecificationsChange?.();
    } catch (error: any) {
      console.error('Failed to save specification:', error);
      toast.error(error || t('settings.saveFailed'));
    }
  };

  const handleDelete = async (id: string) => {
    const spec = specifications.find(s => s.id === id);
    if (spec?.is_root) {
      toast.error(t('settings.specification.message.rootCannotDelete'));
      return;
    }

    setConfirmDialog({
      isOpen: true,
      title: t('settings.specification.action.delete'),
      description: t('settings.specification.confirm.delete'),
      onConfirm: async () => {
        try {
          if (productId) {
            await api.deleteProductSpec(productId!, id);
            toast.success(t("settings.specification.message.deleted"));
            await loadSpecifications();
            if (selectedId === id) setSelectedId(NEW_SPEC_ID);
          } else {
            const updatedSpecs = specifications.filter(spec => spec.id !== id);
            setSpecifications(updatedSpecs);
            onSpecificationsChange?.(updatedSpecs);
            if (selectedId === id) setSelectedId(NEW_SPEC_ID);
          }
          onSpecificationsChange?.();
        } catch (error: any) {
          console.error('Failed to delete specification:', error);
          toast.error(error || t('settings.deleteFailed'));
        }
        setConfirmDialog((prev) => ({ ...prev, isOpen: false }));
      },
    });
  };

  const toggleDefault = async (spec: ProductSpecification) => {
    const newDefaultState = !spec.is_default;

    try {
      if (productId) {
        await api.updateProductSpec(productId!, spec.id!, {
          name: spec.name,
          price: spec.price,
          is_default: newDefaultState,
        });
        loadSpecifications();
      } else {
        const updatedSpecs = specifications.map(s => {
          if (s.id === spec.id) return { ...s, is_default: newDefaultState };
          if (newDefaultState && s.is_default) return { ...s, is_default: false };
          return s;
        });
        setSpecifications(updatedSpecs);
        onSpecificationsChange?.(updatedSpecs);
      }
    } catch (error) {
      console.error('Failed to toggle default:', error);
      toast.error(t('settings.saveFailed'));
    }
  };

  if (loading && !hasLoaded) {
    return <div className="p-4 text-center text-gray-500">{t('settings.specification.loading')}</div>;
  }

  return (
    <div className="flex flex-row h-full bg-white">
      {/* List Panel */}
      <div className="w-80 border-r border-gray-100 bg-gray-50/50 flex flex-col h-full">
        <div className="p-4 border-b border-gray-100 bg-white/50 backdrop-blur-sm flex justify-between items-center sticky top-0 z-10">
          <h3 className="text-sm font-bold text-gray-900 flex items-center gap-2">
            <ShoppingBag size={16} className="text-blue-500" />
            {t('settings.specification.list')}
          </h3>
          <button
            onClick={() => handleSelectSpec(NEW_SPEC_ID)}
            className={`p-2 rounded-lg transition-all ${selectedId === NEW_SPEC_ID 
              ? 'bg-blue-600 text-white shadow-md' 
              : 'bg-white text-gray-500 border border-gray-200 hover:text-blue-600 hover:border-blue-300'}`}
          >
            <Plus size={18} />
          </button>
        </div>

        <div className="flex-1 overflow-y-auto p-3 space-y-2">
          {specifications.map((spec) => (
            <div
              key={spec.id}
              onClick={() => handleSelectSpec(spec.id)}
              className={`relative p-3 rounded-xl border transition-all cursor-pointer group ${
                selectedId === spec.id
                  ? 'bg-white border-blue-500 shadow-md ring-1 ring-blue-500/10'
                  : 'bg-white border-gray-200 hover:border-blue-300'
              }`}
            >
              <div className="flex justify-between items-start mb-1">
                <span className={`font-bold text-sm ${selectedId === spec.id ? 'text-blue-700' : 'text-gray-900'}`}>
                  {spec.is_root && !spec.name ? t('settings.product.specification.label.default') : spec.name}
                  {spec.is_root && spec.name && <span className="ml-1 text-xs font-normal text-gray-400">({t('settings.specification.label.root')})</span>}
                </span>
                {spec.is_default && (
                  <span className="px-1.5 py-0.5 text-[10px] font-bold text-amber-600 bg-amber-50 rounded border border-amber-100 flex items-center gap-1">
                    <Star size={8} fill="currentColor" />
                    {t('settings.specification.label.default')}
                  </span>
                )}
              </div>
              <div className="flex items-center justify-between mt-2">
                <span className="font-mono text-sm text-gray-600 font-medium">
                  {formatCurrency(spec.price)}
                </span>
                <div className="flex items-center gap-1">
                  <button
                    onClick={(e) => {
                      e.stopPropagation();
                      toggleDefault(spec);
                    }}
                    className={`p-1.5 rounded-md transition-colors ${spec.is_default ? 'text-amber-500 hover:bg-amber-50' : 'text-gray-300 hover:text-amber-500 hover:bg-gray-50'}`}
                    title={spec.is_default ? t('settings.specification.label.cancelDefault') : t('settings.specification.label.setDefault')}
                  >
                    <Star size={14} fill={spec.is_default ? "currentColor" : "none"} />
                  </button>
                </div>
              </div>
            </div>
          ))}
          
          {specifications.length === 0 && (
            <div className="text-center py-10 text-gray-400">
              <ShoppingBag size={24} className="mx-auto mb-2 opacity-50" />
              <p className="text-xs">{t('settings.specification.noSpecs')}</p>
              <button
                onClick={() => handleSelectSpec(NEW_SPEC_ID)}
                className="mt-2 text-xs text-blue-600 hover:underline"
              >
                {t('settings.specification.createFirst')}
              </button>
            </div>
          )}
        </div>
      </div>

      {/* Form Panel */}
      <div className="flex-1 flex flex-col h-full bg-white relative">
        <div className="p-6 border-b border-gray-50 flex justify-between items-center">
          <div>
            <h2 className="text-lg font-bold text-gray-900 flex items-center gap-2">
              {selectedId === NEW_SPEC_ID ? <Plus size={20} className="text-blue-500" /> : <ShoppingBag size={20} className="text-orange-500" />}
              {selectedId === NEW_SPEC_ID ? t('settings.specification.addNew') : t('settings.specification.edit')}
            </h2>
          </div>
          {selectedId !== NEW_SPEC_ID && !selectedSpec?.is_root && (
            <button
              onClick={() => handleDelete(selectedId)}
              className="flex items-center gap-1.5 px-3 py-1.5 text-sm font-medium text-red-600 bg-red-50 hover:bg-red-100 rounded-lg transition-colors"
            >
              <Trash2 size={16} />
              {t('settings.specification.action.delete')}
            </button>
          )}
        </div>

        <div className="flex-1 overflow-y-auto p-8">
          <div className="max-w-xl mx-auto space-y-6">
            
            {/* Name Field */}
            <div className="space-y-2">
              <label className="text-sm font-semibold text-gray-700">
                {t('settings.specification.form.name')} <span className="text-red-500">*</span>
              </label>
              <input
                type="text"
                value={formData.name}
                onChange={(e) => setFormData(prev => ({ ...prev, name: e.target.value }))}
                className="w-full px-4 py-2.5 text-sm border border-gray-200 rounded-xl focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
                placeholder={t('settings.specification.form.namePlaceholder')}
                autoFocus={selectedId === NEW_SPEC_ID}
              />
            </div>

            {/* Price Field */}
            <div className="grid grid-cols-2 gap-6">
              <div className="space-y-2">
                <label className="text-sm font-semibold text-gray-700 flex items-center gap-1">
                  {t('settings.specification.form.price')} <span className="text-red-500">*</span>
                  {selectedSpec?.is_root && <Lock size={12} className="text-gray-400" />}
                </label>
                <div className="relative">
                  <span className={`absolute left-3 top-1/2 -translate-y-1/2 text-sm ${selectedSpec?.is_root ? 'text-gray-400' : 'text-gray-500'}`}>$</span>
                  <input
                    type="text"
                    inputMode="decimal"
                    value={priceInputValue}
                    onChange={handlePriceChange}
                    onBlur={() => commitPrice()}
                    onKeyDown={handlePriceKeyDown}
                    disabled={selectedSpec?.is_root}
                    className={`w-full pl-7 pr-4 py-2.5 text-sm font-mono border rounded-xl focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 ${
                      selectedSpec?.is_root
                        ? 'bg-gray-50 text-gray-500 border-gray-200 cursor-not-allowed'
                        : 'border-gray-200'
                    }`}
                  />
                </div>
                {selectedSpec?.is_root && (
                  <p className="text-xs text-gray-400 mt-1">
                    {t('settings.specification.form.baseSpecPriceHint')}
                  </p>
                )}
              </div>

              <div className="space-y-2">
                <label className="text-sm font-semibold text-gray-700 flex items-center gap-1">
                  {t('settings.specification.form.externalId')}
                  {selectedSpec?.is_root && <Lock size={12} className="text-gray-400" />}
                </label>
                <input
                  type="number"
                  value={formData.external_id}
                  onChange={(e) => setFormData(prev => ({ ...prev, external_id: e.target.value }))}
                  disabled={selectedSpec?.is_root}
                  className={`w-full px-4 py-2.5 text-sm border rounded-xl focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500 ${
                    selectedSpec?.is_root
                      ? 'bg-gray-50 text-gray-500 border-gray-200 cursor-not-allowed'
                      : 'border-gray-200'
                  }`}
                  placeholder={t('settings.specification.form.externalIdPlaceholder')}
                />
              </div>
            </div>

            {/* Default Switch */}
            <div className="pt-4 border-t border-gray-100">
              <label className="flex items-center gap-3 p-3 border border-gray-200 rounded-xl cursor-pointer hover:bg-gray-50 transition-colors">
                <div className={`w-5 h-5 rounded border flex items-center justify-center ${formData.is_default ? 'bg-blue-600 border-blue-600' : 'border-gray-300 bg-white'}`}>
                  {formData.is_default && <Check size={14} className="text-white" />}
                </div>
                <input
                  type="checkbox"
                  className="hidden"
                  checked={formData.is_default}
                  onChange={(e) => setFormData(prev => ({ ...prev, is_default: e.target.checked }))}
                />
                <div className="flex-1">
                  <div className="text-sm font-bold text-gray-900">{t('settings.specification.form.setDefault')}</div>
                  <div className="text-xs text-gray-500">{t('settings.specification.form.setDefaultHint')}</div>
                </div>
                {formData.is_default && <Star size={16} className="text-amber-500 fill-current" />}
              </label>
            </div>

            {/* Save Button */}
            <div className="pt-6">
              <button
                onClick={handleSave}
                className="w-full flex items-center justify-center gap-2 px-6 py-3 bg-blue-600 text-white font-bold rounded-xl hover:bg-blue-700 active:scale-95 transition-all shadow-lg shadow-blue-500/30"
              >
                <Save size={18} />
                {selectedId === NEW_SPEC_ID ? t('settings.specification.action.save') : t('settings.specification.action.update')}
              </button>
            </div>

          </div>
        </div>
      </div>

      <ConfirmDialog
        isOpen={confirmDialog.isOpen}
        title={confirmDialog.title}
        description={confirmDialog.description}
        onConfirm={confirmDialog.onConfirm}
        onCancel={() => setConfirmDialog((prev) => ({ ...prev, isOpen: false }))}
      />
    </div>
  );
};
