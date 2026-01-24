import React from 'react';
import { PlusSquare } from 'lucide-react';
import { ProductCard } from '@/features/product';
import { useI18n } from '@/hooks/useI18n';
import { Product } from '@/core/domain/types';

interface ProductGridProps {
  products: Product[];
  isLoading: boolean;
  onAdd: (product: Product, startRect?: DOMRect, skipQuickAdd?: boolean) => void;
  onLongPress?: (product: Product) => void;
  className?: string;
}

const ProductGridInner: React.FC<ProductGridProps> = ({
  products,
  isLoading,
  onAdd,
  onLongPress,
  className,
}) => {
  const { t } = useI18n();

  return (
    <div className="relative flex-1 overflow-y-auto p-4 custom-scrollbar">
      <div className={`grid gap-3 ${className || 'grid-cols-[repeat(auto-fill,minmax(12.5rem,1fr))]'}`}>
        {isLoading ? (
	          <div className="col-span-full flex flex-col items-center justify-center h-64 text-gray-400">
	            <div className="animate-spin w-8 h-8 border-2 border-coral-500 border-t-transparent rounded-full mb-2" />
	            <p>{t('app.status.loading')}</p>
	          </div>
	        ) : products.length === 0 ? (
	          <div className="col-span-full flex flex-col items-center justify-center h-64 text-gray-400">
	            <div className="w-16 h-16 border-2 border-gray-200 border-dashed rounded-xl mb-2 flex items-center justify-center">
	              <PlusSquare className="opacity-20" size={32} />
	            </div>
	            <p>{t('app.empty_category')}</p>
	          </div>
	        ) : (
          products.map((product, index) => (
            <ProductCard
              key={product.id ?? `product-${index}`}
              product={product}
              onAdd={onAdd}
              onLongPress={onLongPress}
              priority={index < 12} // Eager load first 12 images (approx. 3 rows)
            />
          ))
        )}
	      </div>
	    </div>
	  );
	};

ProductGridInner.displayName = 'ProductGrid';

export const ProductGrid: React.FC<ProductGridProps> = React.memo(
	ProductGridInner,
	(prevProps, nextProps) => {
		return (
			prevProps.products === nextProps.products &&
			prevProps.isLoading === nextProps.isLoading &&
			prevProps.onAdd === nextProps.onAdd &&
      prevProps.onLongPress === nextProps.onLongPress &&
      prevProps.className === nextProps.className
    );
  }
);
