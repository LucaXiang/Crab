import React, { useState, useEffect } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '../Toast';
import { Attribute, AttributeOption, ItemOption, ProductAttribute, ProductSpec } from '@/core/domain/types';
import { ItemConfiguratorModal } from './ItemConfiguratorModal';

interface ProductOptionsModalProps {
  isOpen: boolean;
  onClose: () => void;
  productName: string;
  basePrice: number;
  attributes: Attribute[];
  allOptions: Map<number, AttributeOption[]>;
  bindings?: ProductAttribute[];
  specifications?: ProductSpec[]; // Product specifications (use index as ID)
  hasMultiSpec?: boolean; // Whether this product has multiple specifications
  onConfirm: (
    selectedOptions: ItemOption[],
    quantity: number,
    discount: number,
    authorizer?: { id: number; name: string },
    selectedSpecification?: { id: number; name: string; receipt_name?: string; price?: number }
  ) => void;
}

export const ProductOptionsModal: React.FC<ProductOptionsModalProps> = React.memo(({
  isOpen,
  onClose,
  productName,
  basePrice,
  attributes,
  allOptions,
  bindings,
  specifications,
  hasMultiSpec,
  onConfirm,
}) => {
  const { t } = useI18n();

  // Track selected specification
  const [selectedSpecId, setSelectedSpecId] = useState<number | null>(null);

  // Track selected options for each attribute: attributeId -> Map<optionId, quantity>
  const [selections, setSelections] = useState<Map<number, Map<string, number>>>(new Map());
  const [quantity, setQuantity] = useState(1);
  const [discount, setDiscount] = useState(0);
  const [discountAuthorizer, setDiscountAuthorizer] = useState<{ id: number; name: string } | undefined>();
  // Local override for base price (null = use spec/default price)
  const [localBasePrice, setLocalBasePrice] = useState<number | null>(null);

  // Initialize selections with default options and specification
  useEffect(() => {
    if (isOpen) {
      setQuantity(1);
      setDiscount(0);
      setDiscountAuthorizer(undefined);
      setLocalBasePrice(null); // Reset price override when opening

      // Initialize specification selection using stable spec.id
      if (hasMultiSpec && specifications && specifications.length > 0) {
        // Find default specification or use first active one
        const defaultSpec = specifications.find(spec => spec.is_default && spec.is_active);
        const firstActive = specifications.find(spec => spec.is_active);
        const initialSpec = defaultSpec ?? firstActive ?? specifications[0];
        setSelectedSpecId(initialSpec?.id ?? null);
      } else {
        setSelectedSpecId(null);
      }

      const initialSelections = new Map<number, Map<string, number>>();

      attributes.forEach((attr) => {
        const options = allOptions.get(attr.id) || [];
        // binding.to is the attribute ID in AttributeBinding relation
        const binding = bindings?.find(b => b.attribute_id === attr.id);

        const optionMap = new Map<string, number>();

        // Priority: binding override > attribute default
        const defaults = binding?.default_option_ids ?? attr.default_option_ids;
        if (defaults && defaults.length > 0) {
          let selectedDefaults = defaults.filter(id => options.some(o => o.id === id));

          // Enforce Single Choice constraints (is_multi_select=false means single)
          const isSingleChoice = !attr.is_multi_select;
          if (isSingleChoice && selectedDefaults.length > 1) {
            selectedDefaults = [selectedDefaults[0]];
          }

          // Enforce max_selections for multi-select
          if (attr.is_multi_select && attr.max_selections && selectedDefaults.length > attr.max_selections) {
            selectedDefaults = selectedDefaults.slice(0, attr.max_selections);
          }

          // Set each default option with quantity 1 (use option.id as key)
          selectedDefaults.forEach(id => {
            optionMap.set(String(id), 1);
          });
        }

        initialSelections.set(attr.id, optionMap);
      });

      setSelections(initialSelections);
    }
  }, [isOpen, attributes, allOptions, bindings, specifications, hasMultiSpec]);

  const handleAttributeSelect = (attributeId: number, optionMap: Map<string, number>) => {
    const newSelections = new Map(selections);
    newSelections.set(attributeId, optionMap);
    setSelections(newSelections);
  };

  const handleConfirm = () => {
    // Validate specification selection if multi-spec is enabled
    if (hasMultiSpec && specifications && specifications.length > 0) {
      if (selectedSpecId === null) {
        toast.error(t('pos.product.specification_required'));
        return;
      }
    }

    // Validate required attributes
    for (const attr of attributes) {
      const binding = bindings?.find(b => b.attribute_id === attr.id);
      if (binding?.is_required) {
        const optionMap = selections.get(attr.id);
        const hasSelection = optionMap && Array.from(optionMap.values()).some(qty => qty > 0);
        if (!hasSelection) {
          toast.error(t('pos.attributeRequired', { name: attr.name }));
          return;
        }
      }
    }

    // Build ItemOption array (backend type)
    const result: ItemOption[] = [];

    selections.forEach((optionMap, attributeId) => {
      const attr = attributes.find(a => a.id === attributeId);
      if (!attr) return;

      const options = allOptions.get(attributeId) || [];

      optionMap.forEach((qty, optionIdStr) => {
        if (qty <= 0) return; // Skip unselected

        const optionId = parseInt(optionIdStr, 10);
        const option = options.find(o => o.id === optionId);
        if (!option) return;

        result.push({
          attribute_id: attr.id,
          attribute_name: attr.name,
          option_id: optionId,
          option_name: option.name,
          price_modifier: option.price_modifier ?? null,
          quantity: qty, // Include quantity in ItemOption
        });
      });
    });

    // Get selected specification details (use stable spec.id)
    // Use currentPrice (which may be user-overridden) instead of spec.price
    let selectedSpec: { id: number; name: string; receipt_name?: string | null; price?: number; is_multi_spec?: boolean } | undefined;
    if (hasMultiSpec && selectedSpecId !== null && specifications) {
      const spec = specifications.find(s => s.id === selectedSpecId);
      if (spec) {
        selectedSpec = {
          id: spec.id!,
          name: spec.is_default && !spec.name ? t('settings.product.specification.label.default') : spec.name,
          price: currentPrice, // Use possibly user-modified price
          is_multi_spec: hasMultiSpec,
        };
      }
    } else if (specifications && specifications.length > 0) {
      // For non-multi-spec products, use default spec
      const defaultSpec = specifications.find(s => s.is_default) ?? specifications[0];
      selectedSpec = {
        id: defaultSpec.id!,
        name: defaultSpec.name,
        price: currentPrice, // Use possibly user-modified price
        is_multi_spec: hasMultiSpec,
      };
    }

    onConfirm(result, quantity, discount, discountAuthorizer, selectedSpec);
  };

  // Handle specification selection (reset price override when changing spec)
  const handleSpecificationSelect = (specId: number) => {
    setSelectedSpecId(specId);
    setLocalBasePrice(null); // Reset price override when changing spec
  };

  // Calculate current price (local override > specification price > base price)
  const specPrice = hasMultiSpec && selectedSpecId !== null && specifications
    ? specifications.find(s => s.id === selectedSpecId)?.price ?? basePrice
    : basePrice;
  const currentPrice = localBasePrice !== null ? localBasePrice : specPrice;

  return (
    <ItemConfiguratorModal
      isOpen={isOpen}
      onClose={onClose}
      title={t('pos.product.select_options')}
      productName={productName}
      attributes={attributes}
      allOptions={allOptions}
      bindings={bindings}
      selections={selections}
      onAttributeSelect={handleAttributeSelect}
      basePrice={currentPrice} // Use specification price if selected
      quantity={quantity}
      discount={discount}
      onQuantityChange={setQuantity}
      onDiscountChange={(val, auth) => {
        setDiscount(val);
        setDiscountAuthorizer(auth);
      }}
      onBasePriceChange={setLocalBasePrice}
      onConfirm={handleConfirm}
      confirmLabel={t('common.action.confirm')}
      // Specification selection
      specifications={specifications}
      hasMultiSpec={hasMultiSpec}
      selectedSpecId={selectedSpecId}
      onSpecificationSelect={handleSpecificationSelect}
    />
  );
});
