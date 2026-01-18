/**
 * Label Print Service
 * Handles label printing functionality
 */

import { invoke } from '@tauri-apps/api/core';
import type { LabelPrintConfig, LabelPrintJob } from '@/types/labelTemplate';
import type { HeldOrder } from '@/core/domain/types';

export interface LabelPrintService {
  printLabel(config: LabelPrintConfig): Promise<void>;
  printMultipleLabels(configs: LabelPrintConfig[]): Promise<void>;
  printOrderLabels(order: HeldOrder): Promise<void>;
  printItemsLabels(order: HeldOrder, items: any[]): Promise<void>;
  getPrintJobs(): Promise<LabelPrintJob[]>;
  cancelPrintJob(jobId: string): Promise<void>;
}

export const labelPrintService: LabelPrintService = {
  async printLabel(config: LabelPrintConfig): Promise<void> {
    await invoke('print_label', { config });
  },

  async printMultipleLabels(configs: LabelPrintConfig[]): Promise<void> {
    await invoke('print_multiple_labels', { configs });
  },

  async printOrderLabels(order: HeldOrder): Promise<void> {
    await invoke('print_order_labels', { order });
  },

  async printItemsLabels(order: HeldOrder, items: any[]): Promise<void> {
    await invoke('print_items_labels', { order, items });
  },

  async getPrintJobs(): Promise<LabelPrintJob[]> {
    return invoke('get_print_jobs');
  },

  async cancelPrintJob(jobId: string): Promise<void> {
    await invoke('cancel_print_job', { jobId });
  },
};

// Export class for direct instantiation
export const LabelPrintService = labelPrintService;

export default labelPrintService;
