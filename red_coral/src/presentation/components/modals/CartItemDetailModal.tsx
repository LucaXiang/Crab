import React, { useState, useEffect } from 'react';
import { CartItem, Attribute, AttributeOption, ProductAttribute, ItemOption, ProductSpec } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { useProductStore } from '@/features/product';
import { toast } from '../Toast';
import { logger } from '@/utils/logger';
import { ItemConfiguratorModal } from './ItemConfiguratorModal';

interface CartItemDetailModalProps {
  item: CartItem;
  onClose: () => void;
  onUpdate: (instanceId: string, updates: Partial<CartItem>, authorizer?: { id: number; name: string }) => void;
  onRemove: (instanceId: string, authorizer?: { id: number; name: string }) => void;
  readOnlyAttributes?: boolean;
}

export const CartItemDetailModal = React.memo<CartItemDetailModalProps>(({ item, onClose, onUpdate, onRemove, readOnlyAttributes = false }) => {
  const { t } = useI18n();
  const unpaidQuantity = item.unpaid_quantity;
  const [quantity, setQuantity] = useState(unpaidQuantity);
  const [discount, setDiscount] = useState(item.manual_discount_percent || 0);
  const [discountAuthorizer, setDiscountAuthorizer] = useState<{ id: number; name: string } | undefined>();

  // Specification State
  const [specifications, setSpecifications] = useState<ProductSpec[]>([]);
  const [selectedSpecId, setSelectedSpecId] = useState<number | undefined>(item.selected_specification?.id);
  // Local override for base price (null = use spec/default price)
  const [localBasePrice, setLocalBasePrice] = useState<number | null>(null);

  // Attribute State
  const [isLoadingAttributes, setIsLoadingAttributes] = useState(false);
  const [attributes, setAttributes] = useState<Attribute[]>([]);
  const [allOptions, setAllOptions] = useState<Map<number, AttributeOption[]>>(new Map());
  const [bindings, setBindings] = useState<ProductAttribute[]>([]);
  // Map of attributeId -> Map<optionId, quantity>
  const [selections, setSelections] = useState<Map<number, Map<string, number>>>(new Map());

  // Load attributes on mount
  useEffect(() => {
    let mounted = true;
    const load = async () => {
        setIsLoadingAttributes(true);
        try {
            // Get full product data from store (ProductFull includes attributes)
            const productFull = useProductStore.getState().getById(item.id);
            if (!productFull) {
              logger.error('Product not found in store', undefined, { productId: item.id });
              setIsLoadingAttributes(false);
              return;
            }

            if (!mounted) return;

            // Get specs from product (embedded specs)
            const specs = productFull?.specs || [];
            setSpecifications(specs);

            // If we have specs but none selected (or current selection invalid), select default
            if (specs.length > 1) {
                 // Validate current selection still exists in specs (by id)
                 const currentSpec = specs.find(s => s.id === selectedSpecId);
                 if (!currentSpec) {
                     const defaultSpec = specs.find(s => s.is_default);
                     setSelectedSpecId(defaultSpec?.id ?? specs[0]?.id);
                 }
            }

            // ProductFull.attributes 已包含产品直接绑定 + 分类继承属性
            const attrBindings = productFull?.attributes || [];
            const attributeList: Attribute[] = attrBindings.map(b => b.attribute);
            setAttributes(attributeList);

            const allBindings: ProductAttribute[] = attrBindings.map(binding => ({
              id: binding.id,
              owner_id: binding.is_inherited ? productFull.category_id : item.id,
              attribute_id: binding.attribute.id,
              is_required: binding.is_required,
              display_order: binding.display_order,
              default_option_ids: binding.default_option_ids,
              attribute: binding.attribute,
            }));
            setBindings(allBindings);

            const optionsMap = new Map<number, AttributeOption[]>();
            attrBindings.forEach(binding => {
                const attr = binding.attribute;
                if (attr.options && attr.options.length > 0) {
                    optionsMap.set(attr.id, attr.options);
                }
            });
            setAllOptions(optionsMap);

            // Initialize selections from current item state
            const initialSelections = new Map<number, Map<string, number>>();

            // 1. Pre-fill with existing selections (including quantity)
            item.selected_options?.forEach(sel => {
                const attrKey = sel.attribute_id;
                if (!initialSelections.has(attrKey)) {
                  initialSelections.set(attrKey, new Map<string, number>());
                }
                const optionMap = initialSelections.get(attrKey)!;
                optionMap.set(String(sel.option_id), sel.quantity ?? 1);
            });

            // 2. If no selection for an attribute (and we have defaults), fill with defaults
            if ((!item.selected_options || item.selected_options.length === 0) && attributeList.length > 0) {
                 attributeList.forEach(attr => {
                     const attrId = attr.id;
                     if (!initialSelections.has(attrId)) {
                         // Priority: binding override > attribute default
                         const binding = allBindings.find(b => b.attribute_id === attrId);
                         const defaults = binding?.default_option_ids ?? attr.default_option_ids;
                         if (defaults && defaults.length > 0) {
                             let defaultIds = [...defaults];
                             if (!attr.is_multi_select) {
                                 defaultIds = [defaultIds[0]];
                             } else if (attr.max_selections && defaultIds.length > attr.max_selections) {
                                 defaultIds = defaultIds.slice(0, attr.max_selections);
                             }
                             const optionMap = new Map<string, number>();
                             defaultIds.forEach(id => {
                               optionMap.set(String(id), 1);
                             });
                             initialSelections.set(attrId, optionMap);
                         }
                     }
                 });
            }

            // Ensure all attributes have an entry (even if empty)
            attributeList.forEach(attr => {
              const attrId = attr.id;
              if (!initialSelections.has(attrId)) {
                initialSelections.set(attrId, new Map<string, number>());
              }
            });

            setSelections(initialSelections);

        } catch (err) {
            logger.error('Failed to load attributes', err);
            toast.error(t('error.load_attributes'));
        } finally {
            if (mounted) setIsLoadingAttributes(false);
        }
    };
    load();
    return () => { mounted = false; };
  }, [item.id, item.instance_id]);

  // Handle specification selection (reset price override when changing spec)
  const handleSpecificationSelect = (specId: number) => {
    setSelectedSpecId(specId);
    setLocalBasePrice(null);
  };

  const handleAttributeSelect = (attributeId: number, optionMap: Map<string, number>) => {
      const newSelections = new Map(selections);
      newSelections.set(attributeId, optionMap);
      setSelections(newSelections);
  };

  const handleSave = () => {
    // Build selected options array
    const selectedOptions: ItemOption[] = [];
    selections.forEach((optionMap, attributeId) => {
        const attr = attributes.find(a => a.id === attributeId);
        const options = allOptions.get(attributeId) || [];

        if (attr) {
            optionMap.forEach((qty, idStr) => {
                if (qty <= 0) return; // Skip unselected
                const optionId = parseInt(idStr, 10);
                const opt = options.find(o => o.id === optionId);
                if (opt) {
                    selectedOptions.push({
                        attribute_id: attr.id,
                        attribute_name: attr.name,
                        option_id: optionId,
                        option_name: opt.name,
                        price_modifier: opt.price_modifier ?? null,
                        quantity: qty,
                        receipt_name: opt.receipt_name ?? null,
                        kitchen_print_name: opt.kitchen_print_name ?? null,
                        show_on_receipt: attr.show_on_receipt,
                        show_on_kitchen_print: attr.show_on_kitchen_print,
                    });
                }
            });
        }
    });

    const finalQty = Math.max(1, quantity);
    const finalDisc = Math.min(100, Math.max(0, discount));

    // Resolve specification
    let selectedSpecification = item.selected_specification;
    if (selectedSpecId !== undefined && specifications.length > 0) {
        const spec = specifications.find(s => s.id === selectedSpecId);
        if (spec) {
            selectedSpecification = {
                id: spec.id!,
                name: spec.is_default && !spec.name ? t('settings.product.specification.label.default') : spec.name,
                price: currentPrice,
            };
        }
    }

    // --- Diff: only include actually changed fields ---
    const changes: Partial<CartItem> = {};

    if (finalQty !== unpaidQuantity) {
      changes.quantity = finalQty;
    }

    const origDiscount = item.manual_discount_percent || 0;
    if (finalDisc !== origDiscount) {
      changes.manual_discount_percent = finalDisc;
    }

    // Compare options by (attribute_id, option_id, quantity) tuples
    const origOpts = item.selected_options || [];
    const toKey = (o: ItemOption) => `${o.attribute_id}:${o.option_id}:${o.quantity ?? 1}`;
    const origKeys = origOpts.map(toKey).sort().join(',');
    const newKeys = selectedOptions.map(toKey).sort().join(',');
    if (newKeys !== origKeys) {
      changes.selected_options = selectedOptions;
    }

    // Compare specification by id + price
    const specChanged = selectedSpecification?.id !== item.selected_specification?.id
      || selectedSpecification?.price !== item.selected_specification?.price;
    if (specChanged) {
      changes.selected_specification = selectedSpecification;
    }

    // No actual changes — just close
    if (Object.keys(changes).length === 0) {
      onClose();
      return;
    }

    onUpdate(item.instance_id, changes, discountAuthorizer);
    onClose();
  };

  const handleRemove = (authorizer?: { id: number; name: string }) => {
    onRemove(item.instance_id, authorizer);
    onClose();
  };

  // --- Calculations ---
  // Dynamic price: local override > selected spec price > item original price
  const itemBasePrice = item.original_price || item.price;
  const specPrice = selectedSpecId !== undefined && specifications.length > 0
    ? specifications.find(s => s.id === selectedSpecId)?.price ?? itemBasePrice
    : itemBasePrice;
  const currentPrice = localBasePrice !== null ? localBasePrice : specPrice;

  return (
    <ItemConfiguratorModal
      isOpen={true}
      onClose={onClose}
      title={t('pos.cart.edit_item')}
      productName={item.name}
      isLoading={isLoadingAttributes}
      attributes={attributes}
      allOptions={allOptions}
      bindings={bindings}
      selections={selections}
      onAttributeSelect={handleAttributeSelect}
      basePrice={currentPrice}
      quantity={quantity}
      discount={discount}
      onQuantityChange={setQuantity}
      onDiscountChange={(val, auth) => {
        setDiscount(val);
        setDiscountAuthorizer(auth);
      }}
      onBasePriceChange={setLocalBasePrice}
      onConfirm={handleSave}
      confirmLabel={t('common.action.save')}
      onDelete={handleRemove}
      showDelete={true}
      readOnlyAttributes={readOnlyAttributes}
      specifications={specifications}
      hasMultiSpec={specifications.length > 1}
      selectedSpecId={selectedSpecId}
      onSpecificationSelect={handleSpecificationSelect}
    />
  );
});
