import React, { useState, useEffect, useCallback } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { toast } from '@/presentation/components/Toast';
import { logger } from '@/utils/logger';
import { invokeApi } from '@/infrastructure/api/tauri-client';
import { StoreOverviewDisplay } from './StoreOverviewDisplay';
import { RedFlagsBar, type RedFlagsData } from './RedFlagsBar';
import { TimeRangeSelector, useTimeRange, previousRange, lastWeekRange, useCutoffMinutes } from './TimeRangeSelector';
import type { StoreOverview } from '@/core/domain/types';

const EMPTY_OVERVIEW: StoreOverview = {
  revenue: 0, net_revenue: 0, orders: 0, guests: 0, average_order_value: 0,
  per_guest_spend: 0, average_dining_minutes: 0,
  total_tax: 0, total_discount: 0, total_surcharge: 0,
  avg_items_per_order: 0, voided_orders: 0, voided_amount: 0,
  loss_orders: 0, loss_amount: 0, anulacion_count: 0, anulacion_amount: 0,
  refund_count: 0, refund_amount: 0,
  revenue_trend: [], daily_trend: [], payment_breakdown: [],
  tax_breakdown: [], category_sales: [], top_products: [],
  tag_sales: [], refund_method_breakdown: [], service_type_breakdown: [],
  zone_sales: [], discount_breakdown: [], surcharge_breakdown: [],
};

export const OverviewTab: React.FC = () => {
  const { t } = useI18n();
  const [range, setRange] = useTimeRange();
  const cutoffMinutes = useCutoffMinutes();

  const [data, setData] = useState<StoreOverview>(EMPTY_OVERVIEW);
  const [prevData, setPrevData] = useState<StoreOverview | null>(null);
  const [lastWeekData, setLastWeekData] = useState<StoreOverview | null>(null);
  const [redFlags, setRedFlags] = useState<RedFlagsData | null>(null);

  const fetchOverview = useCallback(async (from: number, to: number): Promise<StoreOverview> => {
    return invokeApi<StoreOverview>('get_statistics', { from, to });
  }, []);

  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      try {
        const prev = previousRange(range);
        const lw = lastWeekRange(range);
        const [current, prevResult, lwResult, flags] = await Promise.all([
          fetchOverview(range.from, range.to),
          fetchOverview(prev.from, prev.to).catch(() => null),
          range.preset === 'today'
            ? fetchOverview(lw.from, lw.to).catch(() => null)
            : Promise.resolve(null),
          invokeApi<RedFlagsData>('get_red_flags', { from: range.from, to: range.to }).catch(() => null),
        ]);
        if (cancelled) return;
        setData(current);
        setPrevData(prevResult);
        setLastWeekData(lwResult);
        setRedFlags(flags);
      } catch (error) {
        if (cancelled) return;
        logger.error('Failed to fetch statistics', error);
        toast.error(t('statistics.error.load'));
      }
    };

    load();
    return () => { cancelled = true; };
  }, [range, fetchOverview, t]);

  return (
    <>
      <TimeRangeSelector value={range} onChange={setRange} />
      {redFlags && <RedFlagsBar data={redFlags} />}
      <StoreOverviewDisplay
        overview={data}
        previousOverview={prevData}
        lastWeekOverview={lastWeekData}
        cutoffMinutes={cutoffMinutes}
      />
    </>
  );
};
