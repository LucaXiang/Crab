/**
 * Domain Types
 *
 * This file re-exports types from the API infrastructure.
 * These types are aligned with the Rust crab-edge-server backend.
 */

export * from '@/infrastructure/api/types';

// Frontend-specific types for cart and orders
export interface CartItem {
  id: string;
  instanceId?: string;
  originalInstanceId?: string;
  productId: number;
  specificationId?: number;
  name: string;
  price: number;
  originalPrice?: number;
  quantity: number;
  note?: string;
  attributes?: ItemAttributeSelection[];
  selectedOptions?: ItemAttributeSelection[];
  selectedSpecification?: {
    id: string;
    name: string;
    receiptName?: string;
    price?: number;
  };
  _removed?: boolean;
  discountPercent?: number;
  surcharge?: number;
  externalId?: string;
  authorizerId?: string;
  authorizerName?: string;
}

export interface ItemAttributeSelection {
  attribute_id: number;
  option_id: number;
  name: string;
  value: string;
  price_modifier?: number;
  // Frontend display fields
  attribute_name?: string;
  attribute_receipt_name?: string | null;
  kitchen_printer_id?: number | null;
  receipt_name?: string | null;
  option_name?: string;  // Added for option name display
}

export interface PaymentRecord {
  id: string;
  amount: number;
  method: string;
  timestamp: number;
  note?: string;
  tendered?: number;
  change?: number;
}

export interface SurchargeInfo {
  type: 'percentage' | 'fixed';
  amount: number;
  total: number;
  value?: number;
  name?: string;
}

export type OrderStatus = 'ACTIVE' | 'COMPLETED' | 'VOID' | 'MOVED' | 'MERGED';

export interface HeldOrder {
  id?: string;
  key?: string;
  tableKey?: string;
  tableId?: number;
  tableName?: string;
  zoneId?: number;
  zoneName?: string;
  guestCount?: number;
  items: CartItem[];
  subtotal: number;
  tax: number;
  discount: number;
  surcharge?: SurchargeInfo;
  surchargeExempt?: boolean;
  total: number;
  paidAmount?: number;
  paidItemQuantities?: Record<string, number>;
  payments: PaymentRecord[];
  note?: string;
  receiptNumber?: string;
  isPrePayment?: boolean;
  isRetail?: boolean;
  status?: OrderStatus;
  startTime?: number;
  endTime?: number;
  timeline: TimelineEvent[];
  createdAt: number;
  updatedAt: number;
}

export interface TimelineEvent {
  id?: string;
  type: 'ITEM_ADDED' | 'ITEM_REMOVED' | 'QUANTITY_CHANGED' | 'NOTE_ADDED' | 'ORDER_CREATED' | 'PAYMENT_ADDED' | 'STATUS_CHANGED';
  timestamp: number;
  data: Record<string, unknown>;
  userId?: number;
  title?: string;
  summary?: string;
}

export type CheckoutMode = 'retail' | 'dine-in' | 'takeout';
export type DetailTab = 'items' | 'payments' | 'timeline';
export interface PendingCashTx {
  id: string;
  amount: number;
  timestamp: number;
}

export type DraftOrder = HeldOrder;
export type CompletedOrder = HeldOrder;

// Legacy type aliases for backward compatibility
export type User = import('@/infrastructure/api/types').User;
export type Product = import('@/infrastructure/api/types').Product;
export type Category = import('@/infrastructure/api/types').Category;
export type Table = import('@/infrastructure/api/types').Table;
export type Zone = import('@/infrastructure/api/types').Zone;
export type KitchenPrinter = import('@/infrastructure/api/types').KitchenPrinter;
export type Tag = import('@/infrastructure/api/types').Tag;
export type Role = import('@/infrastructure/api/types').Role;
export type AttributeTemplate = import('@/infrastructure/api/types').AttributeTemplate;
export type AttributeOption = import('@/infrastructure/api/types').AttributeOption;
export type ProductSpecification = import('@/infrastructure/api/types').ProductSpecification;
export type ProductAttribute = import('@/infrastructure/api/types').ProductAttribute;
export type CategoryAttribute = import('@/infrastructure/api/types').CategoryAttribute;
export type PriceAdjustmentRule = import('@/infrastructure/api/types').PriceAdjustmentRule;
export type Order = import('@/infrastructure/api/types').Order;
export type OrderItem = import('@/infrastructure/api/types').OrderItem;
export type OrderEvent = import('@/infrastructure/api/types').OrderEvent;
export type Payment = import('@/infrastructure/api/types').Payment;

// Request/Response types
export type LoginRequest = import('@/infrastructure/api/types').LoginRequest;
export type CreateProductRequest = import('@/infrastructure/api/types').CreateProductRequest;
export type ProductQuery = import('@/infrastructure/api/types').ProductQuery;
export type CreateTagRequest = import('@/infrastructure/api/types').CreateTagRequest;
export type UpdateTagRequest = import('@/infrastructure/api/types').UpdateTagRequest;
export type Permission = string; // Simplified permission type

// Permission constants
export const Permission = {
  MANAGE_USERS: 'manage_users' as Permission,
  VOID_ORDER: 'void_order' as Permission,
  RESTORE_ORDER: 'restore_order' as Permission,
  MANAGE_PRODUCTS: 'manage_products' as Permission,
  CREATE_PRODUCT: 'create_product' as Permission,
  UPDATE_PRODUCT: 'update_product' as Permission,
  DELETE_PRODUCT: 'delete_product' as Permission,
  MANAGE_CATEGORIES: 'manage_categories' as Permission,
  MANAGE_ZONES: 'manage_zones' as Permission,
  MANAGE_TABLES: 'manage_tables' as Permission,
  MODIFY_PRICE: 'modify_price' as Permission,
  APPLY_DISCOUNT: 'apply_discount' as Permission,
  VIEW_STATISTICS: 'view_statistics' as Permission,
  MANAGE_PRINTERS: 'manage_printers' as Permission,
  MANAGE_ATTRIBUTES: 'manage_attributes' as Permission,
  MANAGE_SETTINGS: 'manage_settings' as Permission,
  PRINT_RECEIPTS: 'print_receipts' as Permission,
  REFUND: 'refund' as Permission,
  DISCOUNT: 'discount' as Permission,
  CANCEL_ITEM: 'cancel_item' as Permission,
} as const;
