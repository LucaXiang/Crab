/**
 * Label Printer Store
 */

import { create } from 'zustand';

interface LabelPrinterState {
  selectedPrinterId: string | null;
  setSelectedPrinterId: (id: string | null) => void;
}

export const useLabelPrinter = create<LabelPrinterState>((set) => ({
  selectedPrinterId: null,

  setSelectedPrinterId: (id) => {
    set({ selectedPrinterId: id });
  },
}));
