/**
 * Label Print Service
 *
 * NOTE: 标签打印功能现在由服务端 (edge-server) 处理。
 * 前端不再需要直接调用打印命令。
 * 保留此文件仅用于类型导出和向后兼容。
 */

import type { LabelPrintConfig, LabelPrintJob } from '@/core/domain/types/print';
import type { HeldOrder, CartItem } from '@/core/domain/types';

export interface LabelPrintService {
  printLabel(config: LabelPrintConfig): Promise<void>;
  printMultipleLabels(configs: LabelPrintConfig[]): Promise<void>;
  printOrderLabels(order: HeldOrder): Promise<void>;
  printItemsLabels(order: HeldOrder, items: CartItem[]): Promise<void>;
  getPrintJobs(): Promise<LabelPrintJob[]>;
  cancelPrintJob(jobId: string): Promise<void>;
}

// 标签打印功能已移至服务端，这些函数现在是空操作
const notImplemented = async () => {
  console.warn('[LabelPrintService] 标签打印功能已由服务端处理，前端调用无效');
};

export const labelPrintService: LabelPrintService = {
  printLabel: notImplemented,
  printMultipleLabels: notImplemented,
  printOrderLabels: notImplemented,
  printItemsLabels: notImplemented,
  getPrintJobs: async () => [],
  cancelPrintJob: notImplemented,
};

export default labelPrintService;
