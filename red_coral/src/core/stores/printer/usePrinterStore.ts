/**
 * Printer Store - 管理所有打印机配置
 */

import { create } from 'zustand';
import { useShallow } from 'zustand/react/shallow';

interface PrinterStore {
  // State
  receiptPrinter: string | null;
  labelPrinter: string | null;
  cashDrawerPrinter: string | null;
  activeLabelTemplateId: number | null;
  autoOpenCashDrawerAfterReceipt: boolean;

  // Actions
  setReceiptPrinter: (name: string | null) => void;
  setLabelPrinter: (name: string | null) => void;
  setCashDrawerPrinter: (name: string | null) => void;
  setActiveLabelTemplateId: (id: number | null) => void;
  setAutoOpenCashDrawerAfterReceipt: (enabled: boolean) => void;
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
  labelPrinter: getItem('printer_label'),
  cashDrawerPrinter: getItem('printer_cash_drawer'),
  activeLabelTemplateId: (() => { const v = getItem('active_label_template_id'); return v ? Number(v) : null; })(),
  autoOpenCashDrawerAfterReceipt: getItem('auto_open_cash_drawer_after_receipt') === 'true',

  // Actions
  setReceiptPrinter: (name) => {
    setItem('printer_receipt', name);
    set({ receiptPrinter: name });
  },

  setLabelPrinter: (name) => {
    setItem('printer_label', name);
    set({ labelPrinter: name });
  },

  setCashDrawerPrinter: (name) => {
    setItem('printer_cash_drawer', name);
    set({ cashDrawerPrinter: name });
  },

  setActiveLabelTemplateId: (id) => {
    setItem('active_label_template_id', id != null ? String(id) : null);
    set({ activeLabelTemplateId: id });
  },

  setAutoOpenCashDrawerAfterReceipt: (enabled) => {
    setItem('auto_open_cash_drawer_after_receipt', enabled ? 'true' : null);
    set({ autoOpenCashDrawerAfterReceipt: enabled });
  },
}));

// Selectors
export const useReceiptPrinter = () => usePrinterStore((state) => state.receiptPrinter);
export const useLabelPrinter = () => usePrinterStore((state) => state.labelPrinter);
export const useCashDrawerPrinter = () => usePrinterStore((state) => state.cashDrawerPrinter);
export const useActiveLabelTemplateId = () => usePrinterStore((state) => state.activeLabelTemplateId);
export const useAutoOpenCashDrawerAfterReceipt = () => usePrinterStore((state) => state.autoOpenCashDrawerAfterReceipt);


// Actions hook
export const usePrinterActions = () => usePrinterStore(
  useShallow((state) => ({
    setReceiptPrinter: state.setReceiptPrinter,
    setLabelPrinter: state.setLabelPrinter,
    setCashDrawerPrinter: state.setCashDrawerPrinter,
    setActiveLabelTemplateId: state.setActiveLabelTemplateId,
    setAutoOpenCashDrawerAfterReceipt: state.setAutoOpenCashDrawerAfterReceipt,
  }))
);
