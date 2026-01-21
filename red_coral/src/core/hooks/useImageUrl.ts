/**
 * useImageUrl Hook
 *
 * 将图片 hash 转换为可显示的 URL（base64 缓存）
 */

import { useState, useEffect } from 'react';
import { getImageUrl } from '@/core/services/imageCache';

/**
 * 获取图片 URL
 * @param imageRef - hash、完整路径或外部 URL
 * @returns [url, loading]
 */
export function useImageUrl(imageRef: string | null | undefined): [string, boolean] {
  const [url, setUrl] = useState<string>('');
  const [loading, setLoading] = useState(false);

  useEffect(() => {
    if (!imageRef) {
      setUrl('');
      return;
    }

    // 外部 URL 直接设置
    if (/^(https?:\/\/|data:)/.test(imageRef)) {
      setUrl(imageRef);
      return;
    }

    let cancelled = false;
    setLoading(true);

    getImageUrl(imageRef).then((result) => {
      if (!cancelled) {
        setUrl(result);
        setLoading(false);
      }
    });

    return () => {
      cancelled = true;
    };
  }, [imageRef]);

  return [url, loading];
}

/**
 * 批量获取图片 URL
 */
export function useImageUrls(imageRefs: (string | null | undefined)[]): Map<string, string> {
  const [urls, setUrls] = useState<Map<string, string>>(new Map());

  useEffect(() => {
    const validRefs = imageRefs.filter((ref): ref is string => !!ref);
    if (validRefs.length === 0) return;

    let cancelled = false;

    Promise.all(
      validRefs.map(async (ref) => {
        const url = await getImageUrl(ref);
        return [ref, url] as const;
      })
    ).then((results) => {
      if (!cancelled) {
        setUrls(new Map(results));
      }
    });

    return () => {
      cancelled = true;
    };
  }, [imageRefs.join(',')]);

  return urls;
}
