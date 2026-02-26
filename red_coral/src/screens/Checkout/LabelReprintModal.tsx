import React, { useEffect, useState, useCallback, useMemo } from 'react';
import { X, Tag, RefreshCw } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { createTauriClient } from '@/infrastructure/api/tauri-client';
import type { LabelPrintRecord } from '@/core/domain/types/api';

interface LabelReprintModalProps {
  isOpen: boolean;
  orderId: string;
  onClose: () => void;
}

/** Group label records by product+spec for display */
interface LabelGroup {
  key: string;
  productName: string;
  kitchenName: string;
  specName: string | null;
  quantity: number;
  /** Take any record ID from this group for reprint API */
  representativeId: string;
  totalPrintCount: number;
}

export const LabelReprintModal: React.FC<LabelReprintModalProps> = ({ isOpen, orderId, onClose }) => {
  const { t } = useI18n();
  const [labelRecords, setLabelRecords] = useState<LabelPrintRecord[]>([]);
  const [loading, setLoading] = useState(false);
  const [reprintingKey, setReprintingKey] = useState<string | null>(null);

  const fetchData = useCallback(async () => {
    setLoading(true);
    try {
      const client = createTauriClient();
      const records = await client.getLabelRecordsForOrder(orderId);
      setLabelRecords(records);
    } catch (err) {
      logger.error('Failed to fetch label records', err);
    } finally {
      setLoading(false);
    }
  }, [orderId]);

  useEffect(() => {
    if (isOpen) fetchData();
  }, [isOpen, fetchData]);

  // Group by product_id + spec_name
  const groups = useMemo<LabelGroup[]>(() => {
    const map = new Map<string, LabelGroup>();
    for (const lr of labelRecords) {
      const key = `${lr.context.product_id}:${lr.context.spec_name ?? ''}`;
      const existing = map.get(key);
      if (existing) {
        existing.quantity += 1;
        existing.totalPrintCount += lr.print_count;
      } else {
        map.set(key, {
          key,
          productName: lr.context.product_name,
          kitchenName: lr.context.kitchen_name,
          specName: lr.context.spec_name ?? null,
          quantity: 1,
          representativeId: lr.id,
          totalPrintCount: lr.print_count,
        });
      }
    }
    return Array.from(map.values());
  }, [labelRecords]);

  const handleReprint = async (group: LabelGroup) => {
    setReprintingKey(group.key);
    try {
      const client = createTauriClient();
      await client.reprintLabelRecord(group.representativeId);
      toast.success(t('checkout.label_reprint.success'));
      await fetchData();
    } catch (err) {
      logger.error('Label reprint failed', err);
      toast.error(t('checkout.label_reprint.failed'));
    } finally {
      setReprintingKey(null);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50" onClick={onClose}>
      <div className="bg-white rounded-2xl shadow-2xl w-[600px] max-h-[80vh] flex flex-col" onClick={(e) => e.stopPropagation()}>
        {/* Header */}
        <div className="flex items-center justify-between p-5 border-b border-gray-200">
          <h2 className="text-xl font-bold text-gray-800">{t('checkout.label_reprint.tab_label')}</h2>
          <button onClick={onClose} className="p-2 rounded-lg hover:bg-gray-100 transition-colors">
            <X size={20} className="text-gray-500" />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5">
          {loading ? (
            <div className="flex items-center justify-center py-12">
              <RefreshCw size={24} className="animate-spin text-gray-400" />
            </div>
          ) : groups.length === 0 ? (
            <div className="text-center py-12 text-gray-400">
              {t('checkout.label_reprint.empty')}
            </div>
          ) : (
            <div className="space-y-3">
              {groups.map((g) => (
                <div key={g.key} className="flex items-center gap-4 p-4 bg-gray-50 rounded-xl">
                  <div className="flex-1 min-w-0">
                    <div className="flex items-center gap-2 mb-1">
                      <span className="text-sm font-medium text-gray-700">{g.kitchenName}</span>
                      <span className="text-sm text-gray-400">·</span>
                      <span className="text-sm font-medium text-gray-700">x{g.quantity}</span>
                      {g.totalPrintCount > 0 && (
                        <span className="text-xs bg-amber-100 text-amber-700 px-2 py-0.5 rounded-full font-medium">
                          {t('checkout.kitchen_reprint.print_count', { count: String(g.totalPrintCount) })}
                        </span>
                      )}
                    </div>
                    {g.specName && (
                      <div className="text-xs text-gray-400">{g.specName}</div>
                    )}
                  </div>
                  <button
                    onClick={() => handleReprint(g)}
                    disabled={reprintingKey === g.key}
                    className="flex items-center gap-2 px-4 py-2 bg-amber-500 hover:bg-amber-600 text-white rounded-lg font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed shrink-0"
                  >
                    <Tag size={16} className={reprintingKey === g.key ? 'animate-pulse' : ''} />
                    {t('checkout.label_reprint.button')}
                  </button>
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
