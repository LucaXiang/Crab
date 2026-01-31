/**
 * Print Service - Re-exports from printService
 */

export { printService, printService as default } from './printService';
export type { PrintService } from './printService';

// Re-export commonly used functions
export {
  printReceipt,
  reprintReceipt,
  printKitchenTicket,
  openCashDrawer,
  listPrinters,
} from './printService';
