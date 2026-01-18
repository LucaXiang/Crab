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
    selectedSpecification?: { id: string; name: string; receiptName?: string; price?: number }
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
        const binding = bindings?.find(b => b.attribute_id === attr.id);

        let initialIds: string[] = [];

        // Handle default option from binding
        const bindingDefaultId = binding?.default_option_id;

        // Priority 1: Product-specific default from binding
        if (bindingDefaultId && bindingDefaultId > 0) {
           // Filter to ensure default option actually exists and is active
           const exists = options.some(opt => opt.id === bindingDefaultId && opt.is_active);
           if (exists) {
             initialIds = [String(bindingDefaultId)];
           }
        }

        // Priority 2: Attribute-level defaults (Legacy fallback)
        if (initialIds.length === 0) {
           initialIds = options
             .filter(opt => opt.is_default && opt.is_active)
             .map(opt => String(opt.id));
        }

        // Enforce Single Choice constraints
        if (attr.type_.startsWith('SINGLE') && initialIds.length > 1) {
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
      if (attr.type_.includes('REQUIRED')) {
        const selected = selections.get(String(attr.id)) || [];
        if (selected.length === 0) {
          toast.error(t('pos.attributeRequired', { name: attr.name }));
          return;
        }
      }
    }

    // Build ItemAttributeSelection array
    const result: ItemAttributeSelection[] = [];

    selections.forEach((optionIds, attributeId) => {
      const attr = attributes.find(a => String(a.id) === attributeId);
      if (!attr) return;

      const options = allOptions.get(Number(attributeId)) || [];

      optionIds.forEach((optionId) => {
        const option = options.find(o => o.id === Number(optionId));
        if (!option) return;

        result.push({
          attribute_id: attr.id,
          attribute_name: attr.name,
          attribute_receipt_name: attr.receipt_name,
          kitchen_printer_id: attr.kitchen_printer_id,
          option_id: option.id,
          option_name: option.name,
          receipt_name: option.receipt_name,
          price_modifier: option.price_modifier,
        });
      });
    });

    // Get selected specification details
    let selectedSpec: { id: number; name: string; receiptName?: string | null; price?: number } | undefined;
    if (hasMultiSpec && selectedSpecId && specifications) {
      const spec = specifications.find(s => s.id === Number(selectedSpecId));
      if (spec) {
        selectedSpec = {
          id: spec.id,
          name: spec.is_root && !spec.name ? t('settings.product.specification.label.default') : spec.name,
          receiptName: spec.receipt_name,
          price: spec.price,
        };
      }
    }

    onConfirm(result, quantity, discount, discountAuthorizer, selectedSpec);
  };

  // Calculate current price (specification price or base price)
  const currentPrice = hasMultiSpec && selectedSpecId && specifications
    ? specifications.find(s => s.id === Number(selectedSpecId))?.price || basePrice
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
