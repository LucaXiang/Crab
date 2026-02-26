import React, { useEffect, useState } from 'react';
import { ImageOff } from 'lucide-react';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { getImageUrl } from '@/infrastructure/api/store';

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
      setSrc(null);
      return;
    }
    let cancelled = false;
    getImageUrl(token, hash)
      .then(url => { if (!cancelled) setSrc(url); })
      .catch(() => { if (!cancelled) setSrc(null); });
    return () => { cancelled = true; };
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
