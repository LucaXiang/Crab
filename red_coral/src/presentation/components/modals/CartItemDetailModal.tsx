import React, { useState, useEffect } from 'react';
import { CartItem, Attribute, AttributeOption, ProductAttribute, ItemOption, EmbeddedSpec } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';
import { useProductStore } from '@/features/product';
import { toast } from '../Toast';

const api = createTauriClient();
import { ItemConfiguratorModal } from './ItemConfiguratorModal';

interface CartItemDetailModalProps {
  item: CartItem;
  onClose: () => void;
  onUpdate: (instanceId: string, updates: Partial<CartItem>, options?: { userId?: string }) => void;
  onRemove: (instanceId: string, options?: { userId?: string }) => void;
  readOnlyAttributes?: boolean;
}

export const CartItemDetailModal = React.memo<CartItemDetailModalProps>(({ item, onClose, onUpdate, onRemove, readOnlyAttributes = false }) => {
  const { t } = useI18n();
  const [quantity, setQuantity] = useState(item.quantity);
  const [discount, setDiscount] = useState(item.manual_discount_percent || 0);
  const [discountAuthorizer, setDiscountAuthorizer] = useState<{ id: string; username: string } | undefined>();

  // Specification State
  const [specifications, setSpecifications] = useState<EmbeddedSpec[]>([]);
  const [selectedSpecId, setSelectedSpecId] = useState<string | undefined>(item.selected_specification?.id);

  // Attribute State
  const [isLoadingAttributes, setIsLoadingAttributes] = useState(false);
  const [attributes, setAttributes] = useState<Attribute[]>([]);
  const [allOptions, setAllOptions] = useState<Map<string, AttributeOption[]>>(new Map());
  const [bindings, setBindings] = useState<ProductAttribute[]>([]);
  const [selections, setSelections] = useState<Map<string, string[]>>(new Map());

  // Load attributes on mount
  useEffect(() => {
    let mounted = true;
    const load = async () => {
        setIsLoadingAttributes(true);
        try {
            // Get full product data from store (ProductFull includes attributes)
            const productFull = useProductStore.getState().getById(String(item.id));
            if (!productFull) {
              console.error('Product not found in store:', item.id);
              setIsLoadingAttributes(false);
              return;
            }

            if (!mounted) return;

            // Get specs from product (embedded specs)
            const specs = productFull?.specs || [];
            setSpecifications(specs);

            // If we have specs but none selected (or current selection invalid), select default
            if (specs.length > 1) {
                 const currentSpecIdx = specs.findIndex((_s, idx) => String(idx) === selectedSpecId);
                 if (currentSpecIdx < 0) {
                     const defaultIdx = specs.findIndex(s => s.is_default);
                     setSelectedSpecId(String(defaultIdx >= 0 ? defaultIdx : 0));
                 }
            }

            // Extract attributes from ProductFull.attributes (AttributeBindingFull[])
            const attrBindings = productFull?.attributes || [];

            // Build set of product attribute IDs for deduplication
            const productAttrIds = new Set(attrBindings.map(b => String(b.attribute.id)));

            // Fetch category attributes (inherited)
            let categoryAttributes: Attribute[] = [];
            if (productFull.category) {
                try {
                    categoryAttributes = await api.listCategoryAttributes(productFull.category);
                    // Filter out duplicates (product-level binding takes precedence)
                    categoryAttributes = categoryAttributes.filter(
                        attr => !productAttrIds.has(String(attr.id))
                    );
                } catch (err) {
                    console.warn('Failed to load category attributes:', err);
                }
            }

            // Extract Attribute objects directly (Attribute = Attribute)
            const productAttributeList: Attribute[] = attrBindings.map(binding => binding.attribute);
            // Merge: product attributes first, then category attributes (inherited)
            const attributeList: Attribute[] = [...productAttributeList, ...categoryAttributes];
            setAttributes(attributeList);

            // Convert AttributeBindingFull[] to ProductAttribute[] (AttributeBinding relation)
            const productBindings: ProductAttribute[] = attrBindings.map(binding => ({
              id: binding.id,
              from: String(item.id),
              to: String(binding.attribute.id),
              is_required: binding.is_required,
              display_order: binding.display_order,
              attribute: binding.attribute,
            }));
            // Add category attributes as bindings (inherited, not required by default)
            const categoryBindings: ProductAttribute[] = categoryAttributes.map((attr, idx) => ({
              id: null, // No binding ID for inherited attributes
              from: productFull.category,
              to: String(attr.id),
              is_required: false, // Category attributes are optional by default
              display_order: 1000 + idx, // Place after product attributes
              attribute: attr,
            }));
            setBindings([...productBindings, ...categoryBindings]);

            // Process options from attributes array (unified structure)
            // Options are stored as arrays, the index IS the option ID
            const optionsMap = new Map<string, AttributeOption[]>();
            attrBindings.forEach(binding => {
                const attr = binding.attribute;
                if (attr.options && attr.options.length > 0) {
                    optionsMap.set(String(attr.id), attr.options);
                }
            });
            // Add category attribute options
            categoryAttributes.forEach(attr => {
                if (attr.options && attr.options.length > 0) {
                    optionsMap.set(String(attr.id), attr.options);
                }
            });
            setAllOptions(optionsMap);

            // Initialize selections from current item state
            const initialSelections = new Map<string, string[]>();

            // 1. Pre-fill with existing selections
            item.selected_options?.forEach(sel => {
                const attrKey = String(sel.attribute_id);
                const current = initialSelections.get(attrKey) || [];
                initialSelections.set(attrKey, [...current, String(sel.option_idx)]);
            });

            // 2. If no selection for an attribute (and we have defaults), fill with defaults
            if ((!item.selected_options || item.selected_options.length === 0) && attributeList.length > 0) {
                 attributeList.forEach(attr => {
                     const attrId = String(attr.id);
                     if (!initialSelections.has(attrId)) {
                         // Check default from attribute's default_option_idx
                         if (attr.default_option_idx != null) {
                             // Single choice constraint
                             const isMulti = attr.is_multi_select;
                             if (!isMulti) {
                                 initialSelections.set(attrId, [String(attr.default_option_idx)]);
                             } else {
                                 initialSelections.set(attrId, [String(attr.default_option_idx)]);
                             }
                         }
                     }
                 });
            }

            setSelections(initialSelections);

        } catch (err) {
            console.error(err);
            toast.error(t('error.load_attributes'));
        } finally {
            if (mounted) setIsLoadingAttributes(false);
        }
    };
    load();
    return () => { mounted = false; };
  }, [item.id, item.instance_id]); 

  const handleAttributeSelect = (attributeId: string, optionIds: string[]) => {
      const newSelections = new Map(selections);
      newSelections.set(attributeId, optionIds);
      setSelections(newSelections);
  };

  const handleSave = () => {
    // Validate required attributes
    // Note: With new model, required is determined by binding.is_required, not attr_type
    // For now, skip validation as required info is in bindings
    // TODO: Check bindings for is_required if validation needed

    // Build selected options array
    const selectedOptions: ItemOption[] = [];
    selections.forEach((optionIdxs, attributeId) => {
        const attr = attributes.find(a => String(a.id) === attributeId);
        const options = allOptions.get(attributeId) || [];

        if (attr) {
            optionIdxs.forEach(idxStr => {
                const idx = parseInt(idxStr, 10);
                const opt = options[idx];
                if (opt) {
                    selectedOptions.push({
                        attribute_id: String(attr.id),
                        attribute_name: attr.name,
                        option_idx: idx,
                        option_name: opt.name,
                        price_modifier: opt.price_modifier ?? null,
                    });
                }
            });
        }
    });

    // Final safety check
    const finalQty = Math.max(1, quantity);
    const finalDisc = Math.min(100, Math.max(0, discount));

    // Resolve specification object (using index since EmbeddedSpec doesn't have ID)
    // If spec was changed, use new one; otherwise keep original
    let selectedSpecification = item.selected_specification;
    if (selectedSpecId !== undefined && specifications.length > 0) {
        const specIdx = parseInt(selectedSpecId, 10);
        const spec = specifications[specIdx];
        if (spec) {
            selectedSpecification = {
                id: String(specIdx),
                name: spec.is_default && !spec.name ? t('settings.product.specification.label.default') : spec.name,
                external_id: spec.external_id,
                price: spec.price,
            };
        }
    }

    onUpdate(item.instance_id, {
      quantity: finalQty,
      manual_discount_percent: finalDisc,
      selected_options: selectedOptions,
      selected_specification: selectedSpecification
    }, discountAuthorizer ? { userId: discountAuthorizer.id } : undefined);
    
    onClose();
  };

  const handleRemove = (authorizer?: { id: string; username: string }) => {
    onRemove(item.instance_id, authorizer ? { userId: authorizer.id } : undefined);
    onClose();
  };

  // --- Calculations ---
  const basePrice = item.original_price ?? item.price;
  
  return (
    <ItemConfiguratorModal
      isOpen={true} 
      onClose={onClose}
      title={t('pos.cart.edit_item')}
      productName={item.selected_specification?.external_id ? `${item.selected_specification.external_id} ${item.name}` : item.name}
      isLoading={isLoadingAttributes}
      attributes={attributes}
      allOptions={allOptions}
      bindings={bindings}
      selections={selections}
      onAttributeSelect={handleAttributeSelect}
      basePrice={basePrice}
      quantity={quantity}
      discount={discount}
      onQuantityChange={setQuantity}
      onDiscountChange={(val, auth) => {
        setDiscount(val);
        setDiscountAuthorizer(auth);
      }}
      onConfirm={handleSave}
      confirmLabel={t('common.action.save')}
      onDelete={handleRemove}
      showDelete={true}
      readOnlyAttributes={readOnlyAttributes}
      specifications={specifications}
      hasMultiSpec={specifications.length > 1}
      selectedSpecId={selectedSpecId}
      onSpecificationSelect={setSelectedSpecId}
    />
  );
});
