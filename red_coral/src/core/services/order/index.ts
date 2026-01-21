/**
 * Order Services - Re-export for backward compatibility
 */
// TODO: refactor eventReducer to use new event system
// export { reduceOrderEvents, recalculateOrderTotal, createEmptyOrder } from './eventReducer';
export { saveCompletedOrder } from './eventPersistence';
export { openCashDrawer, printOrderReceipt } from './paymentService';
