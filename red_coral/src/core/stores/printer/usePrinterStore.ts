/**
 * Printer Store - 管理所有打印机配置
 */

import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';

interface PrinterStore {
  // State
  receiptPrinter: string | null;
  kitchenPrinter: string | null;
  labelPrinter: string | null;
  activeLabelTemplateId: string | null;

  // Actions
  setReceiptPrinter: (name: string | null) => void;
  setKitchenPrinter: (name: string | null) => void;
  setLabelPrinter: (name: string | null) => void;
  setActiveLabelTemplateId: (id: string | null) => void;
}

const getItem = (key: string): string | null => {
  if (typeof window !== 'undefined' && typeof localStorage?.getItem === 'function') {
    return localStorage.getItem(key);
  }
  return null;
};

const setItem = (key: string, value: string | null) => {
  if (typeof window !== 'undefined') {
    if (value) localStorage.setItem(key, value);
    else localStorage.removeItem(key);
  }
};

export const usePrinterStore = create<PrinterStore>((set) => ({
  // Initial State (from localStorage)
  receiptPrinter: getItem('printer_receipt'),
  kitchenPrinter: getItem('printer_kitchen'),
  labelPrinter: getItem('printer_label'),
  activeLabelTemplateId: getItem('active_label_template_id'),

  // Actions
  setReceiptPrinter: (name) => {
    setItem('printer_receipt', name);
    set({ receiptPrinter: name });
  },

  setKitchenPrinter: (name) => {
    setItem('printer_kitchen', name);
    set({ kitchenPrinter: name });
  },

  setLabelPrinter: (name) => {
    setItem('printer_label', name);
    set({ labelPrinter: name });
  },

  setActiveLabelTemplateId: (id) => {
    setItem('active_label_template_id', id);
    set({ activeLabelTemplateId: id });
  },
}));

// Selectors
export const useReceiptPrinter = () => usePrinterStore((state) => state.receiptPrinter);
export const useKitchenPrinter = () => usePrinterStore((state) => state.kitchenPrinter);
export const useLabelPrinter = () => usePrinterStore((state) => state.labelPrinter);
export const useActiveLabelTemplateId = () => usePrinterStore((state) => state.activeLabelTemplateId);

// Alias for backward compatibility
export const useSelectedPrinter = useReceiptPrinter;

// Actions hook
export const usePrinterActions = () => usePrinterStore(
  useShallow((state) => ({
    setReceiptPrinter: state.setReceiptPrinter,
    setKitchenPrinter: state.setKitchenPrinter,
    setLabelPrinter: state.setLabelPrinter,
    setActiveLabelTemplateId: state.setActiveLabelTemplateId,
  }))
);
