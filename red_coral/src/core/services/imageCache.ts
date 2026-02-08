/**
 * Image Cache Service
 *
 * 缓存图片 hash -> asset URL 映射，避免重复 IPC 调用。
 * 使用 Tauri asset 协议直接加载本地图片，利用浏览器原生缓存。
 */

import { convertFileSrc, invoke } from '@tauri-apps/api/core';
import { logger } from '@/utils/logger';

// 内存缓存: hash -> asset URL
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
 */
function extractHashFromPath(path: string): string | null {
  const match = path.match(/([a-f0-9]{64})\.[^.]+$/i);
  return match ? match[1] : null;
}

/**
 * 判断是否是外部 URL
 */
function isExternalUrl(value: string): boolean {
  return /^(https?:\/\/|data:|asset:)/.test(value);
}

/**
 * 获取缓存 key（始终用 hash）
 */
function getCacheKey(imageRef: string): string {
  if (isHash(imageRef)) {
    return imageRef;
  }
  const hash = extractHashFromPath(imageRef);
  return hash || imageRef;
}

/**
 * 获取图片的 asset URL
 * @param imageRef - hash、完整路径或外部 URL
 * @returns asset:// URL 或原始 URL
 */
export async function getImageUrl(imageRef: string | null | undefined): Promise<string> {
  if (!imageRef) return '';

  // 外部 URL 或已是 asset URL，直接返回
  if (isExternalUrl(imageRef)) {
    return imageRef;
  }

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
  const loadPromise = resolveAssetUrl(imageRef);
  pending.set(cacheKey, loadPromise);

  try {
    const result = await loadPromise;
    if (result) {
      cache.set(cacheKey, result);
    }
    return result;
  } finally {
    pending.delete(cacheKey);
  }
}

/**
 * 解析图片路径并转换为 asset URL
 */
async function resolveAssetUrl(imageRef: string): Promise<string> {
  try {
    let filePath: string;

    if (isHash(imageRef)) {
      // hash -> 调用 Tauri 获取完整路径
      filePath = await invoke<string>('get_image_path', { hash: imageRef });
    } else {
      // 已经是完整路径
      filePath = imageRef;
    }

    if (!filePath) {
      logger.warn('Empty path for image', { component: 'ImageCache', imageRef });
      return '';
    }

    // 直接返回 asset URL，利用浏览器原生缓存
    return convertFileSrc(filePath);
  } catch (error) {
    logger.error('Failed to resolve image', error, { component: 'ImageCache', imageRef });
    return '';
  }
}
