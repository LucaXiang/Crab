/**
 * ProductImage Component
 *
 * 统一的图片显示组件，处理 hash -> base64 转换
 */

import { memo } from 'react';
import { useImageUrl } from '@/core/hooks';
import DefaultImage from '@/assets/reshot.svg';

interface ProductImageProps {
  src: string | null | undefined;
  alt?: string;
  className?: string;
  fallback?: React.ReactNode;
  onClick?: () => void;
}

export const ProductImage = memo(function ProductImage({
  src,
  alt = '',
  className = '',
  fallback,
  onClick,
}: ProductImageProps) {
  const [url, loading] = useImageUrl(src);

  if (loading) {
    return (
      <div className={`bg-gray-200 animate-pulse ${className}`} />
    );
  }

  if (!url) {
    return fallback ? <>{fallback}</> : (
      <div className={`bg-gray-50 flex items-center justify-center ${className}`}>
        <img src={DefaultImage} alt={alt} className="w-full h-full object-contain p-2" />
      </div>
    );
  }

  return (
    <img
      src={url}
      alt={alt}
      className={className}
      onClick={onClick}
      loading="lazy"
    />
  );
});
