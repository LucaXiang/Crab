import React, { useState, useEffect } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '../Toast';
import { AttributeTemplate, AttributeOption, ItemAttributeSelection, ProductAttribute, ProductSpecification } from '@/core/domain/types';
import { ItemConfiguratorModal } from './ItemConfiguratorModal';

interface ProductOptionsModalProps {
  isOpen: boolean;
  onClose: () => void;
  productName: string;
  basePrice: number;
  attributes: AttributeTemplate[];
  allOptions: Map<string, AttributeOption[]>;
  bindings?: ProductAttribute[];
  specifications?: ProductSpecification[]; // Product specifications
  hasMultiSpec?: boolean; // Whether this product has multiple specifications
  onConfirm: (
    selectedOptions: ItemAttributeSelection[],
    quantity: number,
    discount: number,
    authorizer?: { id: string; username: string },
    selectedSpecification?: { id: string; name: string; receipt_name?: string; price?: number }
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
  const [selectedSpecId, setSelectedSpecId] = useState<string | null>(null);

  // Track selected option IDs for each attribute: attributeId -> optionIds[]
  const [selections, setSelections] = useState<Map<string, string[]>>(new Map());
  const [quantity, setQuantity] = useState(1);
  const [discount, setDiscount] = useState(0);
  const [discountAuthorizer, setDiscountAuthorizer] = useState<{ id: string; username: string } | undefined>();

  // Initialize selections with default options and specification
  useEffect(() => {
    if (isOpen) {
      setQuantity(1);
      setDiscount(0);
      setDiscountAuthorizer(undefined);

      // Initialize specification selection
      if (hasMultiSpec && specifications && specifications.length > 0) {
        // Find default specification or use first one
        const defaultSpec = specifications.find(spec => spec.is_default && spec.is_active);
        const initialSpec = defaultSpec || specifications.find(spec => spec.is_active);
        setSelectedSpecId(initialSpec?.id ? String(initialSpec.id) : null);
      } else {
        setSelectedSpecId(null);
      }

      const initialSelections = new Map<string, string[]>();

      attributes.forEach((attr) => {
        const options = allOptions.get(String(attr.id)) || [];
        // binding.out is the attribute ID in HasAttribute relation
        const binding = bindings?.find(b => b.out === attr.id);

        let initialIds: string[] = [];

        // Handle default option from binding (default_option_idx is index-based)
        const bindingDefaultIdx = binding?.default_option_idx;

        // Priority 1: Product-specific default from binding
        if (bindingDefaultIdx !== null && bindingDefaultIdx !== undefined && bindingDefaultIdx >= 0) {
           // Filter to ensure default option actually exists and is active
           const opt = options[bindingDefaultIdx];
           if (opt && opt.is_active) {
             initialIds = [String(bindingDefaultIdx)];
           }
        }

        // Priority 2: Attribute-level defaults (Legacy fallback)
        if (initialIds.length === 0) {
           initialIds = options
             .filter(opt => opt.is_default && opt.is_active)
             .map((_, idx) => String(idx));
        }

        // Enforce Single Choice constraints
        const isSingleChoice = attr.attr_type.startsWith('SINGLE') || attr.attr_type === 'single_select';
        if (isSingleChoice && initialIds.length > 1) {
           initialIds = [initialIds[0]];
        }

        initialSelections.set(String(attr.id), initialIds);
      });

      setSelections(initialSelections);
    }
  }, [isOpen, attributes, allOptions, bindings, specifications, hasMultiSpec]);

  const handleAttributeSelect = (attributeId: string, optionIds: string[]) => {
    const newSelections = new Map(selections);
    newSelections.set(attributeId, optionIds);
    setSelections(newSelections);
  };

  const handleConfirm = () => {
    // Validate specification selection if multi-spec is enabled
    if (hasMultiSpec && specifications && specifications.length > 0) {
      if (!selectedSpecId) {
        toast.error(t('pos.product.specificationRequired'));
        return;
      }
    }

    // Validate required attributes
    for (const attr of attributes) {
      if (attr.attr_type.includes('REQUIRED')) {
        const selected = selections.get(String(attr.id)) || [];
        if (selected.length === 0) {
          toast.error(t('pos.attributeRequired', { name: attr.name }));
          return;
        }
      }
    }

    // Build ItemAttributeSelection array
    const result: ItemAttributeSelection[] = [];

    selections.forEach((optionIdxs, attributeId) => {
      const attr = attributes.find(a => String(a.id) === attributeId);
      if (!attr) return;

      const options = allOptions.get(attributeId) || [];

      optionIdxs.forEach((optionIdxStr) => {
        const optionIdx = parseInt(optionIdxStr, 10);
        const option = options[optionIdx];
        if (!option) return;

        result.push({
          attribute_id: String(attr.id),
          option_idx: optionIdx,
          name: attr.name,
          value: option.name,
          price_modifier: option.price_modifier,
          attribute_name: attr.name,
          attribute_receipt_name: attr.receipt_name,
          kitchen_printer: attr.kitchen_printer,
          option_name: option.name,
          receipt_name: option.receipt_name,
        });
      });
    });

    // Get selected specification details
    let selectedSpec: { id: string; name: string; receipt_name?: string | null; price?: number } | undefined;
    if (hasMultiSpec && selectedSpecId && specifications) {
      const spec = specifications.find(s => String(s.id) === selectedSpecId);
      if (spec) {
        selectedSpec = {
          id: String(spec.id),
          name: spec.is_root && !spec.name ? t('settings.product.specification.label.default') : spec.name,
          price: spec.price,
        };
      }
    }

    onConfirm(result, quantity, discount, discountAuthorizer, selectedSpec);
  };

  // Calculate current price (specification price or base price)
  const currentPrice = hasMultiSpec && selectedSpecId && specifications
    ? specifications.find(s => String(s.id) === selectedSpecId)?.price || basePrice
    : basePrice;

  return (
    <ItemConfiguratorModal
      isOpen={isOpen}
      onClose={onClose}
      title={t('pos.product.selectOptions')}
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
      onConfirm={handleConfirm}
      confirmLabel={t('common.confirm')}
      // Specification selection
      specifications={specifications}
      hasMultiSpec={hasMultiSpec}
      selectedSpecId={selectedSpecId}
      onSpecificationSelect={setSelectedSpecId}
    />
  );
});
