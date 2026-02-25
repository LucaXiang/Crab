import React, { useCallback, useEffect, useRef, useState } from 'react';
import { ImagePlus, X, Loader2 } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useAuthStore } from '@/core/stores/useAuthStore';
import { uploadImage, getImageBlobUrl } from '@/infrastructure/api/store';

interface ImageUploadProps {
  /** Current image hash (from product.image) */
  value: string;
  /** Called with new hash after successful upload, or '' to clear */
  onChange: (hash: string) => void;
  /** Optional class for the container */
  className?: string;
}

export const ImageUpload: React.FC<ImageUploadProps> = ({ value, onChange, className }) => {
  const { t } = useI18n();
  const token = useAuthStore(s => s.token);
  const fileRef = useRef<HTMLInputElement>(null);

  const [blobUrl, setBlobUrl] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [error, setError] = useState('');

  // Load existing image from hash
  useEffect(() => {
    if (!value || !token) {
      setBlobUrl(prev => { if (prev) URL.revokeObjectURL(prev); return null; });
      return;
    }
    let cancelled = false;
    setLoading(true);
    getImageBlobUrl(token, value)
      .then(url => {
        if (!cancelled) {
          setBlobUrl(prev => { if (prev) URL.revokeObjectURL(prev); return url; });
        } else {
          URL.revokeObjectURL(url);
        }
      })
      .catch(() => { if (!cancelled) setBlobUrl(null); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
  }, [value, token]);

  const handleFile = useCallback(async (file: File) => {
    if (!token) return;
    if (!file.type.startsWith('image/')) {
      setError(t('common.error.invalid_image'));
      return;
    }
    if (file.size > 20 * 1024 * 1024) {
      setError(t('common.error.file_too_large'));
      return;
    }

    setUploading(true);
    setError('');
    try {
      const hash = await uploadImage(token, file);
      const localUrl = URL.createObjectURL(file);
      setBlobUrl(prev => { if (prev) URL.revokeObjectURL(prev); return localUrl; });
      onChange(hash);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('auth.error_generic'));
    } finally {
      setUploading(false);
    }
  }, [token, onChange, t]);

  const handleInputChange = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) handleFile(file);
    // Reset input so same file can be selected again
    e.target.value = '';
  }, [handleFile]);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    const file = e.dataTransfer.files?.[0];
    if (file) handleFile(file);
  }, [handleFile]);

  const handleClear = useCallback(() => {
    if (blobUrl) URL.revokeObjectURL(blobUrl);
    setBlobUrl(null);
    onChange('');
    setError('');
  }, [blobUrl, onChange]);

  const showPreview = blobUrl && !loading;

  return (
    <div className={className}>
      <input
        ref={fileRef}
        type="file"
        accept="image/*"
        onChange={handleInputChange}
        className="hidden"
      />

      {showPreview ? (
        /* ── Preview mode ── */
        <div className="relative group">
          <img
            src={blobUrl}
            alt=""
            className="w-full h-40 object-cover rounded-xl border border-gray-200"
          />
          <div className="absolute inset-0 bg-black/40 opacity-0 group-hover:opacity-100 transition-opacity rounded-xl flex items-center justify-center gap-3">
            <button
              type="button"
              onClick={() => fileRef.current?.click()}
              className="px-3 py-1.5 bg-white/90 text-gray-800 rounded-lg text-xs font-medium hover:bg-white transition-colors"
            >
              {t('common.action.change')}
            </button>
            <button
              type="button"
              onClick={handleClear}
              className="p-1.5 bg-white/90 text-red-600 rounded-lg hover:bg-white transition-colors"
            >
              <X size={14} />
            </button>
          </div>
        </div>
      ) : (
        /* ── Upload zone ── */
        <button
          type="button"
          onClick={() => fileRef.current?.click()}
          onDragOver={e => e.preventDefault()}
          onDrop={handleDrop}
          disabled={uploading}
          className="w-full h-32 border-2 border-dashed border-gray-200 rounded-xl flex flex-col items-center justify-center gap-2 text-gray-400 hover:border-blue-300 hover:text-blue-500 hover:bg-blue-50/30 transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-wait"
        >
          {uploading || loading ? (
            <Loader2 size={24} className="animate-spin" />
          ) : (
            <>
              <ImagePlus size={24} />
              <span className="text-xs">{t('common.action.upload_image')}</span>
            </>
          )}
        </button>
      )}

      {error && (
        <p className="text-xs text-red-500 mt-1.5">{error}</p>
      )}
    </div>
  );
};
