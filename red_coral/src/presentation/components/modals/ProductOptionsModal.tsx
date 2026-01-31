import React, { useState, useEffect } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '../Toast';
import { Attribute, AttributeOption, ItemOption, ProductAttribute, EmbeddedSpec } from '@/core/domain/types';
import { ItemConfiguratorModal } from './ItemConfiguratorModal';

interface ProductOptionsModalProps {
  isOpen: boolean;
  onClose: () => void;
  productName: string;
  basePrice: number;
  attributes: Attribute[];
  allOptions: Map<string, AttributeOption[]>;
  bindings?: ProductAttribute[];
  specifications?: EmbeddedSpec[]; // Embedded specifications (use index as ID)
  hasMultiSpec?: boolean; // Whether this product has multiple specifications
  onConfirm: (
    selectedOptions: ItemOption[],
    quantity: number,
    discount: number,
    authorizer?: { id: string; name: string },
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
  const [discountAuthorizer, setDiscountAuthorizer] = useState<{ id: string; name: string } | undefined>();
  // Local override for base price (null = use spec/default price)
  const [localBasePrice, setLocalBasePrice] = useState<number | null>(null);

  // Initialize selections with default options and specification
  useEffect(() => {
    if (isOpen) {
      setQuantity(1);
      setDiscount(0);
      setDiscountAuthorizer(undefined);
      setLocalBasePrice(null); // Reset price override when opening

      // Initialize specification selection (use index as ID since EmbeddedSpec has no id)
      if (hasMultiSpec && specifications && specifications.length > 0) {
        // Find default specification index or use first active one
        const defaultIdx = specifications.findIndex(spec => spec.is_default && spec.is_active);
        const firstActiveIdx = specifications.findIndex(spec => spec.is_active);
        const initialIdx = defaultIdx >= 0 ? defaultIdx : (firstActiveIdx >= 0 ? firstActiveIdx : 0);
        setSelectedSpecId(String(initialIdx));
      } else {
        setSelectedSpecId(null);
      }

      const initialSelections = new Map<string, string[]>();

      attributes.forEach((attr) => {
        const options = allOptions.get(String(attr.id)) || [];
        // binding.to is the attribute ID in AttributeBinding relation
        const binding = bindings?.find(b => b.to === attr.id);

        let initialIds: string[] = [];

        // Use attribute-level default_option_idx
        const attrDefaultIdx = attr.default_option_idx;
        if (attrDefaultIdx !== null && attrDefaultIdx !== undefined && attrDefaultIdx >= 0) {
           const opt = options[attrDefaultIdx];
           if (opt && opt.is_active) {
             initialIds = [String(attrDefaultIdx)];
           }
        }

        // Enforce Single Choice constraints (is_multi_select=false means single)
        const isSingleChoice = !attr.is_multi_select;
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
        toast.error(t('pos.product.specification_required'));
        return;
      }
    }

    // Validate required attributes
    // Note: With new model, required is determined by binding.is_required, not attr_type
    for (const attr of attributes) {
      const binding = bindings?.find(b => b.to === attr.id);
      if (binding?.is_required) {
        const selected = selections.get(String(attr.id)) || [];
        if (selected.length === 0) {
          toast.error(t('pos.attributeRequired', { name: attr.name }));
          return;
        }
      }
    }

    // Build ItemOption array (backend type)
    const result: ItemOption[] = [];

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
          attribute_name: attr.name,
          option_idx: optionIdx,
          option_name: option.name,
          price_modifier: option.price_modifier ?? null,
        });
      });
    });

    // Get selected specification details (use index as ID)
    // Use currentPrice (which may be user-overridden) instead of spec.price
    let selectedSpec: { id: string; name: string; external_id?: number | null; receipt_name?: string | null; price?: number; is_multi_spec?: boolean } | undefined;
    if (hasMultiSpec && selectedSpecId !== null && specifications) {
      const specIdx = parseInt(selectedSpecId, 10);
      const spec = specifications[specIdx];
      if (spec) {
        selectedSpec = {
          id: String(specIdx),
          name: spec.is_default && !spec.name ? t('settings.product.specification.label.default') : spec.name,
          external_id: spec.external_id,
          price: currentPrice, // Use possibly user-modified price
          is_multi_spec: hasMultiSpec,
        };
      }
    } else if (specifications && specifications.length > 0) {
      // For non-multi-spec products, use default spec
      const defaultSpec = specifications.find(s => s.is_default) ?? specifications[0];
      const specIdx = specifications.indexOf(defaultSpec);
      selectedSpec = {
        id: String(specIdx),
        name: defaultSpec.name,
        external_id: defaultSpec.external_id,
        price: currentPrice, // Use possibly user-modified price
        is_multi_spec: hasMultiSpec,
      };
    }

    onConfirm(result, quantity, discount, discountAuthorizer, selectedSpec);
  };

  // Handle specification selection (reset price override when changing spec)
  const handleSpecificationSelect = (specId: string) => {
    setSelectedSpecId(specId);
    setLocalBasePrice(null); // Reset price override when changing spec
  };

  // Calculate current price (local override > specification price > base price)
  const specPrice = hasMultiSpec && selectedSpecId !== null && specifications
    ? specifications[parseInt(selectedSpecId, 10)]?.price ?? basePrice
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
