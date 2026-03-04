import React from 'react';
import { useI18n } from '@/hooks/useI18n';
import { StoreOverviewDisplay } from './StoreOverviewDisplay';
import { RedFlagsBar, type RedFlagsData } from './RedFlagsBar';
import type { StoreOverview } from '@/core/domain/types';

interface OverviewProps {
  overview: StoreOverview;
  previousOverview?: StoreOverview | null;
  lastWeekOverview?: StoreOverview | null;
  cutoffMinutes?: number;
  redFlags?: RedFlagsData | null;
}

export const Overview: React.FC<OverviewProps> = ({
  overview,
  previousOverview,
  lastWeekOverview,
  cutoffMinutes,
  redFlags,
}) => {
  const { t } = useI18n();
  const hasData = overview.orders > 0 || overview.revenue_trend.length > 0;

  if (!hasData) {
    return (
      <div className="flex flex-col items-center justify-center py-20 bg-white rounded-xl border border-gray-100 shadow-sm">
        <div className="p-4 bg-gray-50 rounded-full mb-4">
          <svg className="w-12 h-12 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
          </svg>
        </div>
        <h3 className="text-lg font-medium text-gray-900 mb-1">{t('common.empty.no_data')}</h3>
        <p className="text-sm text-gray-500">{t('statistics.sidebar.analytics')}</p>
      </div>
    );
  }

  return (
    <>
      {redFlags && <RedFlagsBar data={redFlags} />}
      <StoreOverviewDisplay
        overview={overview}
        previousOverview={previousOverview}
        lastWeekOverview={lastWeekOverview}
        cutoffMinutes={cutoffMinutes}
      />
    </>
  );
};
