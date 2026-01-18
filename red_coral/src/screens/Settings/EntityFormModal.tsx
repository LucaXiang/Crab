import React, { useEffect, useState, useRef } from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useSettingsModal,
  useSettingsFormMeta,
  useSettingsZones,
  useSettingsCategories,
  useSettingsActions,
  useSettingsProducts,
  useSettingsStore,
} from '@/core/stores/settings/useSettingsStore';
import { createClient } from '@/infrastructure/api';

const api = createClient();
import { toast } from '@/presentation/components/Toast';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { syncAttributeBindings } from './utils';
import { useProductActions, useProductStore } from '@/core/stores/product/useProductStore';
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
  const { zones } = useSettingsZones();
  const { categories, setCategories } = useSettingsCategories();
  const { setLastSelectedCategory, refreshData } = useSettingsActions();
  const { updateProductInList, removeProductFromList } = useSettingsProducts();
  const { clearProductCache, addProduct } = useProductActions();
  const productStore = useProductStore;
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
      api.listCategories()
        .then(resp => {
          const cats = resp.data?.categories || [];
          setCategories(cats);
          // Auto-select first category if none selected and creating a new product
          if (cats.length > 0 && modal.action === 'CREATE' && !formData.categoryId && !defaultCategorySet.current) {
            setFormData({ categoryId: cats[0].id });
            defaultCategorySet.current = true;
          }
        })
        .catch(console.error);
    }
  }, [modal.open, modal.entity, modal.action]);

  // Load product attributes when editing a product
  useEffect(() => {
    if (modal.open && modal.entity === 'PRODUCT' && modal.action === 'EDIT' && modal.data?.id) {
      const loadProductAttributes = async () => {
        try {
          const resp = await api.fetchProductAttributes(String(modal.data.id));
          const attributeIds = resp.attributes.map(attr => attr.id);

          // Identify inherited attributes
          const bindings = resp.bindings ?? [];
          const inherited = bindings
            .filter(b => b.id.startsWith('global-') || b.id.startsWith('cat-'))
            .map(b => b.attributeId);
          setInheritedAttributeIds(inherited);

          // Load default options
          const defaultOptions: Record<string, string[]> = {};
          bindings.forEach(binding => {
            const defaults = binding.defaultOptionIds || [];
            if (defaults.length > 0) {
              defaultOptions[binding.attributeId] = defaults;
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
          const resp = await api.listCategoryAttributes(modal.data.id ? Number(modal.data.id) : undefined);
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
        await api.deleteTable(Number(data.id));
        clearZoneTableCache(); // Invalidate tables cache
        toast.success(t('settings.table.action.deleted'));
      } else if (entity === 'ZONE') {
        try {
          await api.deleteZone(Number(data.id));
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
        await api.deleteProduct(Number(data.id));
        // Optimistic update: remove from both Settings and ProductStore
        removeProductFromList(data.id);
        // Also remove from ProductStore cache
        const { products } = productStore.getState();
        const updatedProducts = products.filter(p => p.id !== data.id);
        productStore.setState({ products: updatedProducts });
        toast.success(t('settings.product.action.deleted'));
      } else if (entity === 'CATEGORY') {
        try {
          await api.deleteCategory(Number(data.id));
          clearProductCache(); // Invalidate products + categories cache
          // Refresh categories list
          const resp = await api.listCategories();
          setCategories(resp.data?.categories || []);
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
          await api.createTable({ name: tableData.name, zone_id: Number(tableData.zoneId), capacity: Number(tableData.capacity) });
          clearZoneTableCache(); // Invalidate tables cache
          toast.success(t("settings.table.message.created"));
        } else {
          await api.updateTable(Number(data.id), { name: tableData.name, capacity: Number(tableData.capacity) });
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
          await api.updateZone(Number(data.id), { name: zoneData.name });
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
        const category = categories.find(c => c.id === productData.categoryId);
        if (category) {
          setLastSelectedCategory(category.name);
        }

        let productId: string;
        if (action === 'CREATE') {
          // Send price as float (backend will convert to cents for storage)
          const created = await api.createProduct({
            name: productData.name,
            category_id: productData.categoryId,
            price: productData.price,
            image: productData.image,
            has_multi_spec: false,
            tax_rate: productData.taxRate || 0,
            sort_order: productData.sortOrder || 0,
            receipt_name: productData.receiptName,
            kitchen_print_name: productData.kitchenPrintName,
            kitchen_printer_id: productData.kitchenPrinterId,
            is_kitchen_print_enabled: productData.isKitchenPrintEnabled,
            is_label_print_enabled: productData.isLabelPrintEnabled,
            external_id: productData.externalId,
          });
          productId = created.id;
          // Optimistic update: add to ProductStore cache
          addProduct(created);
          toast.success(t("settings.product.message.created"));
        } else {
          // Send price as float (backend will convert to cents for storage)
          await api.updateProduct(Number(data.id), {
            name: productData.name,
            category_id: productData.categoryId,
            price: productData.price,
            image: productData.image,
            has_multi_spec: false,
            tax_rate: productData.taxRate || 0,
            sort_order: productData.sortOrder || 0,
            receipt_name: productData.receiptName,
            kitchen_print_name: productData.kitchenPrintName,
            kitchen_printer_id: productData.kitchenPrinterId,
            is_kitchen_print_enabled: productData.isKitchenPrintEnabled,
            is_label_print_enabled: productData.isLabelPrintEnabled,
            external_id: productData.externalId,
          });
          productId = data.id;
          // Optimistic update: directly update both Settings and ProductStore
          updateProductInList(data.id, {
            name: productData.name,
            price: productData.price,
            categoryId: productData.categoryId,
            image: productData.image,
            externalId: productData.externalId,
            receiptName: productData.receiptName,
            sortOrder: productData.sortOrder,
            taxRate: productData.taxRate,
            kitchenPrinterId: productData.kitchenPrinterId,
            kitchenPrintName: productData.kitchenPrintName,
            isKitchenPrintEnabled: productData.isKitchenPrintEnabled,
            isLabelPrintEnabled: productData.isLabelPrintEnabled,
          });
          // Also update ProductStore cache
          const { products } = productStore.getState();
          const updatedProducts = products.map(p => p.id === data.id ? { id: data.id, ...productData } : p);
          productStore.setState({ products: updatedProducts });
          toast.success(t("settings.product.message.updated"));
        }

        // Handle attribute bindings
        const selectedAttributeIds = formData.selectedAttributeIds || [];

        // Get existing bindings (only for EDIT mode)
        let existingBindings: any[] = [];
        if (action === 'EDIT') {
          try {
            const resp = await api.fetchProductAttributes(productId);
            // existingAttributeIds unused, removed
            existingBindings = resp.bindings ?? [];
          } catch (error) {
            console.error('Failed to fetch existing attributes:', error);
          }
        }

        // Handle attribute bindings using helper
        await syncAttributeBindings(
          selectedAttributeIds,
          formData.attributeDefaultOptions || {},
          existingBindings,
          async (attrId) => api.unbindProductAttribute(Number(productId), Number(attrId)),
          async (attrId, defaultOptionIds, index) => {
            await api.bindProductAttribute({
              productId,
              attributeId: attrId,
              isRequired: false,
              displayOrder: index,
              defaultOptionIds
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

        if (action === 'CREATE') {
          await api.createCategory({
            name: categoryName,
            sort_order: formData.sortOrder ?? 0,
            kitchen_printer_id: formData.kitchenPrinterId ?? undefined,
            is_kitchen_print_enabled: isKitchenPrintEnabled,
            is_label_print_enabled: isLabelPrintEnabled
          });
          clearProductCache(); // Invalidate products + categories cache
          refreshData(); // Trigger UI refresh
          toast.success(t('settings.category.action.createSuccess'));
        } else {
          await api.updateCategory(Number(data.id), {
            name: categoryName,
            sort_order: formData.sortOrder ?? 0,
            kitchen_printer_id: formData.kitchenPrinterId ?? undefined,
            is_kitchen_print_enabled: isKitchenPrintEnabled,
            is_label_print_enabled: isLabelPrintEnabled
          });
          clearProductCache(); // Invalidate products + categories cache
          refreshData(); // Trigger UI refresh
          toast.success(t('settings.category.action.updateSuccess'));
        }

        // Handle attribute bindings
        const selectedAttributeIds = formData.selectedAttributeIds || [];

        // Get existing bindings (only for EDIT mode)
        let existingBindings: any[] = [];
        if (action === 'EDIT') {
          try {
            const resp = await api.listCategoryAttributes(Number(categoryName));
            const catAttrs = resp.data?.category_attributes || [];
            // Transform to expected format for syncAttributeBindings
            existingBindings = catAttrs.map((ca: any) => ({
              attributeId: ca.attribute_id,
              defaultOptionId: ca.default_option_id ? String(ca.default_option_id) : undefined
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
          async (attrId) => api.unbindCategoryAttribute(Number(categoryName), Number(attrId)),
          async (attrId, defaultOptionIds, index) => {
            await api.bindCategoryAttribute({
              categoryId: categoryName,
              attributeId: attrId,
              isRequired: false,
              displayOrder: index,
              defaultOptionIds
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
            zones={zones}
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
