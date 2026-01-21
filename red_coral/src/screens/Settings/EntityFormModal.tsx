import React, { useEffect, useState, useRef } from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useSettingsModal,
  useSettingsFormMeta,
  useSettingsStore,
} from '@/core/stores/settings/useSettingsStore';
import { createTauriClient, invokeApi, ApiError } from '@/infrastructure/api';
import { invoke } from '@tauri-apps/api/core';
import { useProductStore, useZones, useCategoryStore } from '@/core/stores/resources';
import { getErrorMessage } from '@/utils/error';

const api = createTauriClient();
import { toast } from '@/presentation/components/Toast';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
import { syncAttributeBindings } from './utils';
import { useZoneTableStore } from '@/hooks/useZonesAndTables';

// Form Components
import {
  TableForm,
  ZoneForm,
  ProductForm,
  CategoryForm,
  TagForm,
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
        setFormData({ categoryId: categories[0].id ?? '' });
        defaultCategorySet.current = true;
      }
    }
  }, [modal.open, modal.entity, modal.action, categories.length, categoryStore.isLoaded]);

  // Load full product data (specs, attributes, tags) when editing a product
  useEffect(() => {
    if (modal.open && modal.entity === 'PRODUCT' && modal.action === 'EDIT' && modal.data?.id) {
      const loadProductFullData = async () => {
        try {
          const resp = await api.getProductFull(String(modal.data.id));
          const productFull = resp.data?.product;
          if (!productFull) {
            console.error('Failed to load product full data');
            return;
          }

          // Extract attribute bindings
          const attributeIds = productFull.attributes.map((binding) => binding.attribute.id).filter(Boolean) as string[];

          // Identify inherited attributes (from global or category bindings)
          const inherited: string[] = [];
          setInheritedAttributeIds(inherited);

          // Load default options from attributes (default_option_idx is on the attribute itself)
          const defaultOptions: Record<string, string[]> = {};
          productFull.attributes.forEach((binding) => {
            const attrId = binding.attribute.id;
            const defaultIdx = binding.attribute.default_option_idx;
            if (attrId && defaultIdx !== null && defaultIdx !== undefined) {
              defaultOptions[attrId] = [String(defaultIdx)];
            }
          });

          // Extract tag IDs from full tag objects
          const tagIds = productFull.tags.map((tag) => tag.id).filter(Boolean) as string[];

          // Get price and externalId from default spec (is_default=true)
          const defaultSpec = productFull.specs.find((s) => s.is_default === true) ?? productFull.specs[0];
          const price = defaultSpec?.price ?? 0;
          const externalId = defaultSpec?.external_id ?? undefined;

          setAsyncFormData({
            selectedAttributeIds: attributeIds,
            attributeDefaultOptions: defaultOptions,
            // Store specs for ProductForm to use
            loadedSpecs: productFull.specs,
            // Store tag IDs
            selectedTagIds: tagIds,
            // Load price and externalId from root spec
            price,
            externalId,
            // Determine hasMultiSpec from specs count (specs.length > 1)
            hasMultiSpec: productFull.specs.length > 1,
          });
        } catch (error) {
          console.error('Failed to load product full data:', error, JSON.stringify(error));
        }
      };
      loadProductFullData();
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
        TABLE: t('settings.table.addTable'),
        ZONE: t('settings.zone.addZone'),
        PRODUCT: t('settings.product.addProduct'),
        CATEGORY: t('settings.category.addCategory'),
        TAG: t('settings.tag.addTag')
      },
      EDIT: {
        TABLE: t('settings.table.editTable'),
        ZONE: t('settings.zone.editZone'),
        PRODUCT: t('settings.product.editProduct'),
        CATEGORY: t('settings.category.editCategory'),
        TAG: t('settings.tag.editTag')
      },
      DELETE: {
        TABLE: t('settings.table.deleteTable'),
        ZONE: t('settings.zone.deleteZone'),
        PRODUCT: t('settings.product.deleteProduct'),
        CATEGORY: t('settings.category.deleteCategory'),
        TAG: t('settings.tag.deleteTag')
      }
    };
    return titles[action]?.[entity] || '';
  };

  const getAccentColor = () => {
    const colors: Record<string, string> = {
      TABLE: 'blue',
      ZONE: 'purple',
      PRODUCT: 'orange',
      CATEGORY: 'teal',
      TAG: 'indigo'
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
        toast.success(t('settings.table.tableDeleted'));
      } else if (entity === 'ZONE') {
        try {
          await api.deleteZone(String(data.id));
          clearZoneTableCache(); // Invalidate zones cache
          toast.success(t('settings.zone.zoneDeleted'));
        } catch (e: any) {
          // Use getErrorMessage to get localized message from error_code
          toast.error(getErrorMessage(e));
          return;
        }
      } else if (entity === 'PRODUCT') {
        try {
          await api.deleteProduct(String(data.id));
          // Optimistic update: remove from ProductStore
          useProductStore.getState().optimisticRemove(data.id);
          toast.success(t('settings.product.productDeleted'));
        } catch (e: any) {
          toast.error(getErrorMessage(e));
          return;
        }
      } else if (entity === 'CATEGORY') {
        try {
          await api.deleteCategory(String(data.id));
          // Refresh products and categories from resources stores
          useProductStore.getState().fetchAll();
          useCategoryStore.getState().fetchAll();
          toast.success(t('settings.category.categoryDeleted'));
        } catch (e: any) {
          toast.error(getErrorMessage(e));
          return;
        }
      } else if (entity === 'TAG') {
        try {
          await api.deleteTag(String(data.id));
          // Trigger refresh
          refreshData();
          toast.success(t('settings.tag.tagDeleted'));
        } catch (e: any) {
          toast.error(getErrorMessage(e));
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
      toast.error(t('common.message.invalidForm'));
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
          // Create product with embedded specs (price is in specs, not on product)
          const resp = await api.createProduct({
            name: productData.name,
            category: String(productData.categoryId),
            image: productData.image,
            tax_rate: productData.taxRate || 0,
            sort_order: productData.sortOrder || 0,
            receipt_name: productData.receiptName,
            kitchen_print_name: productData.kitchenPrintName,
            print_destinations: productData.kitchenPrinterId ? [String(productData.kitchenPrinterId)] : [],
            is_label_print_enabled: productData.isLabelPrintEnabled,
            // Price is embedded in specs
            specs: [{
              name: productData.name,
              price: productData.price ?? 0,
              display_order: 0,
              is_default: true,
              is_active: true,
              external_id: productData.externalId ?? null,
            }],
          });
          const created = resp.data?.product;
          productId = created?.id || '';

          // Optimistic update: add to resources ProductStore
          if (created) {
            useProductStore.getState().optimisticAdd(created as any);
          }
          toast.success(t("settings.product.message.created"));
        } else {
          // Update product with embedded specs (price and external_id are in specs now)
          await api.updateProduct(String(data.id), {
            name: productData.name,
            category: String(productData.categoryId),
            image: productData.image,
            tax_rate: productData.taxRate || 0,
            sort_order: productData.sortOrder || 0,
            receipt_name: productData.receiptName,
            kitchen_print_name: productData.kitchenPrintName,
            print_destinations: productData.kitchenPrinterId ? [String(productData.kitchenPrinterId)] : [],
            is_label_print_enabled: productData.isLabelPrintEnabled,
            // Update specs with price and external_id
            specs: [{
              name: productData.name,
              price: productData.price ?? 0,
              display_order: 0,
              is_default: true,
              is_active: true,
              external_id: productData.externalId ?? null,
            }],
          });
          productId = data.id;

          // Optimistic update: update ProductStore cache with snake_case fields
          useProductStore.getState().optimisticUpdate(data.id, (p) => ({
            ...p,
            name: productData.name,
            image: productData.image,
            category: String(productData.categoryId),
            tax_rate: productData.taxRate ?? p.tax_rate,
            sort_order: productData.sortOrder ?? p.sort_order,
            receipt_name: productData.receiptName ?? null,
            kitchen_print_name: productData.kitchenPrintName ?? null,
            print_destinations: productData.kitchenPrinterId ? [String(productData.kitchenPrinterId)] : p.print_destinations,
            is_label_print_enabled: productData.isLabelPrintEnabled ?? p.is_label_print_enabled,
            // Update embedded specs
            specs: [{
              name: productData.name,
              price: productData.price ?? 0,
              display_order: 0,
              is_default: true,
              is_active: true,
              external_id: productData.externalId ?? null,
            }],
          }));
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
              attributeId: pa.to,
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
          async (attrId, _defaultOptionIds, index) => {
            // Note: default_option_idx is now stored on the Attribute itself, not on the binding
            await api.bindProductAttribute({
              product_id: productId,
              attribute_id: attrId,
              is_required: false,
              display_order: index,
            });
          }
        );

        // Handle specifications (for CREATE mode with temp specifications)
        // Specs are now embedded in product, so update the product with all specs
        if (action === 'CREATE' && formData.hasMultiSpec && formData.tempSpecifications && formData.tempSpecifications.length > 0) {
          try {
            // Build embedded specs array from temp specifications
            const embeddedSpecs = formData.tempSpecifications.map((spec: { name: string; price: number; isDefault?: boolean; receiptName?: string }, idx: number) => ({
              name: spec.name,
              price: spec.price,
              display_order: idx,
              is_default: spec.isDefault ?? false,
              is_active: true,
              external_id: null,
            }));

            // Update product with all embedded specs
            await api.updateProduct(productId, {
              specs: embeddedSpecs,
            });
          } catch (error) {
            console.error('Failed to update specifications:', error);
            toast.error(t('settings.specification.message.partialCreateFailed'));
          }
        }
      } else if (entity === 'CATEGORY') {
        const categoryName = formData.name.trim();
        // Backend expects print_destinations array and is_label_print_enabled boolean
        const isLabelPrintEnabled = (formData.isLabelPrintEnabled as unknown) !== false;
        const printDestinations = formData.kitchenPrinterId ? [String(formData.kitchenPrinterId)] : [];
        // Virtual category fields (snake_case for API)
        const isVirtual = formData.isVirtual ?? false;
        const tagIds = formData.tagIds || [];
        const matchMode = formData.matchMode || 'any';

        let categoryId: string;
        if (action === 'CREATE') {
          const resp = await api.createCategory({
            name: categoryName,
            sort_order: formData.sortOrder ?? 0,
            print_destinations: printDestinations,
            is_label_print_enabled: isLabelPrintEnabled,
            is_virtual: isVirtual,
            tag_ids: tagIds,
            match_mode: matchMode,
          });
          categoryId = resp.data?.category?.id || '';
          // Trigger refresh of products store
          useProductStore.getState().fetchAll();
          refreshData(); // Trigger UI refresh
          toast.success(t('settings.category.createCategorySuccess'));
        } else {
          categoryId = String(data.id);
          await api.updateCategory(categoryId, {
            name: categoryName,
            sort_order: formData.sortOrder ?? 0,
            print_destinations: printDestinations,
            is_label_print_enabled: isLabelPrintEnabled,
            is_virtual: isVirtual,
            tag_ids: tagIds,
            match_mode: matchMode,
          });
          // Trigger refresh of products store
          useProductStore.getState().fetchAll();
          refreshData(); // Trigger UI refresh
          toast.success(t('settings.category.updateCategorySuccess'));
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
              attributeId: ca.to,
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
      } else if (entity === 'TAG') {
        const tagName = formData.name.trim();
        const tagColor = formData.color || '#3B82F6';
        const displayOrder = formData.displayOrder ?? 0;

        if (action === 'CREATE') {
          await api.createTag({
            name: tagName,
            color: tagColor,
            display_order: displayOrder
          });
          refreshData();
          toast.success(t('settings.tag.message.created'));
        } else {
          await api.updateTag(String(data.id), {
            name: tagName,
            color: tagColor,
            display_order: displayOrder
          });
          refreshData();
          toast.success(t('settings.tag.message.updated'));
        }
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
      toast.error(t('common.label.none'));
    }
  };

  const handleSelectImage = async () => {
    try {
      const file = await dialogOpen({
        multiple: false,
        filters: [{ name: 'Images', extensions: ['png', 'jpg', 'jpeg', 'webp'] }],
      });
      if (!file || Array.isArray(file)) return;
      const hash = await invoke<string>('save_image', { source_path: file });
      setFormField('image', hash);
    } catch {
      toast.error(t('common.label.none'));
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
      case 'TAG':
        return (
          <TagForm
            formData={{
              name: formData.name,
              color: formData.color || '#3B82F6',
              displayOrder: formData.displayOrder ?? 0,
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
            {t('common.action.cancel')}
          </button>
          {action === 'DELETE' ? (
            <button
              onClick={handleDelete}
              className="px-5 py-2.5 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 transition-colors shadow-lg shadow-red-600/20"
            >
              {t('common.action.delete')}
            </button>
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
