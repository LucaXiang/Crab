import { useCallback, useState } from 'react';
import DefaultImage from '../../../assets/reshot.svg';
import { getImageUrl } from '@/core/services/imageCache';
import { useCartActions } from '@/core/stores/cart/useCartStore';
import { useProductStore } from '@/core/stores/resources';
import { useUIActions } from '@/core/stores/ui';
import { useSettingsStore } from '@/core/stores/settings';
import { toast } from '@/presentation/components/Toast';
import { useI18n } from '@/hooks/useI18n';
import type { Product, ItemOption, Attribute, AttributeOption, ProductSpec, ProductAttribute } from '@/core/domain/types';

const CART_ANIMATION_TARGET_X = 190;

interface SelectedProductState {
  product: Product;
  basePrice: number;
  startRect?: DOMRect;
  attributes: Attribute[];
  options: Map<number, AttributeOption[]>;
  bindings: ProductAttribute[];
  specifications?: ProductSpec[];
  hasMultiSpec?: boolean;
}

export function useAddToCart() {
  const { t } = useI18n();
  const { addToCart: addToCartStore } = useCartActions();
  const { addAnimation } = useUIActions();
  const performanceMode = useSettingsStore((state) => state.performanceMode);

  const [optionsModalOpen, setOptionsModalOpen] = useState(false);
  const [selectedProductForOptions, setSelectedProductForOptions] = useState<SelectedProductState | null>(null);

  const playFlyAnimation = useCallback(
    (product: Product, startRect?: DOMRect) => {
      if (startRect && !performanceMode) {
        const id = `fly-${Date.now()}-${Math.random()}`;
        const targetX = CART_ANIMATION_TARGET_X;
        const targetY = window.innerHeight / 2;
        getImageUrl(product.image).then((imageForAnim) => {
          addAnimation({ id, type: 'fly', image: imageForAnim || DefaultImage, startRect, targetX, targetY });
        });
      }
    },
    [addAnimation, performanceMode],
  );

  const addToCart = useCallback(
    async (product: Product, startRect?: DOMRect, skipQuickAdd: boolean = false) => {
      const productFull = useProductStore.getState().getById(product.id);
      if (!productFull) {
        toast.error(t('pos.error.product_not_found'));
        return;
      }

      const attrBindings = productFull.attributes || [];
      const attributeList: Attribute[] = attrBindings.map(b => b.attribute);
      const optionsMap = new Map<number, AttributeOption[]>();
      attributeList.forEach(attr => {
        if (attr.options && attr.options.length > 0) {
          optionsMap.set(attr.id, attr.options);
        }
      });
      const allBindings: ProductAttribute[] = attrBindings.map(binding => ({
        id: binding.id,
        owner_id: binding.is_inherited ? productFull.category_id : product.id,
        attribute_id: binding.attribute.id,
        is_required: binding.is_required,
        display_order: binding.display_order,
        default_option_ids: binding.default_option_ids,
        attribute: binding.attribute,
      }));

      const hasMultiSpec = productFull.specs.length > 1;
      const specifications: ProductSpec[] = productFull.specs || [];
      const defaultSpec = specifications.find((s) => s.is_default) || specifications[0];
      const basePrice = defaultSpec?.price ?? 0;

      // CASE 1: Force Detail View (e.g. Image Click)
      if (skipQuickAdd) {
        setSelectedProductForOptions({
          product, basePrice, startRect,
          attributes: attributeList, options: optionsMap, bindings: allBindings,
          specifications, hasMultiSpec,
        });
        setOptionsModalOpen(true);
        return;
      }

      // CASE 2: Has Multi-Spec or Attributes -> Check if we need modal
      let selectedDefaultSpec: ProductSpec | undefined = undefined;

      if (hasMultiSpec) {
        selectedDefaultSpec = specifications.find((s) => s.is_default === true);
        if (!selectedDefaultSpec) {
          setSelectedProductForOptions({
            product, basePrice, startRect,
            attributes: attributeList, options: optionsMap, bindings: allBindings,
            specifications, hasMultiSpec,
          });
          setOptionsModalOpen(true);
          return;
        }
      }

      if (hasMultiSpec || allBindings.length > 0) {
        const canQuickAdd = allBindings.every(binding => {
          if (!binding.is_required) return true;
          const defaults = binding.default_option_ids
            ?? binding.attribute?.default_option_ids;
          if (!defaults || defaults.length === 0) return false;
          const attrOpts = optionsMap.get(binding.attribute?.id) || [];
          return defaults.some(id => attrOpts.some(o => o.id === id));
        });

        if (canQuickAdd && (!hasMultiSpec || selectedDefaultSpec)) {
          const quickOptions: ItemOption[] = [];
          allBindings.forEach(binding => {
            const attr = binding.attribute;
            if (!attr) return;
            const defaults = binding.default_option_ids
              ?? attr.default_option_ids;
            if (!defaults || defaults.length === 0) return;
            const attrOpts = optionsMap.get(attr.id) || [];
            let count = 0;
            defaults.forEach(id => {
              if (attr.is_multi_select && attr.max_selections && count >= attr.max_selections) return;
              const opt = attrOpts.find(o => o.id === id);
              if (opt) {
                quickOptions.push({
                  attribute_id: attr.id,
                  attribute_name: attr.name,
                  option_id: opt.id,
                  option_name: opt.name,
                  price_modifier: opt.price_modifier ?? null,
                  quantity: 1,
                });
                count++;
              }
            });
          });

          let quickSpec: { id: number; name: string; price?: number; is_multi_spec?: boolean } | undefined;
          if (selectedDefaultSpec) {
            quickSpec = {
              id: selectedDefaultSpec.id!,
              name: selectedDefaultSpec.name,
              price: selectedDefaultSpec.price,
              is_multi_spec: hasMultiSpec,
            };
          } else if (specifications.length > 0) {
            const spec = specifications.find(s => s.is_default) ?? specifications[0];
            quickSpec = {
              id: spec.id!,
              name: spec.name,
              price: spec.price,
              is_multi_spec: hasMultiSpec,
            };
          }

          addToCartStore(product, quickOptions, 1, 0, undefined, quickSpec);
          playFlyAnimation(product, startRect);
          return;
        }

        // Cannot quick-add -> Open Modal for selection
        setSelectedProductForOptions({
          product, basePrice, startRect,
          attributes: attributeList, options: optionsMap, bindings: allBindings,
          specifications, hasMultiSpec,
        });
        setOptionsModalOpen(true);
        return;
      }

      // CASE 3: No Attributes -> Direct Add
      if (!skipQuickAdd) {
        addToCartStore(product);
      }
      playFlyAnimation(product, startRect);
    },
    [addToCartStore, playFlyAnimation, t],
  );

  const handleOptionsConfirmed = useCallback(
    (
      selectedOptions: ItemOption[],
      quantity: number,
      discount: number,
      authorizer?: { id: number; name: string },
      selectedSpecification?: { id: number; name: string; receiptName?: string; price?: number }
    ) => {
      if (!selectedProductForOptions) return;
      const { product, startRect } = selectedProductForOptions;

      addToCartStore(product, selectedOptions, quantity, discount, authorizer, selectedSpecification);
      playFlyAnimation(product, startRect);

      setOptionsModalOpen(false);
      setSelectedProductForOptions(null);
    },
    [selectedProductForOptions, addToCartStore, playFlyAnimation],
  );

  const closeOptionsModal = useCallback(() => {
    setOptionsModalOpen(false);
    setSelectedProductForOptions(null);
  }, []);

  return {
    addToCart,
    optionsModalOpen,
    selectedProductForOptions,
    handleOptionsConfirmed,
    closeOptionsModal,
  };
}
