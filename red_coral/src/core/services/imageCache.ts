/**
 * Image Cache Service
 *
 * 缓存图片为 base64，避免重复 IPC 调用。
 * - hash -> base64 data URL
 * - 自动处理 hash 到路径的转换
 */

import { convertFileSrc } from '@tauri-apps/api/core';
import { invokeApi } from '@/infrastructure/api/tauri-client';

// 内存缓存: hash -> base64 data URL
const cache = new Map<string, string>();

// 正在加载的 Promise，防止重复请求
const pending = new Map<string, Promise<string>>();

/**
 * 判断是否是 hash（64位十六进制）
 */
function isHash(value: string): boolean {
  return /^[a-f0-9]{64}$/i.test(value);
}

/**
 * 从路径中提取 hash（文件名去掉扩展名）
 * 例如: /path/to/abc123def456.jpg -> abc123def456
 */
function extractHashFromPath(path: string): string | null {
  const match = path.match(/([a-f0-9]{64})\.[^.]+$/i);
  return match ? match[1] : null;
}

/**
 * 判断是否是外部 URL
 */
function isExternalUrl(value: string): boolean {
  return /^(https?:\/\/|data:)/.test(value);
}

/**
 * 获取缓存 key（始终用 hash）
 */
function getCacheKey(imageRef: string): string {
  if (isHash(imageRef)) {
    return imageRef;
  }
  // 尝试从路径提取 hash
  const hash = extractHashFromPath(imageRef);
  return hash || imageRef;
}

/**
 * 获取图片的 base64 URL
 * @param imageRef - hash、完整路径或外部 URL
 * @returns base64 data URL 或原始 URL
 */
export async function getImageUrl(imageRef: string | null | undefined): Promise<string> {
  if (!imageRef) return '';

  // 外部 URL 直接返回
  if (isExternalUrl(imageRef)) {
    return imageRef;
  }

  // 用 hash 作为缓存 key
  const cacheKey = getCacheKey(imageRef);

  // 检查缓存
  if (cache.has(cacheKey)) {
    return cache.get(cacheKey)!;
  }

  // 检查是否正在加载
  if (pending.has(cacheKey)) {
    return pending.get(cacheKey)!;
  }

  // 开始加载
  const loadPromise = loadImage(imageRef);
  pending.set(cacheKey, loadPromise);

  try {
    const result = await loadPromise;
    cache.set(cacheKey, result);
    return result;
  } finally {
    pending.delete(cacheKey);
  }
}

/**
 * 加载图片并转为 base64
 */
async function loadImage(imageRef: string): Promise<string> {
  try {
    let filePath: string;

    if (isHash(imageRef)) {
      // hash -> 调用 Tauri 获取完整路径
      filePath = await invokeApi<string>('get_image_path', { hash: imageRef });
    } else {
      // 已经是完整路径
      filePath = imageRef;
    }

    if (!filePath) {
      console.warn('[ImageCache] Empty path for:', imageRef);
      return '';
    }

    // 转换为 asset URL 并 fetch
    const assetUrl = convertFileSrc(filePath);
    const response = await fetch(assetUrl);

    if (!response.ok) {
      throw new Error(`Failed to fetch: ${response.status}`);
    }

    const blob = await response.blob();

    // 转换为 base64 data URL
    return new Promise((resolve, reject) => {
      const reader = new FileReader();
      reader.onloadend = () => resolve(reader.result as string);
      reader.onerror = reject;
      reader.readAsDataURL(blob);
    });
  } catch (error) {
    console.error('[ImageCache] Failed to load image:', imageRef, error);
    return '';
  }
}

/**
 * 批量预加载图片
 */
export async function preloadImages(imageRefs: (string | null | undefined)[]): Promise<void> {
  const validRefs = imageRefs.filter((ref): ref is string =>
    !!ref && !isExternalUrl(ref) && !cache.has(ref)
  );

  if (validRefs.length === 0) return;

  await Promise.all(validRefs.map(ref => getImageUrl(ref)));
}

/**
 * 清除缓存
 */
export function clearImageCache(): void {
  cache.clear();
}

/**
 * 获取缓存大小
 */
export function getImageCacheSize(): number {
  return cache.size;
}
