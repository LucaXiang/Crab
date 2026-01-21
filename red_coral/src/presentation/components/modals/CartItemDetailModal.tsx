import React, { useState, useEffect } from 'react';
import { CartItem, AttributeTemplate, AttributeOption, ProductAttribute, ItemAttributeSelection, EmbeddedSpec } from '@/core/domain/types';
import { useI18n } from '@/hooks/useI18n';
import { createTauriClient } from '@/infrastructure/api';
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
  const [discount, setDiscount] = useState(item.discountPercent || 0);
  const [discountAuthorizer, setDiscountAuthorizer] = useState<{ id: string; username: string } | undefined>();
  
  // Specification State
  const [specifications, setSpecifications] = useState<EmbeddedSpec[]>([]);
  const [selectedSpecId, setSelectedSpecId] = useState<string | undefined>(item.selectedSpecification?.id);

  // Attribute State
  const [isLoadingAttributes, setIsLoadingAttributes] = useState(false);
  const [attributes, setAttributes] = useState<AttributeTemplate[]>([]);
  const [allOptions, setAllOptions] = useState<Map<string, AttributeOption[]>>(new Map());
  const [bindings, setBindings] = useState<ProductAttribute[]>([]);
  const [selections, setSelections] = useState<Map<string, string[]>>(new Map());

  // Load attributes on mount
  useEffect(() => {
    let mounted = true;
    const load = async () => {
        setIsLoadingAttributes(true);
        try {
            const [attrData, productResp] = await Promise.all([
                api.fetchProductAttributes(String(item.id)),
                api.getProductFull(String(item.id))
            ]);

            if (!mounted) return;

            // Get specs from product (embedded specs)
            const specs = productResp.data?.product?.specs || [];
            setSpecifications(specs);

            // If we have specs but none selected (or current selection invalid), select default
            if (specs.length > 1) {
                 const currentSpecIdx = specs.findIndex((s, idx) => String(idx) === selectedSpecId);
                 if (currentSpecIdx < 0) {
                     const defaultIdx = specs.findIndex(s => s.is_default);
                     setSelectedSpecId(String(defaultIdx >= 0 ? defaultIdx : 0));
                 }
            }

            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            setAttributes((attrData as any).attributes as unknown as AttributeTemplate[]);
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            setBindings((attrData as any).bindings as unknown as ProductAttribute[]);

            // Process options from attributes array (unified structure)
            const optionsMap = new Map<string, AttributeOption[]>();
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            (attrData as any).attributes?.forEach((attr: any) => {
                if (attr.options) {
                    optionsMap.set(String(attr.id), attr.options.map((opt: any) => ({
                        id: opt.id,
                        uuid: '',
                        attribute_id: attr.id,
                        name: opt.name,
                        value_code: opt.value_code || '',
                        price_modifier: opt.price_modifier ?? 0,
                        is_default: opt.is_default ?? false,
                        display_order: 0,
                        is_active: opt.is_active ?? true,
                        receipt_name: opt.receipt_name,
                        created_at: '',
                        updated_at: '',
                    })));
                }
            });
            setAllOptions(optionsMap);

            // Initialize selections from current item state
            const initialSelections = new Map<string, string[]>();

            // 1. Pre-fill with existing selections
            item.selectedOptions?.forEach(sel => {
                const attrKey = String(sel.attribute_id);
                const current = initialSelections.get(attrKey) || [];
                initialSelections.set(attrKey, [...current, String(sel.option_idx)]);
            });

            // 2. If no selection for an attribute (and we have defaults), maybe fill?
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            if ((!item.selectedOptions || item.selectedOptions.length === 0) && (attrData as any).attributes?.length > 0) {
                 // eslint-disable-next-line @typescript-eslint/no-explicit-any
                 ((attrData as any).attributes as Array<{id: string; defaultOptionIds?: string[]; type?: string}>).forEach((attr) => {
                     if (!initialSelections.has(attr.id)) {
                         // eslint-disable-next-line @typescript-eslint/no-explicit-any
                         const binding = (attrData as any).bindings?.find((b: any) => b.attributeId === attr.id);

                         let defaultIds: string[] = [];

                         // Priority 1: Product binding defaults
                         if (binding?.defaultOptionIds && binding.defaultOptionIds.length > 0) {
                             defaultIds = binding.defaultOptionIds;
                         }

                         // Priority 2: Attribute defaults
                         if (defaultIds.length === 0 && attr.defaultOptionIds) {
                             defaultIds = attr.defaultOptionIds;
                         }

                         // Single choice constraint
                         if (attr.type?.startsWith('SINGLE') && defaultIds.length > 1) {
                             defaultIds = [defaultIds[0]];
                         }

                         if (defaultIds.length > 0) {
                             initialSelections.set(attr.id, defaultIds);
                         }
                     }
                 });
            }

            setSelections(initialSelections);

        } catch (err) {
            console.error(err);
            toast.error(t('error.loadAttributes'));
        } finally {
            if (mounted) setIsLoadingAttributes(false);
        }
    };
    load();
    return () => { mounted = false; };
  }, [item.id, item.instanceId]); 

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
    const selectedOptions: ItemAttributeSelection[] = [];
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
                        option_idx: idx,
                        name: attr.name,
                        value: opt.name,
                        price_modifier: opt.price_modifier
                    });
                }
            });
        }
    });

    // Final safety check
    const finalQty = Math.max(1, quantity);
    const finalDisc = Math.min(100, Math.max(0, discount));

    // Resolve specification object (using index since EmbeddedSpec doesn't have ID)
    let selectedSpecification: { id: string; name: string; } | undefined;
    if (selectedSpecId !== undefined) {
        const specIdx = parseInt(selectedSpecId, 10);
        const spec = specifications[specIdx];
        if (spec) {
            selectedSpecification = {
                id: String(specIdx),
                name: spec.is_default && !spec.name ? t('settings.product.specification.label.default') : spec.name,
            };
        }
    }

    onUpdate(item.instanceId, {
      quantity: finalQty,
      discountPercent: finalDisc,
      selectedOptions: selectedOptions,
      selectedSpecification
    }, discountAuthorizer ? { userId: discountAuthorizer.id } : undefined);
    
    onClose();
  };

  const handleRemove = (authorizer?: { id: string; username: string }) => {
    onRemove(item.instanceId, authorizer ? { userId: authorizer.id } : undefined);
    onClose();
  };

  // --- Calculations ---
  const basePrice = item.originalPrice ?? item.price;
  
  return (
    <ItemConfiguratorModal
      isOpen={true} 
      onClose={onClose}
      title={t('pos.cart.editItem')}
      productName={item.externalId ? `${item.externalId} ${item.name}` : item.name}
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
