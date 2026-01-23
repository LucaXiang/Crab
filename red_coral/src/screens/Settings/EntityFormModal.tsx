import React, { useEffect, useState, useRef } from 'react';
import { X } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import {
  useSettingsModal,
  useSettingsFormMeta,
  useSettingsStore,
} from '@/core/stores/settings/useSettingsStore';
import { createTauriClient } from '@/infrastructure/api';
import { invoke } from '@tauri-apps/api/core';
import type { Attribute, Product } from '@/core/domain/types/api';
import { useProductStore, useZones, useCategoryStore } from '@/core/stores/resources';
import { getErrorMessage } from '@/utils/error';

const api = createTauriClient();
import { toast } from '@/presentation/components/Toast';
import { open as dialogOpen } from '@tauri-apps/plugin-dialog';
import { syncAttributeBindings } from './utils';
import { useZoneStore, useTableStore } from '@/core/stores/resources';

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

  const refreshZones = () => useZoneStore.getState().fetchAll(true);
  const refreshTables = () => useTableStore.getState().fetchAll(true);
  const [unsavedDialogOpen, setUnsavedDialogOpen] = useState(false);
  const [isSaving, setIsSaving] = useState(false);
  const [inheritedAttributeIds, setInheritedAttributeIds] = useState<string[]>([]);
  const defaultCategorySet = useRef(false);

  useEffect(() => {
    if (!modal.open) {
      setInheritedAttributeIds([]);
    }
  }, [modal.open, modal.entity, modal.action, modal.data, formData.print_destinations]);

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
      if (categories.length > 0 && modal.action === 'CREATE' && !formData.category && !defaultCategorySet.current) {
        setFormData({ category: categories[0].id ?? '' });
        defaultCategorySet.current = true;
      }
    }
  }, [modal.open, modal.entity, modal.action, categories.length, categoryStore.isLoaded]);

  // Load full product data (specs, attributes, tags) when editing a product
  useEffect(() => {
    if (modal.open && modal.entity === 'PRODUCT' && modal.action === 'EDIT' && modal.data?.id) {
      const loadProductFullData = async () => {
        try {
          const productFull = await api.getProductFull(String(modal.data.id));
          if (!productFull) {
            console.error('Failed to load product full data');
            return;
          }

          // Debug: log print settings from API
          console.log('[DEBUG] getProductFull - is_kitchen_print_enabled:', productFull.is_kitchen_print_enabled,
            'is_label_print_enabled:', productFull.is_label_print_enabled);

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
            selected_attribute_ids: attributeIds,
            attribute_default_options: defaultOptions,
            // Store specs for ProductForm to use
            specs: productFull.specs,
            // Store tag IDs
            tags: tagIds,
            // Determine has_multi_spec from specs count (specs.length > 1)
            has_multi_spec: productFull.specs.length > 1,
            // Print settings
            is_kitchen_print_enabled: productFull.is_kitchen_print_enabled,
            is_label_print_enabled: productFull.is_label_print_enabled,
            print_destinations: productFull.kitchen_print_destinations,
            label_print_destinations: productFull.label_print_destinations,
            kitchen_print_name: productFull.kitchen_print_name,
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
          const catAttrs = await api.listCategoryAttributes(String(modal.data.id));
          // API returns Attribute[] (with id field), not bindings (with attribute_id)
          const safeAttrs: Attribute[] = catAttrs ?? [];
          const attributeIds = safeAttrs.map((ca) => ca.id).filter(Boolean) as string[];

          // Load default options from category attributes
          const defaultOptions: Record<string, string[]> = {};
          safeAttrs.forEach((ca) => {
            const defaults = ca.default_option_idx != null ? [String(ca.default_option_idx)] : [];
            if (defaults.length > 0 && ca.id) {
              defaultOptions[ca.id] = defaults;
            }
          });

          setAsyncFormData({
            selected_attribute_ids: attributeIds,
            attribute_default_options: defaultOptions
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
        TABLE: t('settings.table.add_table'),
        ZONE: t('settings.zone.add_zone'),
        PRODUCT: t('settings.product.add_product'),
        CATEGORY: t('settings.category.add_category'),
        TAG: t('settings.tag.add_tag')
      },
      EDIT: {
        TABLE: t('settings.table.edit_table'),
        ZONE: t('settings.zone.edit_zone'),
        PRODUCT: t('settings.product.edit_product'),
        CATEGORY: t('settings.category.edit_category'),
        TAG: t('settings.tag.edit_tag')
      },
      DELETE: {
        TABLE: t('settings.table.delete_table'),
        ZONE: t('settings.zone.delete_zone'),
        PRODUCT: t('settings.product.delete_product'),
        CATEGORY: t('settings.category.delete_category'),
        TAG: t('settings.tag.delete_tag')
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
        refreshTables(); // Refresh tables from server
        toast.success(t('settings.table.table_deleted'));
      } else if (entity === 'ZONE') {
        try {
          await api.deleteZone(String(data.id));
          refreshZones(); // Refresh zones from server
          toast.success(t('settings.zone.zone_deleted'));
        } catch (e: unknown) {
          // Use getErrorMessage to get localized message from numeric error code
          toast.error(getErrorMessage(e));
          return;
        }
      } else if (entity === 'PRODUCT') {
        try {
          await api.deleteProduct(String(data.id));
          // Optimistic update: remove from ProductStore
          useProductStore.getState().optimisticRemove(data.id);
          toast.success(t('settings.product.product_deleted'));
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
          toast.success(t('settings.category.category_deleted'));
        } catch (e: any) {
          toast.error(getErrorMessage(e));
          return;
        }
      } else if (entity === 'TAG') {
        try {
          await api.deleteTag(String(data.id));
          // Trigger refresh
          refreshData();
          toast.success(t('settings.tag.tag_deleted'));
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
    // Prevent duplicate submissions
    if (isSaving) return;

    const hasError = Object.values(formErrors).some(Boolean);
    if (hasError) {
      toast.error(t('common.message.invalid_form'));
      return;
    }

    setIsSaving(true);
    try {
      if (entity === 'TABLE') {
        const tablePayload = {
          name: formData.name.trim(),
          zone: formData.zone,
          capacity: Math.max(1, formData.capacity),
          is_active: formData.is_active ?? true,
        };
        if (action === 'CREATE') {
          await api.createTable({ name: tablePayload.name, zone: String(tablePayload.zone), capacity: Number(tablePayload.capacity) });
          refreshTables(); // Refresh tables from server
          toast.success(t("settings.table.message.created"));
        } else {
          await api.updateTable(String(data.id), {
            name: tablePayload.name,
            zone: String(tablePayload.zone),  // 支持修改区域
            capacity: Number(tablePayload.capacity),
            is_active: tablePayload.is_active,
          });
          refreshTables(); // Refresh tables from server
          toast.success(t("settings.table.message.updated"));
        }
      } else if (entity === 'ZONE') {
        const zonePayload = {
          name: formData.name.trim(),
          description: formData.description?.trim() || undefined,
          is_active: formData.is_active ?? true,
        };
        if (action === 'CREATE') {
          await api.createZone({
            name: zonePayload.name,
            description: zonePayload.description,
          });
          refreshZones(); // Refresh zones from server
          toast.success(t("settings.zone.message.created"));
        } else {
          await api.updateZone(String(data.id), {
            name: zonePayload.name,
            description: zonePayload.description,
            is_active: zonePayload.is_active,
          });
          refreshZones(); // Refresh zones from server
          toast.success(t("settings.zone.message.updated"));
        }
      } else if (entity === 'PRODUCT') {
        if (!formData.name?.trim()) {
          toast.error(t('settings.product.form.name_required'));
          return;
        }
        // Get price and externalId from root spec
        const rootSpec = formData.specs?.find(s => s.is_root);
        const price = rootSpec?.price ?? 0;
        const externalId = rootSpec?.external_id;

        if (externalId === undefined || externalId === null) {
          toast.error(t('settings.external_id_required'));
          return;
        }

        // external_id uniqueness is enforced by backend via product_spec table UNIQUE index

        if (!formData.category) {
          if (categories.length === 0) {
            toast.error(t('settings.category.create_first'));
          } else {
            toast.error(t('settings.category.required'));
          }
          return;
        }

        const productPayload = {
          name: formData.name.trim(),
          category: formData.category,
          image: formData.image?.trim() ?? '',
          sort_order: formData.sort_order ?? 0,
          tax_rate: formData.tax_rate ?? 0,
          receipt_name: formData.receipt_name?.trim() ?? undefined,
          kitchen_print_name: formData.kitchen_print_name?.trim() ?? undefined,
          kitchen_print_destinations: formData.print_destinations ?? [],  // Form field maps to kitchen
          label_print_destinations: formData.label_print_destinations ?? [],
          is_kitchen_print_enabled: formData.is_kitchen_print_enabled ?? -1,  // 默认继承
          is_label_print_enabled: formData.is_label_print_enabled ?? -1,  // 默认继承
          is_active: formData.is_active ?? true,
          tags: formData.tags ?? [],
          specs: formData.specs ?? [],
          // These are used for the default spec (extracted from formData.specs above)
          price: Math.max(0.01, price),
          externalId: externalId,
        };

        // Save the selected category for next time (lookup name by ID)
        const category = categories.find(c => String(c.id) === String(productPayload.category));
        if (category) {
          setLastSelectedCategory(category.name);
        }

        let productId: string;
        if (action === 'CREATE') {
          // Create product with embedded specs (price is in specs, not on product)
          const created = await api.createProduct({
            name: productPayload.name,
            category: String(productPayload.category),
            image: productPayload.image,
            tax_rate: productPayload.tax_rate,
            sort_order: productPayload.sort_order,
            receipt_name: productPayload.receipt_name,
            kitchen_print_name: productPayload.kitchen_print_name,
            kitchen_print_destinations: productPayload.kitchen_print_destinations,
            label_print_destinations: productPayload.label_print_destinations,
            is_kitchen_print_enabled: productPayload.is_kitchen_print_enabled,
            is_label_print_enabled: productPayload.is_label_print_enabled,
            // Price is embedded in specs
            specs: [{
              name: productPayload.name,
              price: productPayload.price ?? 0,
              display_order: 0,
              is_default: true,
              is_active: true,
              is_root: true,
              external_id: productPayload.externalId ?? null,
              receipt_name: null,
            }],
          });
          productId = created?.id || '';

          // Optimistic update: add to resources ProductStore
          if (created?.id) {
            useProductStore.getState().optimisticAdd(created as Product & { id: string });
          }
          toast.success(t("settings.product.message.created"));
        } else {
          // Update product - preserve all specs, only update root spec's price/external_id
          const existingSpecs = formData.specs ?? [];
          const updatedSpecs = existingSpecs.length > 0
            ? existingSpecs.map(spec => spec.is_root ? {
                ...spec,
                name: spec.name,  // Keep spec name (may differ from product name for multi-spec)
                price: spec.price,
                external_id: spec.external_id,
              } : spec)
            : [{
                name: productPayload.name,
                price: productPayload.price ?? 0,
                display_order: 0,
                is_default: true,
                is_active: true,
                is_root: true,
                external_id: productPayload.externalId ?? null,
                receipt_name: null,
              }];

          const updatePayload = {
            name: productPayload.name,
            category: String(productPayload.category),
            image: productPayload.image,
            tax_rate: productPayload.tax_rate,
            sort_order: productPayload.sort_order,
            receipt_name: productPayload.receipt_name,
            kitchen_print_name: productPayload.kitchen_print_name,
            kitchen_print_destinations: productPayload.kitchen_print_destinations,
            label_print_destinations: productPayload.label_print_destinations,
            is_kitchen_print_enabled: productPayload.is_kitchen_print_enabled,
            is_label_print_enabled: productPayload.is_label_print_enabled,
            is_active: productPayload.is_active,
            specs: updatedSpecs,
          };
          console.log('[DEBUG] Update product payload:', JSON.stringify(updatePayload, null, 2));
          const updated = await api.updateProduct(String(data.id), updatePayload);
          productId = data.id;

          // Debug: log the API response
          console.log('[DEBUG] updateProduct response - is_kitchen_print_enabled:', updated?.is_kitchen_print_enabled,
            'is_label_print_enabled:', updated?.is_label_print_enabled);

          // Update ProductStore cache with API response data
          if (updated) {
            useProductStore.getState().optimisticUpdate(data.id, () => updated as Product);
          }
          toast.success(t("settings.product.message.updated"));
        }

        // Handle attribute bindings
        const selectedAttributeIds = formData.selected_attribute_ids || [];

        // Get existing bindings (only for EDIT mode)
        let existingBindings: any[] = [];
        if (action === 'EDIT') {
          try {
            const productAttrs = await api.fetchProductAttributes(productId);
            // Transform to expected format (handle undefined/null)
            existingBindings = (productAttrs ?? []).map((pa: any) => ({
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
          formData.attribute_default_options || {},
          existingBindings,
          async (attrId) => api.unbindProductAttribute(String(attrId)),
          async (attrId, defaultOptionIds, index) => {
            // default_option_idx is stored on the AttributeBinding
            const defaultOptionIdx = defaultOptionIds.length > 0 ? parseInt(defaultOptionIds[0], 10) : undefined;
            await api.bindProductAttribute({
              product_id: productId,
              attribute_id: attrId,
              is_required: false,
              display_order: index,
              default_option_idx: !isNaN(defaultOptionIdx as number) ? defaultOptionIdx : undefined,
            });
          }
        );

        // Handle specifications (for CREATE mode with multi-spec)
        // Specs are now embedded in product, so update the product with all specs
        if (action === 'CREATE' && formData.has_multi_spec && formData.specs && formData.specs.length > 1) {
          try {
            // Build embedded specs array from specs
            const embeddedSpecs = formData.specs.map((spec, idx: number) => ({
              name: spec.name,
              price: spec.price,
              display_order: idx,
              is_default: spec.is_default ?? false,
              is_active: true,
              is_root: spec.is_root,
              external_id: spec.external_id ?? null,
            }));

            // Update product with all embedded specs
            await api.updateProduct(productId, {
              specs: embeddedSpecs,
            });
          } catch (error) {
            console.error('Failed to update specifications:', error);
            toast.error(t('settings.specification.message.partial_create_failed'));
          }
        }
      } else if (entity === 'CATEGORY') {
        // PrintState 转 boolean: 1=true, 0=false (Category API 使用 boolean)
        const kitchenEnabled = formData.is_kitchen_print_enabled === 0 ? false : true;
        const labelEnabled = formData.is_label_print_enabled === 0 ? false : true;
        const categoryPayload = {
          name: formData.name.trim(),
          sort_order: formData.sort_order ?? 0,
          kitchen_print_destinations: formData.print_destinations ?? [],  // Form field maps to kitchen
          label_print_destinations: formData.label_print_destinations ?? [],
          is_kitchen_print_enabled: kitchenEnabled,
          is_label_print_enabled: labelEnabled,
          is_active: formData.is_active ?? true,
          is_virtual: formData.is_virtual ?? false,
          tag_ids: formData.tag_ids ?? [],
          match_mode: formData.match_mode ?? 'any',
        };

        let categoryId: string;
        if (action === 'CREATE') {
          const created = await api.createCategory({
            name: categoryPayload.name,
            sort_order: categoryPayload.sort_order,
            kitchen_print_destinations: categoryPayload.kitchen_print_destinations,
            label_print_destinations: categoryPayload.label_print_destinations,
            is_kitchen_print_enabled: categoryPayload.is_kitchen_print_enabled,
            is_label_print_enabled: categoryPayload.is_label_print_enabled,
            is_virtual: categoryPayload.is_virtual,
            tag_ids: categoryPayload.tag_ids,
            match_mode: categoryPayload.match_mode,
          });
          categoryId = created?.id || '';
          // Trigger refresh of products store
          useProductStore.getState().fetchAll();
          refreshData(); // Trigger UI refresh
          toast.success(t('settings.category.create_category_success'));
        } else {
          categoryId = String(data.id);
          await api.updateCategory(categoryId, {
            name: categoryPayload.name,
            sort_order: categoryPayload.sort_order,
            kitchen_print_destinations: categoryPayload.kitchen_print_destinations,
            label_print_destinations: categoryPayload.label_print_destinations,
            is_kitchen_print_enabled: categoryPayload.is_kitchen_print_enabled,
            is_label_print_enabled: categoryPayload.is_label_print_enabled,
            is_active: categoryPayload.is_active,
            is_virtual: categoryPayload.is_virtual,
            tag_ids: categoryPayload.tag_ids,
            match_mode: categoryPayload.match_mode,
          });
          // Trigger refresh of products store
          useProductStore.getState().fetchAll();
          refreshData(); // Trigger UI refresh
          toast.success(t('settings.category.update_category_success'));
        }

        // Handle attribute bindings
        const selectedAttributeIds = formData.selected_attribute_ids || [];

        // Get existing bindings (only for EDIT mode)
        let existingBindings: any[] = [];
        if (action === 'EDIT') {
          try {
            const catAttrs = await api.listCategoryAttributes(categoryId);
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
          formData.attribute_default_options || {},
          existingBindings,
          async (attrId) => api.unbindCategoryAttribute(categoryId, String(attrId)),
          async (attrId, defaultOptionIds, index) => {
            await api.bindCategoryAttribute({
              category_id: categoryId,
              attribute_id: attrId,
              is_required: false,
              display_order: index,
              default_option_idx: defaultOptionIds?.[0] ? Number(defaultOptionIds[0]) : undefined
            });
          }
        );
      } else if (entity === 'TAG') {
        const tagPayload = {
          name: formData.name.trim(),
          color: formData.color || '#3B82F6',
          display_order: formData.display_order ?? 0,
          is_active: formData.is_active ?? true,
        };

        if (action === 'CREATE') {
          // Note: is_active defaults to true on server for new tags
          await api.createTag({
            name: tagPayload.name,
            color: tagPayload.color,
            display_order: tagPayload.display_order,
          });
          refreshData();
          toast.success(t('settings.tag.message.created'));
        } else {
          await api.updateTag(String(data.id), {
            name: tagPayload.name,
            color: tagPayload.color,
            display_order: tagPayload.display_order,
            is_active: tagPayload.is_active,
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
    } catch (e: unknown) {
      // Use getErrorMessage for localized error display
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
      const hash = await invoke<string>('save_image', { source_path: file });
      setFormField('image', hash);
    } catch {
      toast.error(t('common.label.none'));
    }
  };

  const accent = getAccentColor();
  const hasError = Object.values(formErrors).some(Boolean);
  const isSaveDisabled = !isFormDirty || hasError || isSaving;
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
            formData={{
              name: formData.name,
              capacity: formData.capacity ?? 1,
              zone: formData.zone ?? '',
              is_active: formData.is_active ?? true,
            }}
            zones={zones}
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            onFieldChange={setFormField as (field: string, value: any) => void}
            t={t}
          />
        );
      case 'ZONE':
        return (
          <ZoneForm
            formData={{
              name: formData.name,
              description: formData.description ?? '',
              is_active: formData.is_active ?? true,
            }}
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            onFieldChange={setFormField as (field: string, value: any) => void}
            t={t}
          />
        );
      case 'PRODUCT': {
        // Get price and externalId from root spec for ProductForm
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
              selected_tag_ids: formData.tags,
            }}
            categories={categories}
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            onFieldChange={setFormField as (field: string, value: any) => void}
            onSelectImage={handleSelectImage}
            t={t}
            inheritedAttributeIds={inheritedAttributeIds}
          />
        );
      }
      case 'CATEGORY':
        return (
          <CategoryForm
            formData={{
              name: formData.name,
              print_destinations: formData.print_destinations,
              is_kitchen_print_enabled: formData.is_kitchen_print_enabled,  // PrintState: 0=禁用, 1=启用
              is_label_print_enabled: formData.is_label_print_enabled,      // PrintState: 0=禁用, 1=启用
              is_active: formData.is_active ?? true,
              selected_attribute_ids: formData.selected_attribute_ids,
              attribute_default_options: formData.attribute_default_options,
              is_virtual: formData.is_virtual,
              tag_ids: formData.tag_ids,
              match_mode: formData.match_mode,
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
              display_order: formData.display_order ?? 0,
              is_active: formData.is_active ?? true,
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
