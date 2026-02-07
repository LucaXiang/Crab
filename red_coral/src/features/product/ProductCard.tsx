import React, { useRef, useCallback } from 'react';
import { Product } from '@/core/domain/types';
import DefaultImage from '@/assets/reshot.svg';
import { useImageUrl } from '@/core/hooks';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { useLongPress } from '@/hooks/useLongPress';
import { formatCurrency } from '@/utils/currency';

/**
 * Extended Product type with computed fields from root spec
 */
export interface ProductWithPrice extends Product {
  price?: number;      // From root spec
}

interface ProductCardProps {
  product: ProductWithPrice;
  onAdd: (product: ProductWithPrice, startRect?: DOMRect, skipQuickAdd?: boolean) => void;
  onLongPress?: (product: ProductWithPrice) => void;
  priority?: boolean;
}

export const ProductCard = React.memo<ProductCardProps>(
  ({ product, onAdd, onLongPress, priority = false }) => {
    const imgRef = useRef<HTMLImageElement>(null);
    const performanceMode = useSettingsStore((state) => state.performanceMode);

    const handleImageClick = useCallback((e: React.MouseEvent | React.TouchEvent) => {
      e.stopPropagation();
      const rect = imgRef.current?.getBoundingClientRect();
      onAdd(product, rect, true);
    }, [onAdd, product]);

    const stopPropagation = useCallback((e: React.SyntheticEvent) => {
      e.stopPropagation();
    }, []);

    const handleCardClick = useCallback((e: React.MouseEvent | React.TouchEvent) => {
      const rect = imgRef.current?.getBoundingClientRect();
      onAdd(product, rect, false);
    }, [onAdd, product]);

    const handleLongPress = useCallback(() => {
      if (onLongPress) {
        onLongPress(product);
      }
    }, [onLongPress, product.id]);

    const longPressHandlers = useLongPress(handleLongPress, handleCardClick, {
      delay: 500,
      isPreventDefault: true
    });

    const [imageUrl] = useImageUrl(product.image);
    const imageSrc = imageUrl || DefaultImage;

    return (
      <div
        {...longPressHandlers}
        className="group relative min-h-[calc(96px+1.2rem)] max-h-[calc(120px+1.8rem)] bg-white rounded-xl shadow-sm border border-gray-200
        cursor-pointer overflow-hidden select-none flex touch-manipulation
        hover:shadow-md hover:border-blue-300
        active:scale-[0.96] active:shadow-inner active:border-blue-500 active:ring-4 active:ring-blue-500/10"
      >
        {/* Image Section - Compact & Full */}
        <div
          onClick={handleImageClick}
          onMouseDown={stopPropagation}
          onMouseUp={stopPropagation}
          onTouchStart={stopPropagation}
          onTouchEnd={stopPropagation}
          className="w-28 shrink-0 bg-white relative overflow-hidden border-r border-gray-100"
        >
          {!performanceMode ? (
            <img
              ref={imgRef}
              src={imageSrc}
              alt={product.name}
              className="absolute inset-0 w-full h-full object-cover object-center transition-transform duration-500 ease-out group-hover:scale-105 group-active:scale-100"
              loading={priority ? "eager" : "lazy"}
              decoding="async"
              onError={(e) => { (e.target as HTMLImageElement).src = DefaultImage; }}
            />
          ) : (
             <img
               ref={imgRef}
               src={DefaultImage}
               alt={product.name}
               className="absolute inset-0 w-full h-full object-cover object-center"
             />
          )}
          {product.external_id !== undefined && (
            <div className="absolute bottom-0 left-0 bg-gray-900/85 text-white text-sm font-bold font-mono px-2 py-1 rounded-tr-lg backdrop-blur-[1px] shadow-sm z-10 leading-none">
              {product.external_id}
            </div>
          )}
        </div>

        {/* Content Section */}
        <div className="flex-1 flex flex-col justify-between p-2 min-w-0 bg-white">
          {/* Header: Name */}
          <div className="w-full">
            <h3 className="font-bold text-gray-800 text-base leading-tight line-clamp-4 transition-colors wrap-break-word group-active:text-blue-600" title={product.name}>
              {product.name}
            </h3>
          </div>

          {/* Footer: Price & Tags */}
          <div className="flex items-end mt-1 w-full">
            <span className="ml-auto text-base font-bold text-rose-500 leading-none transition-transform group-active:scale-110 origin-right duration-200">
              {formatCurrency(product.price ?? 0)}
            </span>
          </div>
        </div>

        {/* Interactive Overlay - Ripple-like feedback */}
        <div className="absolute inset-0 bg-blue-500/0 group-active:bg-blue-500/5 pointer-events-none transition-colors duration-200" />
      </div>
    );
  },
  // Custom comparison function
  (prevProps, nextProps) => {
    return (
      prevProps.product.id === nextProps.product.id &&
      (prevProps.product.price ?? 0) === (nextProps.product.price ?? 0) &&
      prevProps.product.name === nextProps.product.name &&
      prevProps.product.image === nextProps.product.image &&
      prevProps.priority === nextProps.priority
    );
  }
);

ProductCard.displayName = 'ProductCard';
