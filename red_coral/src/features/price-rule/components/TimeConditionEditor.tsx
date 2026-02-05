import React, { useMemo } from 'react';
import { Calendar, Clock, X, Infinity, CalendarClock, CalendarRange } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { PriceRuleUpdate } from '@/core/domain/types/api';

interface TimeConditionEditorProps {
  isOpen: boolean;
  value: {
    active_days?: number[];
    active_start_time?: string;
    active_end_time?: string;
    valid_from?: number;
    valid_until?: number;
  };
  onChange: (updates: Partial<PriceRuleUpdate>) => void;
  onClose: () => void;
}

const DAY_LABELS = ['日', '一', '二', '三', '四', '五', '六'];

type TimeMode = 'always' | 'schedule' | 'onetime';

export const TimeConditionEditor: React.FC<TimeConditionEditorProps> = ({
  isOpen,
  value,
  onChange,
  onClose,
}) => {
  const { t } = useI18n();

  // Determine current time mode
  const currentMode = useMemo((): TimeMode => {
    const hasSchedule = (value.active_days && value.active_days.length > 0 && value.active_days.length < 7) ||
      value.active_start_time || value.active_end_time;
    const hasDateRange = value.valid_from || value.valid_until;

    if (!hasSchedule && !hasDateRange) return 'always';
    if (hasDateRange && !hasSchedule) return 'onetime';
    return 'schedule';
  }, [value]);

  if (!isOpen) return null;

  const activeDays = value.active_days ?? [];

  // Handle mode change
  const handleModeChange = (mode: TimeMode) => {
    switch (mode) {
      case 'always':
        onChange({
          active_days: [],
          active_start_time: undefined,
          active_end_time: undefined,
          valid_from: undefined,
          valid_until: undefined,
        });
        break;
      case 'schedule':
        // Keep schedule settings, clear date range
        onChange({
          valid_from: undefined,
          valid_until: undefined,
        });
        break;
      case 'onetime':
        // Clear schedule, prepare for date range
        onChange({
          active_days: [],
          active_start_time: undefined,
          active_end_time: undefined,
        });
        break;
    }
  };

  const toggleDay = (dayIndex: number) => {
    let newDays: number[];
    if (activeDays.includes(dayIndex)) {
      newDays = activeDays.filter(d => d !== dayIndex);
    } else {
      newDays = [...activeDays, dayIndex].sort();
    }
    // If all days selected or none selected, clear the array
    if (newDays.length === 7 || newDays.length === 0) {
      onChange({ active_days: [] });
    } else {
      onChange({ active_days: newDays });
    }
  };

  const selectAllDays = () => {
    onChange({ active_days: [] }); // Empty means all days
  };

  const selectWeekdays = () => {
    onChange({ active_days: [1, 2, 3, 4, 5] });
  };

  const selectWeekends = () => {
    onChange({ active_days: [0, 6] });
  };

  const formatDateForInput = (timestamp: number | undefined): string => {
    if (!timestamp) return '';
    const date = new Date(timestamp);
    return date.toISOString().split('T')[0];
  };

  const parseDateInput = (dateStr: string): number | undefined => {
    if (!dateStr) return undefined;
    return new Date(dateStr).getTime();
  };

  return (
    <div className="fixed inset-0 z-50 bg-black/50 backdrop-blur-sm flex items-center justify-center p-4">
      <div className="bg-white rounded-2xl shadow-2xl w-full max-w-md max-h-[85vh] overflow-hidden animate-in zoom-in-95 flex flex-col">
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-gray-200 shrink-0">
          <h3 className="text-lg font-bold text-gray-900">
            {t('settings.price_rule.edit.time_condition')}
          </h3>
          <button
            onClick={onClose}
            className="p-2 text-gray-400 hover:text-gray-600 hover:bg-gray-100 rounded-lg transition-colors"
          >
            <X size={20} />
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto p-5 space-y-6">
          {/* Mode Selection */}
          <div>
            <label className="text-sm font-medium text-gray-700 mb-3 block">
              {t('settings.price_rule.edit.time_mode')}
            </label>
            <div className="grid grid-cols-3 gap-2">
              {/* Always */}
              <button
                onClick={() => handleModeChange('always')}
                className={`flex flex-col items-center gap-2 p-4 rounded-xl transition-all ${
                  currentMode === 'always'
                    ? 'bg-teal-50 ring-2 ring-teal-500 text-teal-700'
                    : 'bg-gray-50 text-gray-600 hover:bg-gray-100'
                }`}
              >
                <Infinity size={24} />
                <span className="text-xs font-medium">
                  {t('settings.price_rule.time.always')}
                </span>
              </button>

              {/* Schedule */}
              <button
                onClick={() => handleModeChange('schedule')}
                className={`flex flex-col items-center gap-2 p-4 rounded-xl transition-all ${
                  currentMode === 'schedule'
                    ? 'bg-teal-50 ring-2 ring-teal-500 text-teal-700'
                    : 'bg-gray-50 text-gray-600 hover:bg-gray-100'
                }`}
              >
                <CalendarClock size={24} />
                <span className="text-xs font-medium">
                  {t('settings.price_rule.time.schedule')}
                </span>
              </button>

              {/* One-time */}
              <button
                onClick={() => handleModeChange('onetime')}
                className={`flex flex-col items-center gap-2 p-4 rounded-xl transition-all ${
                  currentMode === 'onetime'
                    ? 'bg-teal-50 ring-2 ring-teal-500 text-teal-700'
                    : 'bg-gray-50 text-gray-600 hover:bg-gray-100'
                }`}
              >
                <CalendarRange size={24} />
                <span className="text-xs font-medium">
                  {t('settings.price_rule.time.onetime')}
                </span>
              </button>
            </div>
          </div>

          {/* Schedule Mode: Active Days */}
          {currentMode === 'schedule' && (
            <div>
              <label className="flex items-center gap-2 text-sm font-medium text-gray-700 mb-3">
                <Calendar size={16} />
                {t('settings.price_rule.edit.active_days')}
              </label>

              {/* Quick select buttons */}
              <div className="flex gap-2 mb-3">
                <button
                  onClick={selectAllDays}
                  className={`px-3 py-1.5 text-xs rounded-lg transition-colors ${
                    activeDays.length === 0
                      ? 'bg-teal-500 text-white'
                      : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                  }`}
                >
                  {t('settings.price_rule.edit.all_days')}
                </button>
                <button
                  onClick={selectWeekdays}
                  className={`px-3 py-1.5 text-xs rounded-lg transition-colors ${
                    activeDays.length === 5 && !activeDays.includes(0) && !activeDays.includes(6)
                      ? 'bg-teal-500 text-white'
                      : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                  }`}
                >
                  {t('settings.price_rule.edit.weekdays')}
                </button>
                <button
                  onClick={selectWeekends}
                  className={`px-3 py-1.5 text-xs rounded-lg transition-colors ${
                    activeDays.length === 2 && activeDays.includes(0) && activeDays.includes(6)
                      ? 'bg-teal-500 text-white'
                      : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
                  }`}
                >
                  {t('settings.price_rule.edit.weekends')}
                </button>
              </div>

              {/* Day buttons */}
              <div className="grid grid-cols-7 gap-2">
                {[1, 2, 3, 4, 5, 6, 0].map(dayIndex => {
                  const isActive = activeDays.length === 0 || activeDays.includes(dayIndex);
                  return (
                    <button
                      key={dayIndex}
                      onClick={() => toggleDay(dayIndex)}
                      className={`
                        w-full aspect-square rounded-xl flex items-center justify-center text-sm font-medium transition-all
                        ${isActive
                          ? 'bg-teal-500 text-white shadow-md'
                          : 'bg-gray-100 text-gray-400 hover:bg-gray-200'
                        }
                      `}
                    >
                      {DAY_LABELS[dayIndex]}
                    </button>
                  );
                })}
              </div>
            </div>
          )}

          {/* Schedule Mode: Time Range */}
          {currentMode === 'schedule' && (
            <div>
              <label className="flex items-center gap-2 text-sm font-medium text-gray-700 mb-3">
                <Clock size={16} />
                {t('settings.price_rule.edit.time_range')}
              </label>

              <div className="flex items-center gap-3">
                <input
                  type="time"
                  value={value.active_start_time || ''}
                  onChange={e =>
                    onChange({
                      active_start_time: e.target.value || undefined,
                    })
                  }
                  className="flex-1 px-4 py-3 bg-gray-100 rounded-xl text-sm outline-none focus:ring-2 focus:ring-teal-500 focus:bg-white transition-all"
                />
                <span className="text-gray-400">~</span>
                <input
                  type="time"
                  value={value.active_end_time || ''}
                  onChange={e =>
                    onChange({
                      active_end_time: e.target.value || undefined,
                    })
                  }
                  className="flex-1 px-4 py-3 bg-gray-100 rounded-xl text-sm outline-none focus:ring-2 focus:ring-teal-500 focus:bg-white transition-all"
                />
              </div>

              {/* Clear time */}
              {(value.active_start_time || value.active_end_time) && (
                <button
                  onClick={() =>
                    onChange({
                      active_start_time: undefined,
                      active_end_time: undefined,
                    })
                  }
                  className="mt-2 text-xs text-gray-400 hover:text-gray-600"
                >
                  {t('settings.price_rule.edit.clear_time')}
                </button>
              )}
            </div>
          )}

          {/* One-time Mode: Date Range */}
          {currentMode === 'onetime' && (
            <div>
              <label className="flex items-center gap-2 text-sm font-medium text-gray-700 mb-3">
                <Calendar size={16} />
                {t('settings.price_rule.edit.date_range')}
              </label>

              <div className="space-y-3">
                <div>
                  <label className="text-xs text-gray-500 mb-1 block">
                    {t('settings.price_rule.edit.valid_from')}
                  </label>
                  <input
                    type="date"
                    value={formatDateForInput(value.valid_from)}
                    onChange={e =>
                      onChange({
                        valid_from: parseDateInput(e.target.value),
                      })
                    }
                    className="w-full px-4 py-3 bg-gray-100 rounded-xl text-sm outline-none focus:ring-2 focus:ring-teal-500 focus:bg-white transition-all"
                  />
                </div>
                <div>
                  <label className="text-xs text-gray-500 mb-1 block">
                    {t('settings.price_rule.edit.valid_until')}
                  </label>
                  <input
                    type="date"
                    value={formatDateForInput(value.valid_until)}
                    onChange={e =>
                      onChange({
                        valid_until: parseDateInput(e.target.value),
                      })
                    }
                    className="w-full px-4 py-3 bg-gray-100 rounded-xl text-sm outline-none focus:ring-2 focus:ring-teal-500 focus:bg-white transition-all"
                  />
                </div>
              </div>

              {/* Clear dates */}
              {(value.valid_from || value.valid_until) && (
                <button
                  onClick={() =>
                    onChange({
                      valid_from: undefined,
                      valid_until: undefined,
                    })
                  }
                  className="mt-2 text-xs text-gray-400 hover:text-gray-600"
                >
                  {t('settings.price_rule.edit.clear_date')}
                </button>
              )}
            </div>
          )}

          {/* Always Mode: Hint */}
          {currentMode === 'always' && (
            <div className="text-center py-8 text-gray-400">
              <Infinity size={48} className="mx-auto mb-3 opacity-50" />
              <p className="text-sm">{t('settings.price_rule.time_viz.always_active')}</p>
            </div>
          )}
        </div>

        {/* Footer */}
        <div className="px-5 py-4 border-t border-gray-200 shrink-0">
          <button
            onClick={onClose}
            className="w-full py-3 bg-teal-500 text-white rounded-xl font-medium hover:bg-teal-600 transition-colors"
          >
            {t('common.action.done')}
          </button>
        </div>
      </div>
    </div>
  );
};
