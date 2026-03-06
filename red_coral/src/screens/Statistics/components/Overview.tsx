import React from 'react';
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
