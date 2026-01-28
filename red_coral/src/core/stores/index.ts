/**
 * Core Stores
 *
 * 新架构: 使用 resources/ 下的统一 Store（服务器权威模型）
 * - 直接从 '@/core/stores/resources' 导入
 * - 或使用别名: import { storeRegistry } from '@/core/stores/resources'
 */

// Domain stores
export * from './order';
export * from './ui';
export * from './settings';

// 新架构 - 仅导出不冲突的部分
export {
  storeRegistry,
  getLoadedStores,
  refreshAllLoadedStores,
  clearAllStores,
} from './resources/registry';
