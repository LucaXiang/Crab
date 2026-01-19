import React, { useEffect, useState, useRef } from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useSettingsModal,
  useSettingsFormMeta,
  useSettingsStore,
} from '@/core/stores/settings/useSettingsStore';
import { createApiClient } from '@/infrastructure/api';
import { useProductStore, useZones, useCategoryStore } from '@/core/stores/resources';

const api = createApiClient();
import { toast } from '@/presentation/components/Toast';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { syncAttributeBindings } from './utils';
import { useZoneTableStore } from '@/hooks/useZonesAndTables';

// Form Components
import {
  TableForm,
  ZoneForm,
  ProductForm,
  CategoryForm,
  DeleteConfirmation
} from './forms';

export const EntityFormModal: React.FC = React.memo(() => {
  const { t } = useI18n();
  const { modal, closeModal } = useSettingsModal();
  const { formData, setFormField, setFormData, setAsyncFormData, isFormDirty, formErrors } = useSettingsFormMeta();

  // Data from resources stores
  const zones = useZones();
  const categoryStore = useCategoryStore();
  const categories = categoryStore.items;

  // UI actions from settings store
  const setLastSelectedCategory = useSettingsStore((s) => s.setLastSelectedCategory);
  const refreshData = useSettingsStore((s) => s.refreshData);

  const clearZoneTableCache = useZoneTableStore((state) => state.clearCache);
  const [unsavedDialogOpen, setUnsavedDialogOpen] = useState(false);
  const [inheritedAttributeIds, setInheritedAttributeIds] = useState<string[]>([]);
  const defaultCategorySet = useRef(false);

  useEffect(() => {
    if (modal.open && modal.entity === 'PRODUCT') {
      console.log('[EntityFormModal] open PRODUCT modal', {
        action: modal.action,
        id: modal.data?.id,
        formIsKitchenPrintEnabled: formData.isKitchenPrintEnabled,
      });
    }
    if (!modal.open) {
      setInheritedAttributeIds([]);
    }
  }, [modal.open, modal.entity, modal.action, modal.data, formData.isKitchenPrintEnabled]);

  // Ensure categories are loaded when opening product form
  useEffect(() => {
    // Reset flag when modal opens or action changes
    defaultCategorySet.current = false;
  }, [modal.open, modal.entity, modal.action]);

  useEffect(() => {
    if (modal.open && modal.entity === 'PRODUCT') {
      // Ensure categories are loaded
      if (!categoryStore.isLoaded) {
        categoryStore.fetchAll();
      }
      // Auto-select first category if none selected and creating a new product
      if (categories.length > 0 && modal.action === 'CREATE' && !formData.categoryId && !defaultCategorySet.current) {
        setFormData({ categoryId: categories[0].id as unknown as number });
        defaultCategorySet.current = true;
      }
    }
  }, [modal.open, modal.entity, modal.action, categories.length, categoryStore.isLoaded]);

  // Load product attributes when editing a product
  useEffect(() => {
    if (modal.open && modal.entity === 'PRODUCT' && modal.action === 'EDIT' && modal.data?.id) {
      const loadProductAttributes = async () => {
        try {
          const resp = await api.fetchProductAttributes(String(modal.data.id));
          const productAttributes = resp.data?.product_attributes ?? [];
          const attributeIds = productAttributes.map((attr: { out: string }) => attr.out);

          // Identify inherited attributes (from global or category bindings)
          // Note: ProductAttribute.id format may indicate source
          const inherited: string[] = [];
          setInheritedAttributeIds(inherited);

          // Load default options
          const defaultOptions: Record<string, string[]> = {};
          productAttributes.forEach((binding: { out: string; default_option_idx?: number | null }) => {
            if (binding.default_option_idx !== null && binding.default_option_idx !== undefined) {
              defaultOptions[binding.out] = [String(binding.default_option_idx)];
            }
          });

          setAsyncFormData({
            selectedAttributeIds: attributeIds,
            attributeDefaultOptions: defaultOptions
          });
        } catch (error) {
          console.error('Failed to load product attributes:', error);
        }
      };
      loadProductAttributes();
    }
  }, [modal.open, modal.entity, modal.action, modal.data?.id]);

  // Load category attributes when editing a category
  useEffect(() => {
    if (modal.open && modal.entity === 'CATEGORY' && modal.action === 'EDIT' && modal.data?.id) {
      const loadCategoryAttributes = async () => {
        try {
          const resp = await api.listCategoryAttributes(modal.data.id ? String(modal.data.id) : undefined);
          const catAttrs = resp.data?.category_attributes || [];
          const attributeIds = catAttrs.map((ca: any) => ca.attribute_id);

          // Load default options from category attributes
          const defaultOptions: Record<string, string[]> = {};
          catAttrs.forEach((ca: any) => {
            const defaults = ca.default_option_id ? [String(ca.default_option_id)] : [];
            if (defaults.length > 0) {
              defaultOptions[ca.attribute_id] = defaults;
            }
          });

          setAsyncFormData({
            selectedAttributeIds: attributeIds,
            attributeDefaultOptions: defaultOptions
          });
        } catch (error) {
          console.error('Failed to load category attributes:', error);
        }
      };
      loadCategoryAttributes();
    }
  }, [modal.open, modal.entity, modal.action, modal.data?.name]);

  if (!modal.open) return null;

  const { action, entity, data } = modal;

  const getTitle = () => {
    const titles: Record<string, Record<string, string>> = {
      CREATE: {
        TABLE: t('settings.table.action.add'),
        ZONE: t('settings.zone.action.add'),
        PRODUCT: t('settings.product.action.add'),
        CATEGORY: t('settings.category.action.add')
      },
      EDIT: {
        TABLE: t('settings.table.action.edit'),
        ZONE: t('settings.zone.action.edit'),
        PRODUCT: t('settings.product.action.edit'),
        CATEGORY: t('settings.category.action.edit')
      },
      DELETE: {
        TABLE: t('settings.table.action.delete'),
        ZONE: t('settings.zone.action.delete'),
        PRODUCT: t('settings.product.action.delete'),
        CATEGORY: t('settings.category.action.delete')
      }
    };
    return titles[action]?.[entity] || '';
  };

  const getAccentColor = () => {
    const colors: Record<string, string> = {
      TABLE: 'blue',
      ZONE: 'purple',
      PRODUCT: 'orange',
      CATEGORY: 'teal'
    };
    return colors[entity] || 'blue';
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
    try {
      if (entity === 'TABLE') {
        await api.deleteTable(String(data.id));
        clearZoneTableCache(); // Invalidate tables cache
        toast.success(t('settings.table.action.deleted'));
      } else if (entity === 'ZONE') {
        try {
          await api.deleteZone(String(data.id));
          clearZoneTableCache(); // Invalidate zones cache
          toast.success(t('settings.zone.action.deleted'));
        } catch (e: any) {
          const msg = String(e);
          if (msg.includes('ZONE_HAS_TABLES')) {
            toast.error(t('settings.zone.deleteBlocked'));
            return;
          }
          toast.error(t('settings.zone.deleteFailed'));
          return;
        }
      } else if (entity === 'PRODUCT') {
        await api.deleteProduct(String(data.id));
        // Optimistic update: remove from ProductStore
        useProductStore.getState().optimisticRemove(data.id);
        toast.success(t('settings.product.action.deleted'));
      } else if (entity === 'CATEGORY') {
        try {
          await api.deleteCategory(String(data.id));
          // Refresh products and categories from resources stores
          useProductStore.getState().fetchAll();
          useCategoryStore.getState().fetchAll();
          toast.success(t('settings.category.action.deleted'));
        } catch (e: any) {
          const msg = String(e);
          if (msg.includes('CATEGORY_HAS_PRODUCTS')) {
            toast.error(t('settings.category.deleteBlocked'));
            return;
          }
          toast.error(t('settings.category.deleteFailed'));
          return;
        }
      }
      closeModal();
    } catch {
      // Error already handled
    }
  };

  const handleSave = async () => {
    const hasError = Object.values(formErrors).some(Boolean);
    if (hasError) {
      toast.error(t('common.invalidForm'));
      return;
    }
    try {
      if (entity === 'TABLE') {
        const tableData = {
          name: formData.name.trim(),
          zoneId: formData.zoneId,
          capacity: Math.max(1, formData.capacity),
        };
        if (action === 'CREATE') {
          await api.createTable({ name: tableData.name, zone: String(tableData.zoneId), capacity: Number(tableData.capacity) });
          clearZoneTableCache(); // Invalidate tables cache
          toast.success(t("settings.table.message.created"));
        } else {
          await api.updateTable(String(data.id), { name: tableData.name, capacity: Number(tableData.capacity) });
          clearZoneTableCache(); // Invalidate tables cache
          toast.success(t("settings.table.message.updated"));
        }
      } else if (entity === 'ZONE') {
        const zoneData = {
          name: formData.name.trim(),
          surchargeType: formData.surchargeType !== 'none' ? formData.surchargeType : undefined,
          surchargeAmount: formData.surchargeType !== 'none' ? formData.surchargeAmount : undefined,
        };
        if (action === 'CREATE') {
          await api.createZone({ name: zoneData.name });
          clearZoneTableCache(); // Invalidate zones cache
          toast.success(t("settings.zone.message.created"));
        } else {
          await api.updateZone(String(data.id), { name: zoneData.name });
          clearZoneTableCache(); // Invalidate zones cache
          toast.success(t("settings.zone.message.updated"));
        }
      } else if (entity === 'PRODUCT') {
        if (!formData.name?.trim()) {
          toast.error(t('settings.product.form.nameRequired'));
          return;
        }
        if (formData.externalId === undefined) {
          toast.error(t('settings.externalIdRequired'));
          return;
        }
        if (!formData.categoryId) {
          if (categories.length === 0) {
            toast.error(t('settings.category.createFirst'));
          } else {
            toast.error(t('settings.category.required'));
          }
          return;
        }

          const productData = {
            name: formData.name.trim(),
            receiptName: formData.receiptName?.trim() ?? undefined,
            price: Math.max(0.01, formData.price),
            categoryId: formData.categoryId,
            image: formData.image?.trim() ?? '',
            externalId: formData.externalId,
            taxRate: formData.taxRate,
            sortOrder: formData.sortOrder ?? undefined,
            kitchenPrinterId: formData.kitchenPrinterId ?? undefined,
            kitchenPrintName: formData.kitchenPrintName?.trim() ?? undefined,
            isKitchenPrintEnabled: formData.isKitchenPrintEnabled ?? undefined,
            isLabelPrintEnabled: formData.isLabelPrintEnabled ?? undefined,
          };

        // Save the selected category for next time (lookup name by ID)
        const category = categories.find(c => String(c.id) === String(productData.categoryId));
        if (category) {
          setLastSelectedCategory(category.name);
        }

        let productId: string;
        if (action === 'CREATE') {
          // Send price as float (backend will convert to cents for storage)
          const resp = await api.createProduct({
            name: productData.name,
            category: productData.categoryId,
            price: productData.price,
            image: productData.image,
            has_multi_spec: false,
            tax_rate: productData.taxRate || 0,
            sort_order: productData.sortOrder || 0,
            receipt_name: productData.receiptName,
            kitchen_print_name: productData.kitchenPrintName,
            kitchen_printer: productData.kitchenPrinterId,
            is_kitchen_print_enabled: productData.isKitchenPrintEnabled,
            is_label_print_enabled: productData.isLabelPrintEnabled,
          });
          const created = resp.data?.product;
          productId = created?.id || '';
          // Optimistic update: add to resources ProductStore
          if (created) {
            useProductStore.getState().optimisticAdd(created as any);
          }
          toast.success(t("settings.product.message.created"));
        } else {
          // Send price as float (backend will convert to cents for storage)
          await api.updateProduct(String(data.id), {
            name: productData.name,
            category: productData.categoryId,
            price: productData.price,
            image: productData.image,
            has_multi_spec: false,
            tax_rate: productData.taxRate || 0,
            sort_order: productData.sortOrder || 0,
            receipt_name: productData.receiptName,
            kitchen_print_name: productData.kitchenPrintName,
            kitchen_printer: productData.kitchenPrinterId,
            is_kitchen_print_enabled: productData.isKitchenPrintEnabled,
            is_label_print_enabled: productData.isLabelPrintEnabled,
          });
          productId = data.id;
          // Optimistic update: update ProductStore cache
          useProductStore.getState().optimisticUpdate(data.id, (p) => ({ ...p, ...productData as any }));
          toast.success(t("settings.product.message.updated"));
        }

        // Handle attribute bindings
        const selectedAttributeIds = formData.selectedAttributeIds || [];

        // Get existing bindings (only for EDIT mode)
        let existingBindings: any[] = [];
        if (action === 'EDIT') {
          try {
            const resp = await api.fetchProductAttributes(productId);
            const productAttrs = resp.data?.product_attributes ?? [];
            // Transform to expected format
            existingBindings = productAttrs.map((pa: any) => ({
              attributeId: pa.out,
              id: pa.id
            }));
          } catch (error) {
            console.error('Failed to fetch existing attributes:', error);
          }
        }

        // Handle attribute bindings using helper
        await syncAttributeBindings(
          selectedAttributeIds,
          formData.attributeDefaultOptions || {},
          existingBindings,
          async (attrId) => api.unbindProductAttribute(String(attrId)),
          async (attrId, defaultOptionIds, index) => {
            await api.bindProductAttribute({
              product_id: productId,
              attribute_id: attrId,
              is_required: false,
              display_order: index,
              default_option_id: defaultOptionIds?.[0]
            });
          }
        );

        // Handle specifications (for CREATE mode with temp specifications)
        if (action === 'CREATE' && formData.hasMultiSpec && formData.tempSpecifications) {
          try {
            // First enable multi-spec for the product
            await invoke('toggle_product_multi_spec', {
              productId,
              enabled: true,
            });

            // Then create all specifications
            for (const spec of formData.tempSpecifications) {
              await invoke('create_product_specification', {
                params: {
                  productId,
                  name: spec.name,
                  receiptName: spec.receiptName || null,
                  price: spec.price,
                },
              });
            }

            // Set default specification if any
            const defaultSpec = formData.tempSpecifications?.find((s: { isDefault?: boolean }) => s.isDefault);
            if (defaultSpec) {
              await invoke('update_product_specification', {
                id: defaultSpec.id,
                params: { isDefault: true },
              });
            }
          } catch (error) {
            console.error('Failed to create specifications:', error);
            toast.error(t('settings.specification.message.partialCreateFailed'));
          }
        }
      } else if (entity === 'CATEGORY') {
        const categoryName = formData.name.trim();
        // Backend expects boolean for Categories (true=Enabled, false=Disabled)
        // Default to Enabled (true) if undefined/null
        const isKitchenPrintEnabled = (formData.isKitchenPrintEnabled as unknown) !== false;
        const isLabelPrintEnabled = (formData.isLabelPrintEnabled as unknown) !== false;

        let categoryId: string;
        if (action === 'CREATE') {
          const resp = await api.createCategory({
            name: categoryName,
            sort_order: formData.sortOrder ?? 0,
            kitchen_printer: formData.kitchenPrinterId ? String(formData.kitchenPrinterId) : undefined,
            is_kitchen_print_enabled: isKitchenPrintEnabled,
            is_label_print_enabled: isLabelPrintEnabled
          });
          categoryId = resp.data?.category?.id || '';
          // Trigger refresh of products store
          useProductStore.getState().fetchAll();
          refreshData(); // Trigger UI refresh
          toast.success(t('settings.category.action.createSuccess'));
        } else {
          categoryId = String(data.id);
          await api.updateCategory(categoryId, {
            name: categoryName,
            sort_order: formData.sortOrder ?? 0,
            kitchen_printer: formData.kitchenPrinterId ? String(formData.kitchenPrinterId) : undefined,
            is_kitchen_print_enabled: isKitchenPrintEnabled,
            is_label_print_enabled: isLabelPrintEnabled
          });
          // Trigger refresh of products store
          useProductStore.getState().fetchAll();
          refreshData(); // Trigger UI refresh
          toast.success(t('settings.category.action.updateSuccess'));
        }

        // Handle attribute bindings
        const selectedAttributeIds = formData.selectedAttributeIds || [];

        // Get existing bindings (only for EDIT mode)
        let existingBindings: any[] = [];
        if (action === 'EDIT') {
          try {
            const resp = await api.listCategoryAttributes(categoryId);
            const catAttrs = resp.data?.category_attributes || [];
            // Transform to expected format for syncAttributeBindings
            existingBindings = catAttrs.map((ca: any) => ({
              attributeId: ca.out,
              id: ca.id
            }));
          } catch (error) {
            console.error('Failed to fetch existing category attributes:', error);
          }
        }

        // Handle attribute bindings using helper
        await syncAttributeBindings(
          selectedAttributeIds,
          formData.attributeDefaultOptions || {},
          existingBindings,
          async (attrId) => api.unbindCategoryAttribute(categoryId, String(attrId)),
          async (attrId, defaultOptionIds, index) => {
            await api.bindCategoryAttribute({
              category_id: categoryId,
              attribute_id: attrId,
              is_required: false,
              display_order: index,
              default_option_id: defaultOptionIds?.[0] ? Number(defaultOptionIds[0]) : undefined
            });
          }
        );
      }
      // Refresh strategy:
      // - CREATE operations: no refresh needed (optimistic update)
      // - PRODUCT UPDATE: already updated optimistically, no refresh needed
      // - Other UPDATE operations: no refresh needed (optimistic update)
      // All operations now use optimistic updates
      closeModal();
    } catch (e: any) {
      const msg = String(e);
      if (msg.includes('CATEGORY_EXISTS')) {
        toast.error(t("settings.category.message.exists"));
        return;
      }
      if (msg.includes('EXTERNAL_ID_EXISTS')) {
         toast.error(t('settings.externalIdExists'));
         return;
      }
      toast.error(t('common.na'));
    }
  };

  const handleSelectImage = async () => {
    try {
      const file = await dialogOpen({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });
      if (!file || Array.isArray(file)) return;
      const saved = await invoke<string>('save_image', { sourcePath: file });
      setFormField('image', saved);
    } catch {
      toast.error(t('common.na'));
    }
  };

  const accent = getAccentColor();
  const hasError = Object.values(formErrors).some(Boolean);
  const isSaveDisabled = !isFormDirty || hasError;
  const saveEnabledClass = `px-5 py-2.5 bg-${accent}-600 text-white rounded-xl text-sm font-semibold hover:bg-${accent}-700 transition-colors shadow-lg shadow-${accent}-600/20`;
  const saveDisabledClass = 'px-5 py-2.5 bg-gray-200 text-gray-400 rounded-xl text-sm font-semibold cursor-not-allowed';

  // Render form content based on entity type
  const renderFormContent = () => {
    if (action === 'DELETE') {
      return <DeleteConfirmation name={data?.name} entity={entity} t={t} />;
    }

    switch (entity) {
      case 'TABLE':
        return (
          <TableForm
            formData={formData}
            zones={zones as any}
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            onFieldChange={setFormField as (field: string, value: any) => void}
            t={t}
          />
        );
      case 'ZONE':
        return (
          <ZoneForm
            formData={formData}
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            onFieldChange={setFormField as (field: string, value: any) => void}
            t={t}
          />
        );
      case 'PRODUCT':
        return (
          <ProductForm
            formData={formData}
            categories={categories}
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            onFieldChange={setFormField as (field: string, value: any) => void}
            onSelectImage={handleSelectImage}
            t={t}
            inheritedAttributeIds={inheritedAttributeIds}
          />
        );
      case 'CATEGORY':
        return (
          <CategoryForm
            formData={{
              ...formData,
              isKitchenPrintEnabled: formData.isKitchenPrintEnabled !== 0 && (formData.isKitchenPrintEnabled as unknown) !== false,
              isLabelPrintEnabled: formData.isLabelPrintEnabled !== 0 && (formData.isLabelPrintEnabled as unknown) !== false,
              kitchenPrinterId: formData.kitchenPrinterId ?? undefined,
            }}
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            onFieldChange={setFormField as (field: string, value: any) => void}
            t={t}
          />
        );
      default:
        return null;
    }
  };

  return (
    <div className="fixed inset-0 z-80 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div
        className={`bg-white rounded-2xl shadow-2xl w-full ${entity === 'PRODUCT' ? 'max-w-2xl' : 'max-w-lg'} flex flex-col max-h-[90vh] overflow-hidden animate-in zoom-in-95 duration-200`}
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
            {t('common.cancel')}
          </button>
          {action === 'DELETE' ? (
            <button
              onClick={handleDelete}
              className="px-5 py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors shadow-lg shadow-red-600/20"
            >
              {t('common.delete')}
            </button>
          ) : (
            <button
              onClick={handleSave}
              disabled={isSaveDisabled}
              className={isSaveDisabled ? saveDisabledClass : saveEnabledClass}
            >
              {action === 'CREATE' ? t('common.create') : t('common.save')}
            </button>
          )}
        </div>
      </div>

      {/* Unsaved Changes Dialog */}
      {unsavedDialogOpen && (
        <div className="fixed inset-0 z-90 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-white rounded-2xl shadow-2xl max-w-sm w-full overflow-hidden animate-in zoom-in-95">
            <div className="p-6">
              <h3 className="text-lg font-bold text-gray-900 mb-2">{t('settings.unsavedConfirm')}</h3>
              <p className="text-sm text-gray-600 mb-6">{t('settings.unsavedConfirmHint')}</p>
              <div className="grid grid-cols-2 gap-3">
                <button
                  onClick={handleCancelDiscard}
                  className="w-full py-2.5 bg-gray-100 text-gray-700 rounded-xl text-sm font-semibold hover:bg-gray-200 transition-colors"
                >
                  {t('common.cancel')}
                </button>
                <button
                  onClick={handleConfirmDiscard}
                  className="w-full py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors shadow-lg shadow-red-600/20"
                >
                  {t('common.confirm')}
                </button>
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
});
