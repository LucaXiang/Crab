import React, { useEffect, useState, useRef } from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useSettingsModal,
  useSettingsFormMeta,
  useSettingsStore,
} from '@/core/stores/settings/useSettingsStore';
import { invoke } from '@tauri-apps/api/core';
import { useCategoryStore } from '@/core/stores/resources';
import { getErrorMessage } from '@/utils/error';
import { toast } from '@/presentation/components/Toast';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
import { ProductForm } from './ProductForm';
import { createProduct, updateProduct, deleteProduct, loadProductFullData } from './mutations';
import { createTauriClient } from '@/infrastructure/api';
import { DeleteConfirmation } from '@/shared/components/DeleteConfirmation';
import { ProtectedGate } from '@/presentation/components/auth/ProtectedGate';
import { Permission } from '@/core/domain/types';

export const ProductModal: React.FC = React.memo(() => {
  const { t } = useI18n();
  const { modal, closeModal } = useSettingsModal();
  const { formData, setFormField, setFormData, setAsyncFormData, isFormDirty, formErrors } = useSettingsFormMeta();

  // Data from resources stores
  const categoryStore = useCategoryStore();
  const categories = categoryStore.items;

  // UI actions from settings store
  const setLastSelectedCategory = useSettingsStore((s) => s.setLastSelectedCategory);

  const [unsavedDialogOpen, setUnsavedDialogOpen] = useState(false);
  const [loadErrorMessage, setLoadErrorMessage] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);
  const [inheritedAttributeIds, setInheritedAttributeIds] = useState<string[]>([]);
  const defaultCategorySet = useRef(false);
  const initialCategoryRef = useRef<string | null>(null);

  // Check if this modal is for PRODUCT entity
  const isProductModal = modal.open && modal.entity === 'PRODUCT';

  useEffect(() => {
    if (!modal.open) {
      setInheritedAttributeIds([]);
    }
  }, [modal.open, modal.entity, modal.action, modal.data, formData.print_destinations]);

  // Reset flag when modal opens or action changes
  useEffect(() => {
    defaultCategorySet.current = false;
  }, [modal.open, modal.entity, modal.action]);

  // Ensure categories are loaded when opening product form
  useEffect(() => {
    if (isProductModal) {
      // Ensure categories are loaded
      if (!categoryStore.isLoaded) {
        categoryStore.fetchAll();
      }
      // Auto-select first category if none selected and creating a new product
      if (categories.length > 0 && modal.action === 'CREATE' && !formData.category && !defaultCategorySet.current) {
        setFormData({ category: categories[0].id ?? '' });
        defaultCategorySet.current = true;
      }
    }
  }, [isProductModal, modal.action, categories.length, categoryStore.isLoaded]);

  // Load full product data when editing
  // NOTE: modal.open is needed as dependency to reload data when reopening the same product
  useEffect(() => {
    const productId = modal.data?.id;
    if (isProductModal && modal.action === 'EDIT' && productId) {
      const loadData = async () => {
        try {
          const fullData = await loadProductFullData(String(productId));
          if (fullData.inherited_attribute_ids) {
            setInheritedAttributeIds(fullData.inherited_attribute_ids);
          }
          setAsyncFormData(fullData);
        } catch (e) {
          setLoadErrorMessage(getErrorMessage(e));
        }
      };
      loadData();
    }
    // Reset initial category tracking
    initialCategoryRef.current = null;
  }, [isProductModal, modal.open, modal.action, modal.data?.id]);

  // Fetch inherited attribute IDs when category changes
  // For EDIT mode, skip the first run (server-computed data is authoritative)
  useEffect(() => {
    if (!isProductModal || !formData.category) return;
    const categoryStr = String(formData.category);
    if (initialCategoryRef.current === null) {
      // First time category is set — record it, don't fetch (server data is authoritative for EDIT)
      initialCategoryRef.current = categoryStr;
      if (modal.action === 'EDIT') return; // Skip; loadProductFullData already set inherited IDs
    }
    // Fetch inherited attributes when category actually changes
    const fetchInherited = async () => {
      try {
        const api = createTauriClient();
        const catAttrs = await api.listCategoryAttributes(categoryStr);
        setInheritedAttributeIds(catAttrs.map((a) => a.id).filter(Boolean) as string[]);
      } catch {
        setInheritedAttributeIds([]);
      }
    };
    fetchInherited();
  }, [isProductModal, formData.category]);

  if (!isProductModal) return null;

  const { action, data } = modal;

  const getTitle = () => {
    const titles: Record<string, string> = {
      CREATE: t('settings.product.add_product'),
      EDIT: t('settings.product.edit_product'),
      DELETE: t('settings.product.delete_product'),
    };
    return titles[action] || '';
  };

  const handleClose = () => {
    if (isFormDirty) {
      setUnsavedDialogOpen(true);
      return;
    }
    closeModal();
  };

  const handleConfirmDiscard = () => {
    setUnsavedDialogOpen(false);
    closeModal();
  };

  const handleCancelDiscard = () => {
    setUnsavedDialogOpen(false);
  };

  const handleDelete = async () => {
    if (!data?.id) return;
    try {
      await deleteProduct(String(data.id));
      toast.success(t('settings.product.product_deleted'));
      closeModal();
    } catch (e: unknown) {
      toast.error(getErrorMessage(e));
    }
  };

  const handleSave = async () => {
    if (isSaving) return;

    const hasError = Object.values(formErrors).some(Boolean);
    if (hasError) {
      toast.error(t('common.message.invalid_form'));
      return;
    }

    // Validation
    if (!formData.name?.trim()) {
      toast.error(t('settings.product.form.name_required'));
      return;
    }

    const rootSpec = formData.specs?.find(s => s.is_root);
    const externalId = rootSpec?.external_id;
    if (externalId === undefined || externalId === null) {
      toast.error(t('settings.external_id_required'));
      return;
    }

    if (!formData.category) {
      if (categories.length === 0) {
        toast.error(t('settings.category.create_first'));
      } else {
        toast.error(t('settings.category.required'));
      }
      return;
    }

    setIsSaving(true);
    try {
      // Save the selected category for next time
      const category = categories.find(c => String(c.id) === String(formData.category));
      if (category) {
        setLastSelectedCategory(category.name);
      }

      if (action === 'CREATE') {
        await createProduct(formData, categories);
        toast.success(t('settings.product.message.created'));
      } else if (data?.id) {
        await updateProduct(String(data.id), formData);
        toast.success(t('settings.product.message.updated'));
      }

      closeModal();
    } catch (e: unknown) {
      toast.error(getErrorMessage(e));
    } finally {
      setIsSaving(false);
    }
  };

  const handleSelectImage = async () => {
    try {
      const file = await dialogOpen({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });
      if (!file || Array.isArray(file)) return;
      const hash = await invoke<string>('save_image', { sourcePath: file });
      setFormField('image', hash);
    } catch (e) {
      toast.error(getErrorMessage(e));
    }
  };

  const accent = 'orange';
  const hasError = Object.values(formErrors).some(Boolean);
  const isSaveDisabled = !isFormDirty || hasError || isSaving;
  const saveEnabledClass = `px-5 py-2.5 bg-${accent}-600 text-white rounded-xl text-sm font-semibold hover:bg-${accent}-700 transition-colors shadow-lg shadow-${accent}-600/20`;
  const saveDisabledClass = 'px-5 py-2.5 bg-gray-200 text-gray-400 rounded-xl text-sm font-semibold cursor-not-allowed';

  const renderFormContent = () => {
    if (action === 'DELETE') {
      return <DeleteConfirmation name={data?.name} entity="PRODUCT" t={t} />;
    }

    const rootSpec = formData.specs?.find(s => s.is_root);
    return (
      <ProductForm
        formData={{
          id: formData.id,
          name: formData.name,
          receipt_name: formData.receipt_name,
          price: rootSpec?.price ?? 0,
          category: formData.category,
          image: formData.image ?? '',
          externalId: rootSpec?.external_id ?? undefined,
          tax_rate: formData.tax_rate ?? 0,
          selected_attribute_ids: formData.selected_attribute_ids,
          attribute_default_options: formData.attribute_default_options,
          print_destinations: formData.print_destinations,
          kitchen_print_name: formData.kitchen_print_name,
          is_kitchen_print_enabled: formData.is_kitchen_print_enabled ?? -1,
          is_label_print_enabled: formData.is_label_print_enabled ?? -1,
          is_active: formData.is_active ?? true,
          specs: formData.specs,
          tags: formData.tags,
        }}
        categories={categories}
        onFieldChange={setFormField}
        onSelectImage={handleSelectImage}
        t={t}
        inheritedAttributeIds={inheritedAttributeIds}
      />
    );
  };

  return (
    <div className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div
        className="bg-white rounded-2xl shadow-2xl w-full max-w-2xl flex flex-col max-h-[90vh] overflow-hidden animate-in zoom-in-95 duration-200"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className={`shrink-0 px-6 py-4 border-b border-gray-100 bg-linear-to-r from-${accent}-50 to-white`}>
          <div className="flex items-center justify-between">
            <h2 className="text-lg font-bold text-gray-900">{getTitle()}</h2>
            <button
              onClick={handleClose}
              className="p-2 hover:bg-gray-100 rounded-xl transition-colors"
            >
              <X size={18} className="text-gray-500" />
            </button>
          </div>
        </div>

        {/* Content */}
        <div className="p-6 overflow-y-auto min-h-0 flex-1">
          <div className="space-y-4">
            {renderFormContent()}
          </div>
        </div>

        {/* Footer */}
        <div className="shrink-0 px-6 py-4 border-t border-gray-100 bg-gray-50/50 flex justify-end gap-3">
          <button
            onClick={handleClose}
            className="px-5 py-2.5 bg-white border border-gray-200 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-50 transition-colors"
          >
            {t('common.action.cancel')}
          </button>
          {action === 'DELETE' ? (
            <ProtectedGate permission={Permission.PRODUCTS_DELETE}>
              <button
                onClick={handleDelete}
                className="px-5 py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors shadow-lg shadow-red-600/20"
              >
                {t('common.action.delete')}
              </button>
            </ProtectedGate>
          ) : (
            <button
              onClick={handleSave}
              disabled={isSaveDisabled}
              className={isSaveDisabled ? saveDisabledClass : saveEnabledClass}
            >
              {action === 'CREATE' ? t('common.action.create') : t('common.action.save')}
            </button>
          )}
        </div>
      </div>

      {/* Load Error Dialog */}
      {loadErrorMessage && (
        <div className="fixed inset-0 z-90 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl max-w-sm w-full overflow-hidden animate-in zoom-in-95">
            <div className="p-6">
              <h3 className="text-lg font-bold text-gray-900 mb-2">{t('common.message.load_failed')}</h3>
              <p className="text-sm text-gray-600 mb-6">{loadErrorMessage}</p>
              <button
                onClick={() => { setLoadErrorMessage(null); closeModal(); }}
                className="w-full py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors"
              >
                {t('common.action.confirm')}
              </button>
            </div>
          </div>
        </div>
      )}

      {/* Unsaved Changes Dialog */}
      {unsavedDialogOpen && (
        <div className="fixed inset-0 z-90 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl max-w-sm w-full overflow-hidden animate-in zoom-in-95">
            <div className="p-6">
              <h3 className="text-lg font-bold text-gray-900 mb-2">{t('settings.unsaved_confirm')}</h3>
              <p className="text-sm text-gray-600 mb-6">{t('settings.unsaved_confirm_hint')}</p>
              <div className="grid grid-cols-2 gap-3">
                <button
                  onClick={handleCancelDiscard}
                  className="w-full py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-200 transition-colors"
                >
                  {t('common.action.cancel')}
                </button>
                <button
                  onClick={handleConfirmDiscard}
                  className="w-full py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors shadow-lg shadow-red-600/20"
                >
                  {t('common.action.confirm')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
});
