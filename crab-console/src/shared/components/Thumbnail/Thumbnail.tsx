import React, { useEffect, useState } from 'react';
import { ImageOff } from 'lucide-react';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getImageBlobUrl } from '@/infrastructure/api/store';

interface ThumbnailProps {
  hash: string;
  size?: number;
  className?: string;
}

export const Thumbnail: React.FC<ThumbnailProps> = ({ hash, size = 40, className }) => {
  const token = useAuthStore(s => s.token);
  const [src, setSrc] = useState<string | null>(null);

  useEffect(() => {
    if (!hash || !token) {
      setSrc(prev => { if (prev) URL.revokeObjectURL(prev); return null; });
      return;
    }
    let cancelled = false;
    getImageBlobUrl(token, hash)
      .then(url => {
        if (!cancelled) {
          setSrc(prev => { if (prev) URL.revokeObjectURL(prev); return url; });
        } else {
          URL.revokeObjectURL(url);
        }
      })
      .catch(() => { if (!cancelled) setSrc(null); });
    return () => {
      cancelled = true;
      setSrc(prev => { if (prev) URL.revokeObjectURL(prev); return null; });
    };
  }, [hash, token]);

  const style = { width: size, height: size, minWidth: size };

  if (!hash) {
    return (
      <div style={style} className={`rounded-lg bg-gray-100 flex items-center justify-center ${className ?? ''}`}>
        <ImageOff size={size * 0.4} className="text-gray-300" />
      </div>
    );
  }

  if (!src) {
    return <div style={style} className={`rounded-lg bg-gray-100 animate-pulse ${className ?? ''}`} />;
  }

  return (
    <img
      src={src}
      alt=""
      style={style}
      className={`rounded-lg object-cover ${className ?? ''}`}
    />
  );
};
