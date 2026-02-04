import React, { useState, useEffect } from 'react';
import { CartItem, Attribute, AttributeOption, ProductAttribute, ItemOption, EmbeddedSpec } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { useProductStore } from '@/features/product';
import { toast } from '../Toast';
import { ItemConfiguratorModal } from './ItemConfiguratorModal';

interface CartItemDetailModalProps {
  item: CartItem;
  onClose: () => void;
  onUpdate: (instanceId: string, updates: Partial<CartItem>, authorizer?: { id: string; name: string }) => void;
  onRemove: (instanceId: string, authorizer?: { id: string; name: string }) => void;
  readOnlyAttributes?: boolean;
}

export const CartItemDetailModal = React.memo<CartItemDetailModalProps>(({ item, onClose, onUpdate, onRemove, readOnlyAttributes = false }) => {
  const { t } = useI18n();
  const productExternalId = useProductStore(state => state.items.find(p => p.id === item.id)?.external_id);
  const [quantity, setQuantity] = useState(item.quantity);
  const [discount, setDiscount] = useState(item.manual_discount_percent || 0);
  const [discountAuthorizer, setDiscountAuthorizer] = useState<{ id: string; name: string } | undefined>();

  // Specification State
  const [specifications, setSpecifications] = useState<EmbeddedSpec[]>([]);
  const [selectedSpecId, setSelectedSpecId] = useState<string | undefined>(item.selected_specification?.id);
  // Local override for base price (null = use spec/default price)
  const [localBasePrice, setLocalBasePrice] = useState<number | null>(null);

  // Attribute State
  const [isLoadingAttributes, setIsLoadingAttributes] = useState(false);
  const [attributes, setAttributes] = useState<Attribute[]>([]);
  const [allOptions, setAllOptions] = useState<Map<string, AttributeOption[]>>(new Map());
  const [bindings, setBindings] = useState<ProductAttribute[]>([]);
  // Map of attributeId -> Map<optionIdx, quantity>
  const [selections, setSelections] = useState<Map<string, Map<string, number>>>(new Map());

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

            // ProductFull.attributes 已包含产品直接绑定 + 分类继承属性
            const attrBindings = productFull?.attributes || [];
            const attributeList: Attribute[] = attrBindings.map(b => b.attribute);
            setAttributes(attributeList);

            const allBindings: ProductAttribute[] = attrBindings.map(binding => ({
              id: binding.id ?? null,
              in: binding.is_inherited ? productFull.category : String(item.id),
              out: String(binding.attribute.id),
              is_required: binding.is_required,
              display_order: binding.display_order,
              default_option_indices: binding.default_option_indices,
              attribute: binding.attribute,
            }));
            setBindings(allBindings);

            const optionsMap = new Map<string, AttributeOption[]>();
            attrBindings.forEach(binding => {
                const attr = binding.attribute;
                if (attr.options && attr.options.length > 0) {
                    optionsMap.set(String(attr.id), attr.options);
                }
            });
            setAllOptions(optionsMap);

            // Initialize selections from current item state
            const initialSelections = new Map<string, Map<string, number>>();

            // 1. Pre-fill with existing selections (including quantity)
            item.selected_options?.forEach(sel => {
                const attrKey = String(sel.attribute_id);
                if (!initialSelections.has(attrKey)) {
                  initialSelections.set(attrKey, new Map<string, number>());
                }
                const optionMap = initialSelections.get(attrKey)!;
                optionMap.set(String(sel.option_idx), sel.quantity ?? 1);
            });

            // 2. If no selection for an attribute (and we have defaults), fill with defaults
            if ((!item.selected_options || item.selected_options.length === 0) && attributeList.length > 0) {
                 attributeList.forEach(attr => {
                     const attrId = String(attr.id);
                     if (!initialSelections.has(attrId)) {
                         // Priority: binding override > attribute default
                         const binding = allBindings.find(b => b.out === attrId);
                         const defaults = binding?.default_option_indices ?? attr.default_option_indices;
                         if (defaults && defaults.length > 0) {
                             let defaultIdxs = [...defaults];
                             if (!attr.is_multi_select) {
                                 defaultIdxs = [defaultIdxs[0]];
                             } else if (attr.max_selections && defaultIdxs.length > attr.max_selections) {
                                 defaultIdxs = defaultIdxs.slice(0, attr.max_selections);
                             }
                             const optionMap = new Map<string, number>();
                             defaultIdxs.forEach(idx => optionMap.set(String(idx), 1));
                             initialSelections.set(attrId, optionMap);
                         }
                     }
                 });
            }

            // Ensure all attributes have an entry (even if empty)
            attributeList.forEach(attr => {
              const attrId = String(attr.id);
              if (!initialSelections.has(attrId)) {
                initialSelections.set(attrId, new Map<string, number>());
              }
            });

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

  // Handle specification selection (reset price override when changing spec)
  const handleSpecificationSelect = (specId: string) => {
    setSelectedSpecId(specId);
    setLocalBasePrice(null);
  };

  const handleAttributeSelect = (attributeId: string, optionMap: Map<string, number>) => {
      const newSelections = new Map(selections);
      newSelections.set(attributeId, optionMap);
      setSelections(newSelections);
  };

  const handleSave = () => {
    // Build selected options array
    const selectedOptions: ItemOption[] = [];
    selections.forEach((optionMap, attributeId) => {
        const attr = attributes.find(a => String(a.id) === attributeId);
        const options = allOptions.get(attributeId) || [];

        if (attr) {
            optionMap.forEach((qty, idxStr) => {
                if (qty <= 0) return; // Skip unselected
                const idx = parseInt(idxStr, 10);
                const opt = options[idx];
                if (opt) {
                    selectedOptions.push({
                        attribute_id: String(attr.id),
                        attribute_name: attr.name,
                        option_idx: idx,
                        option_name: opt.name,
                        price_modifier: opt.price_modifier ?? null,
                        quantity: qty,
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
        const specIdx = parseInt(selectedSpecId, 10);
        const spec = specifications[specIdx];
        if (spec) {
            selectedSpecification = {
                id: String(specIdx),
                name: spec.is_default && !spec.name ? t('settings.product.specification.label.default') : spec.name,
                price: currentPrice,
            };
        }
    }

    // --- Diff: only include actually changed fields ---
    const changes: Partial<CartItem> = {};

    if (finalQty !== item.quantity) {
      changes.quantity = finalQty;
    }

    const origDiscount = item.manual_discount_percent || 0;
    if (finalDisc !== origDiscount) {
      changes.manual_discount_percent = finalDisc;
    }

    // Compare options by (attribute_id, option_idx, quantity) tuples
    const origOpts = item.selected_options || [];
    const toKey = (o: ItemOption) => `${o.attribute_id}:${o.option_idx}:${o.quantity ?? 1}`;
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

  const handleRemove = (authorizer?: { id: string; name: string }) => {
    onRemove(item.instance_id, authorizer);
    onClose();
  };

  // --- Calculations ---
  // Dynamic price: local override > selected spec price > item original price
  const itemBasePrice = item.original_price ?? item.price;
  const specPrice = selectedSpecId !== undefined && specifications.length > 0
    ? specifications[parseInt(selectedSpecId, 10)]?.price ?? itemBasePrice
    : itemBasePrice;
  const currentPrice = localBasePrice !== null ? localBasePrice : specPrice;

  return (
    <ItemConfiguratorModal
      isOpen={true}
      onClose={onClose}
      title={t('pos.cart.edit_item')}
      productName={productExternalId != null ? `${productExternalId} ${item.name}` : item.name}
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
