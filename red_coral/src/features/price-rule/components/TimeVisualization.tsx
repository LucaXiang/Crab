import React from 'react';
import { Calendar } from 'lucide-react';
import type { PriceRule } from '@/core/domain/types/api';
import { useI18n } from '@/hooks/useI18n';

interface TimeVisualizationProps {
  rule: PriceRule;
}

// Day labels (Sunday = 0)
const DAY_LABELS = ['日', '一', '二', '三', '四', '五', '六'];

// Time slots for visualization (every 2 hours)
const TIME_SLOTS = ['06', '08', '10', '12', '14', '16', '18', '20', '22', '24'];

export const TimeVisualization: React.FC<TimeVisualizationProps> = ({ rule }) => {
  const { t } = useI18n();
  const isDiscount = rule.rule_type === 'DISCOUNT';

  // Parse time to minutes since midnight
  const parseTime = (timeStr: string | null): number | null => {
    if (!timeStr) return null;
    const [h, m] = timeStr.split(':').map(Number);
    return h * 60 + m;
  };

  const startMinutes = parseTime(rule.active_start_time);
  const endMinutes = parseTime(rule.active_end_time);

  // Check if a day is active
  const isDayActive = (dayIndex: number): boolean => {
    if (!rule.active_days || rule.active_days.length === 0) return true;
    return rule.active_days.includes(dayIndex);
  };

  // Calculate which time slots are active for visualization
  const isSlotActive = (slotHour: number): boolean => {
    if (startMinutes === null || endMinutes === null) return true;
    const slotMinutes = slotHour * 60;
    return slotMinutes >= startMinutes && slotMinutes < endMinutes;
  };

  // Format active days text
  const formatActiveDays = (): string => {
    if (!rule.active_days || rule.active_days.length === 0 || rule.active_days.length === 7) {
      return t('settings.price_rule.time_viz.every_day');
    }
    const days = rule.active_days.map(d => `周${DAY_LABELS[d]}`).join('、');
    return days;
  };

  // Format time range text
  const formatTimeRange = (): string => {
    if (!rule.active_start_time || !rule.active_end_time) {
      return t('settings.price_rule.time_viz.all_day');
    }
    return `${rule.active_start_time} - ${rule.active_end_time}`;
  };

  // Format date range
  const formatDateRange = (): string | null => {
    if (!rule.valid_from && !rule.valid_until) return null;

    const formatDate = (ts: number) =>
      new Date(ts).toLocaleDateString('zh-CN', {
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
        <div className="text-center py-4">
          <span className="text-sm text-gray-500">
            {t('settings.price_rule.time_viz.always_active')}
          </span>
        </div>
      ) : (
        <>
          {/* Week calendar visualization */}
          <div className="bg-white rounded-lg p-3 mb-3">
            <div className="grid grid-cols-8 gap-1">
              {/* Header row */}
              <div /> {/* Empty corner */}
              {[1, 2, 3, 4, 5, 6, 0].map(dayIndex => (
                <div
                  key={dayIndex}
                  className={`text-center text-xs font-medium py-1 ${
                    isDayActive(dayIndex) ? 'text-gray-700' : 'text-gray-300'
                  }`}
                >
                  {DAY_LABELS[dayIndex]}
                </div>
              ))}

              {/* Time rows */}
              {TIME_SLOTS.slice(0, -1).map((slot, slotIndex) => {
                const hour = parseInt(slot);
                const showActive = isSlotActive(hour);

                return (
                  <React.Fragment key={slot}>
                    {/* Time label */}
                    <div className="text-xs text-gray-400 text-right pr-1 py-0.5">
                      {slot}:00
                    </div>
                    {/* Day cells */}
                    {[1, 2, 3, 4, 5, 6, 0].map(dayIndex => {
                      const isActive = isDayActive(dayIndex) && showActive;
                      return (
                        <div
                          key={`${slot}-${dayIndex}`}
                          className={`h-4 rounded-sm ${
                            isActive ? activeColor : 'bg-gray-100'
                          }`}
                        />
                      );
                    })}
                  </React.Fragment>
                );
              })}
            </div>
          </div>

          {/* Text summary */}
          <div className="space-y-1 text-sm">
            <div className="flex items-center gap-2">
              <span className={`w-3 h-3 rounded ${activeColor}`} />
              <span className="text-gray-600">
                {formatActiveDays()} {formatTimeRange()}
              </span>
            </div>
          </div>
        </>
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
