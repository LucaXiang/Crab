import React from 'react';
import { Calendar } from 'lucide-react';
import type { PriceRule } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';

interface TimeVisualizationProps {
  rule: PriceRule;
}

// Day keys for i18n (Sunday = 0)
const DAY_KEYS = ['sunday', 'monday', 'tuesday', 'wednesday', 'thursday', 'friday', 'saturday'] as const;

// Week order starting from Monday: [1,2,3,4,5,6,0]
const WEEK_ORDER = [1, 2, 3, 4, 5, 6, 0];

export const TimeVisualization: React.FC<TimeVisualizationProps> = ({ rule }) => {
  const { t, locale } = useI18n();
  const isDiscount = rule.rule_type === 'DISCOUNT';

  // Check if time range crosses midnight (e.g., 21:00 - 04:00)
  const isCrossMidnight = (): boolean => {
    if (!rule.active_start_time || !rule.active_end_time) return false;
    const [startH] = rule.active_start_time.split(':').map(Number);
    const [endH] = rule.active_end_time.split(':').map(Number);
    return endH < startH;
  };

  // Format active days - compress consecutive ranges
  const formatActiveDays = (): string => {
    if (!rule.active_days || rule.active_days.length === 0 || rule.active_days.length === 7) {
      return t('settings.price_rule.time_viz.every_day');
    }

    // Sort by week order (Mon-Sun)
    const sorted = [...rule.active_days].sort((a, b) =>
      WEEK_ORDER.indexOf(a) - WEEK_ORDER.indexOf(b)
    );

    // Find consecutive ranges
    const ranges: number[][] = [];
    let currentRange: number[] = [sorted[0]];

    for (let i = 1; i < sorted.length; i++) {
      const prevIdx = WEEK_ORDER.indexOf(sorted[i - 1]);
      const currIdx = WEEK_ORDER.indexOf(sorted[i]);
      if (currIdx === prevIdx + 1) {
        currentRange.push(sorted[i]);
      } else {
        ranges.push(currentRange);
        currentRange = [sorted[i]];
      }
    }
    ranges.push(currentRange);

    // Format each range
    const listFormatter = new Intl.ListFormat(locale, { style: 'narrow', type: 'conjunction' });

    const parts = ranges.map(range => {
      if (range.length === 1) {
        return t(`calendar.days.${DAY_KEYS[range[0]]}`);
      } else if (range.length === 2) {
        // Use ListFormat for 2 consecutive days: "一、二" / "L, M"
        const twoDays = range.map(d => t(`calendar.days.${DAY_KEYS[d]}`));
        return listFormatter.format(twoDays);
      } else {
        // Range of 3+: "一至五" / "L-V"
        return `${t(`calendar.days.${DAY_KEYS[range[0]]}`)}${t('settings.price_rule.time_viz.to')}${t(`calendar.days.${DAY_KEYS[range[range.length - 1]]}`)}`;
      }
    });

    return listFormatter.format(parts);
  };

  // Format time range with cross-midnight indicator
  const formatTimeRange = (): string => {
    if (!rule.active_start_time || !rule.active_end_time) {
      return t('settings.price_rule.time_viz.all_day');
    }
    const timeStr = `${rule.active_start_time} - ${rule.active_end_time}`;
    if (isCrossMidnight()) {
      return `${timeStr} ${t('settings.price_rule.time_viz.next_day')}`;
    }
    return timeStr;
  };

  // Format date range
  const formatDateRange = (): string | null => {
    if (!rule.valid_from && !rule.valid_until) return null;

    const formatDate = (ts: number) =>
      new Date(ts).toLocaleDateString(locale, {
        year: 'numeric',
        month: 'long',
        day: 'numeric',
      });

    if (rule.valid_from && rule.valid_until) {
      const daysRemaining = Math.ceil((rule.valid_until - Date.now()) / (1000 * 60 * 60 * 24));
      return `${formatDate(rule.valid_from)} ~ ${formatDate(rule.valid_until)} (${daysRemaining > 0 ? `${t('settings.price_rule.time_viz.remaining')} ${daysRemaining} ${t('settings.price_rule.time_viz.days')}` : t('settings.price_rule.time_viz.expired')})`;
    }
    if (rule.valid_from) {
      return `${formatDate(rule.valid_from)} ${t('settings.price_rule.time_viz.onwards')}`;
    }
    if (rule.valid_until) {
      return `${t('settings.price_rule.time_viz.until')} ${formatDate(rule.valid_until)}`;
    }
    return null;
  };

  // Check if always active (no time restrictions)
  const isAlwaysActive =
    (!rule.active_days || rule.active_days.length === 0 || rule.active_days.length === 7) &&
    !rule.active_start_time &&
    !rule.active_end_time &&
    !rule.valid_from &&
    !rule.valid_until;

  const activeColor = isDiscount ? 'bg-amber-400' : 'bg-purple-400';
  const dateRange = formatDateRange();

  return (
    <div className="bg-gray-50 rounded-xl p-4">
      <div className="flex items-center gap-2 mb-3">
        <Calendar size={16} className="text-gray-500" />
        <span className="text-sm font-medium text-gray-700">
          {t('settings.price_rule.time_viz.title')}
        </span>
      </div>

      {isAlwaysActive ? (
        <div className="text-sm text-gray-500">
          {t('settings.price_rule.time_viz.always_active')}
        </div>
      ) : (
        <div className="flex items-center gap-2 text-sm">
          <span className={`w-3 h-3 rounded-full shrink-0 ${activeColor}`} />
          <span className="text-gray-700">
            {formatActiveDays()} {formatTimeRange()}
          </span>
        </div>
      )}

      {/* Date range */}
      {dateRange && (
        <div className="mt-3 pt-3 border-t border-gray-200 text-sm text-gray-500">
          {t('settings.price_rule.time_viz.date_range')}: {dateRange}
        </div>
      )}
    </div>
  );
};
