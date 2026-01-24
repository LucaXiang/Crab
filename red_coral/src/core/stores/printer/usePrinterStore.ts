/**
 * Printer Store - 管理所有打印机配置
 */

import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';

interface PrinterStore {
  // State
  receiptPrinter: string | null;
  kitchenPrinter: string | null;
  isKitchenPrintEnabled: boolean;
  labelPrinter: string | null;
  isLabelPrintEnabled: boolean;
  activeLabelTemplateId: string | null;

  // Actions
  setReceiptPrinter: (name: string | null) => void;
  setKitchenPrinter: (name: string | null) => void;
  setIsKitchenPrintEnabled: (enabled: boolean) => void;
  setLabelPrinter: (name: string | null) => void;
  setIsLabelPrintEnabled: (enabled: boolean) => void;
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
  isKitchenPrintEnabled: getItem('kitchen_print_enabled') !== 'false',
  labelPrinter: getItem('printer_label'),
  isLabelPrintEnabled: getItem('label_print_enabled') !== 'false',
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

  setIsKitchenPrintEnabled: (enabled) => {
    setItem('kitchen_print_enabled', String(enabled));
    set({ isKitchenPrintEnabled: enabled });
  },

  setLabelPrinter: (name) => {
    setItem('printer_label', name);
    set({ labelPrinter: name });
  },

  setIsLabelPrintEnabled: (enabled) => {
    setItem('label_print_enabled', String(enabled));
    set({ isLabelPrintEnabled: enabled });
  },

  setActiveLabelTemplateId: (id) => {
    setItem('active_label_template_id', id);
    set({ activeLabelTemplateId: id });
  },
}));

// Selectors
export const useReceiptPrinter = () => usePrinterStore((state) => state.receiptPrinter);
export const useKitchenPrinter = () => usePrinterStore((state) => state.kitchenPrinter);
export const useIsKitchenPrintEnabled = () => usePrinterStore((state) => state.isKitchenPrintEnabled);
export const useLabelPrinter = () => usePrinterStore((state) => state.labelPrinter);
export const useIsLabelPrintEnabled = () => usePrinterStore((state) => state.isLabelPrintEnabled);
export const useActiveLabelTemplateId = () => usePrinterStore((state) => state.activeLabelTemplateId);

// Alias for backward compatibility
export const useSelectedPrinter = useReceiptPrinter;

// Actions hook
export const usePrinterActions = () => usePrinterStore(
  useShallow((state) => ({
    setReceiptPrinter: state.setReceiptPrinter,
    setKitchenPrinter: state.setKitchenPrinter,
    setIsKitchenPrintEnabled: state.setIsKitchenPrintEnabled,
    setLabelPrinter: state.setLabelPrinter,
    setIsLabelPrintEnabled: state.setIsLabelPrintEnabled,
    setActiveLabelTemplateId: state.setActiveLabelTemplateId,
  }))
);
