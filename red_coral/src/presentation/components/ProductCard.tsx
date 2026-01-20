import React, { useRef, useCallback } from 'react';
import { convertFileSrc } from '@tauri-apps/api/core';
import { Product } from '@/core/domain/types';
import DefaultImage from '../../assets/reshot.svg';
import { ImageOff } from 'lucide-react';
import { useSettingsStore } from '@/core/stores/settings/useSettingsStore';
import { useLongPress } from '@/hooks/useLongPress';
import { formatCurrency } from '@/utils/currency';

/**
 * Extended Product type with computed fields from ProductSpecification
 * These fields are populated by the product store from the root spec
 */
export interface ProductWithPrice extends Product {
  price?: number;      // From root spec
  externalId?: number | string; // From root spec external_id
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
    }, [onAdd]);

    const stopPropagation = useCallback((e: React.SyntheticEvent) => {
      e.stopPropagation();
    }, []);

    const handleCardClick = useCallback((e: React.MouseEvent | React.TouchEvent) => {
      const rect = imgRef.current?.getBoundingClientRect();
      onAdd(product, rect, false);
    }, [onAdd]);

    const handleLongPress = useCallback(() => {
      if (onLongPress) {
        onLongPress(product);
      }
    }, [onLongPress, product.id]);

    const longPressHandlers = useLongPress(handleLongPress, handleCardClick, {
      delay: 500,
      isPreventDefault: true
    });

    const imageSrc = product.image 
      ? (/^(https?:\/\/|data:)/.test(product.image) ? product.image : convertFileSrc(product.image)) 
      : DefaultImage;

    return (
      <div
        {...longPressHandlers}
        className="group relative min-h-24 max-h-32 bg-white rounded-xl shadow-sm border border-gray-200
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
          className="w-24 shrink-0 bg-white relative overflow-hidden border-r border-gray-100"
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
             <div className="absolute inset-0 w-full h-full flex items-center justify-center bg-gray-50 text-gray-300">
               <ImageOff size={24} />
             </div>
          )}
          {product.externalId !== undefined && (
            <div className="absolute bottom-0 left-0 bg-gray-900/85 text-white text-[10px] font-bold font-mono px-1.5 py-0.5 rounded-tr-md backdrop-blur-[1px] shadow-sm z-10 leading-none">
              {product.externalId}
            </div>
          )}
        </div>

        {/* Content Section */}
        <div className="flex-1 flex flex-col justify-between p-2 min-w-0 bg-white">
          {/* Header: Name */}
          <div className="w-full">
            <h3 className="font-bold text-gray-800 text-xs leading-tight line-clamp-5 transition-colors wrap-break-word group-active:text-blue-600" title={product.name}>
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
