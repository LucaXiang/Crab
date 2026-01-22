/**
 * Label Print Service
 * Handles label printing functionality
 */

import { invokeApi } from '@/infrastructure/api/tauri-client';
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

export const labelPrintService: LabelPrintService = {
  printLabel: (config) => invokeApi('print_label', { config }),
  printMultipleLabels: (configs) => invokeApi('print_multiple_labels', { configs }),
  printOrderLabels: (order) => invokeApi('print_order_labels', { order }),
  printItemsLabels: (order, items) => invokeApi('print_items_labels', { order, items }),
  getPrintJobs: () => invokeApi('get_print_jobs'),
  cancelPrintJob: (jobId) => invokeApi('cancel_print_job', { job_id: jobId }),
};

export default labelPrintService;
