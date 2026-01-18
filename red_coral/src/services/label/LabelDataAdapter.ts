/**
 * Label Data Adapter
 * Adapts data for label printing
 */

import type { LabelField } from '@/types/labelTemplate';

export interface LabelData {
  product?: {
    name: string;
    price: number;
    externalId?: string;
  };
  specification?: {
    name: string;
  };
  order?: {
    receiptNumber: string;
    tableName?: string;
  };
  item?: {
    quantity: number;
    productName: string;
    options?: string;
  };
  print?: {
    time: string;
  };
}

export const LabelDataAdapter = {
  /**
   * Adapt product data for label template
   */
  adaptProductData(product: {
    name: string;
    price: number;
    externalId?: string;
  }): Partial<LabelData> {
    return {
      product: {
        name: product.name,
        price: product.price,
        externalId: product.externalId,
      },
    };
  },

  /**
   * Adapt order data for label template
   */
  adaptOrderData(order: {
    receiptNumber: string;
    tableName?: string;
    items: Array<{
      quantity: number;
      productName: string;
      options?: string;
    }>;
  }): Partial<LabelData> {
    return {
      order: {
        receiptNumber: order.receiptNumber,
        tableName: order.tableName,
      },
      item: order.items[0] ? {
        quantity: order.items[0].quantity,
        productName: order.items[0].productName,
        options: order.items[0].options,
      } : undefined,
    };
  },

  /**
   * Get all data sources used in a template
   */
  getDataSources(fields: LabelField[]): string[] {
    const sources = new Set<string>();
    fields.forEach(field => {
      if (field.dataSource) {
        sources.add(field.dataSource.split('.')[0]);
      }
    });
    return Array.from(sources);
  },
};

export default LabelDataAdapter;
