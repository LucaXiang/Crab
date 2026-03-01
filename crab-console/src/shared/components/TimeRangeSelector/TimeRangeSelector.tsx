import React, { useState, useRef, useEffect } from 'react';
import { Calendar } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

export type TimeRangePreset = 'today' | 'yesterday' | 'this_week' | 'this_month' | 'custom';

export interface TimeRange {
  from: number;
  to: number;
  preset: TimeRangePreset;
  label: string;
}

interface Props {
  value: TimeRange;
  onChange: (range: TimeRange) => void;
  /** Business day cutoff hour (e.g. 4 for 04:00). Defaults to 0 (midnight). */
  cutoffHour?: number;
}

/** Start of business day: midnight + cutoff hours */
function startOfBusinessDay(date: Date, cutoffHour: number): Date {
  const d = new Date(date.getFullYear(), date.getMonth(), date.getDate(), cutoffHour);
  // If current time is before cutoff, the business day started yesterday
  if (date < d) d.setDate(d.getDate() - 1);
  return d;
}

function startOfDay(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), date.getDate());
}

function endOfNow(): number {
  return Date.now() + 60_000;
}

function startOfWeek(date: Date): Date {
  const d = startOfDay(date);
  const day = d.getDay();
  const diff = day === 0 ? 6 : day - 1; // Monday = start of week
  d.setDate(d.getDate() - diff);
  return d;
}

function startOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), 1);
}

function formatDateInput(date: Date): string {
  const y = date.getFullYear();
  const m = String(date.getMonth() + 1).padStart(2, '0');
  const d = String(date.getDate()).padStart(2, '0');
  return `${y}-${m}-${d}`;
}

export function getPresetRange(preset: TimeRangePreset, t: (key: string) => string, customFrom?: number, customTo?: number, cutoffHour = 0): TimeRange {
  const now = new Date();
  const sod = cutoffHour > 0 ? startOfBusinessDay : (_d: Date) => startOfDay(_d);

  switch (preset) {
    case 'today':
      return { from: sod(now, cutoffHour).getTime(), to: endOfNow(), preset, label: t('stats.today') };
    case 'yesterday': {
      const todayStart = sod(now, cutoffHour);
      const yesterdayStart = new Date(todayStart);
      yesterdayStart.setDate(yesterdayStart.getDate() - 1);
      return { from: yesterdayStart.getTime(), to: todayStart.getTime(), preset, label: t('stats.yesterday') };
    }
    case 'this_week': {
      const weekStart = startOfWeek(now);
      // Apply cutoff to the week start day
      weekStart.setHours(cutoffHour, 0, 0, 0);
      return { from: weekStart.getTime(), to: endOfNow(), preset, label: t('stats.this_week') };
    }
    case 'this_month': {
      const monthStart = startOfMonth(now);
      monthStart.setHours(cutoffHour, 0, 0, 0);
      return { from: monthStart.getTime(), to: endOfNow(), preset, label: t('stats.this_month') };
    }
    case 'custom':
      return {
        from: customFrom ?? sod(now, cutoffHour).getTime(),
        to: customTo ?? endOfNow(),
        preset,
        label: t('stats.custom_range'),
      };
  }
}

/** Returns the equivalent previous period for comparison (e.g. today → yesterday). */
export function getPreviousRange(range: TimeRange): { from: number; to: number } {
  const duration = range.to - range.from;
  switch (range.preset) {
    case 'today': {
      // Compare vs yesterday (same hours)
      const yesterday = new Date(range.from);
      yesterday.setDate(yesterday.getDate() - 1);
      return { from: yesterday.getTime(), to: yesterday.getTime() + duration };
    }
    case 'yesterday': {
      // Compare vs day before yesterday
      const dayBefore = new Date(range.from);
      dayBefore.setDate(dayBefore.getDate() - 1);
      return { from: dayBefore.getTime(), to: dayBefore.getTime() + duration };
    }
    case 'this_week': {
      // Compare vs last week
      const prevWeekStart = new Date(range.from);
      prevWeekStart.setDate(prevWeekStart.getDate() - 7);
      return { from: prevWeekStart.getTime(), to: prevWeekStart.getTime() + duration };
    }
    case 'this_month': {
      // Compare vs last month
      const prevMonth = new Date(range.from);
      prevMonth.setMonth(prevMonth.getMonth() - 1);
      const prevMonthEnd = new Date(range.to);
      prevMonthEnd.setMonth(prevMonthEnd.getMonth() - 1);
      return { from: prevMonth.getTime(), to: prevMonthEnd.getTime() };
    }
    case 'custom': {
      // Compare vs same-length period immediately before
      return { from: range.from - duration, to: range.from };
    }
  }
}

/** Returns the same day last week range (for hourly comparison). */
export function getLastWeekSameDayRange(range: TimeRange): { from: number; to: number } {
  const from = new Date(range.from);
  from.setDate(from.getDate() - 7);
  const to = new Date(range.to);
  to.setDate(to.getDate() - 7);
  return { from: from.getTime(), to: to.getTime() };
}

const PRESETS: TimeRangePreset[] = ['today', 'yesterday', 'this_week', 'this_month'];

export const TimeRangeSelector: React.FC<Props> = ({ value, onChange, cutoffHour = 0 }) => {
  const { t } = useI18n();
  const [showCustom, setShowCustom] = useState(false);
  const customRef = useRef<HTMLDivElement>(null);

  const [customFrom, setCustomFrom] = useState(() => formatDateInput(new Date(value.from)));
  const [customTo, setCustomTo] = useState(() => formatDateInput(new Date(value.to)));

  // Close custom picker on outside click
  useEffect(() => {
    if (!showCustom) return;
    const handler = (e: MouseEvent) => {
      if (customRef.current && !customRef.current.contains(e.target as Node)) {
        setShowCustom(false);
      }
    };
    document.addEventListener('mousedown', handler);
    return () => document.removeEventListener('mousedown', handler);
  }, [showCustom]);

  const handlePreset = (preset: TimeRangePreset) => {
    if (preset === 'custom') {
      setShowCustom(!showCustom);
      return;
    }
    setShowCustom(false);
    onChange(getPresetRange(preset, t, undefined, undefined, cutoffHour));
  };

  const handleCustomApply = () => {
    const from = new Date(customFrom);
    from.setHours(cutoffHour, 0, 0, 0);
    const toDate = new Date(customTo);
    toDate.setDate(toDate.getDate() + 1);
    toDate.setHours(cutoffHour, 0, 0, 0);
    onChange(getPresetRange('custom', t, from.getTime(), toDate.getTime(), cutoffHour));
    setShowCustom(false);
  };

  const presetLabels: Record<TimeRangePreset, string> = {
    today: t('stats.today'),
    yesterday: t('stats.yesterday'),
    this_week: t('stats.this_week'),
    this_month: t('stats.this_month'),
    custom: t('stats.custom_range'),
  };

  return (
    <div className="relative">
      {/* Preset buttons — horizontal scroll on mobile */}
      <div className="flex items-center gap-1.5 overflow-x-auto no-scrollbar pb-1">
        {PRESETS.map(preset => (
          <button
            key={preset}
            onClick={() => handlePreset(preset)}
            className={`shrink-0 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors cursor-pointer ${
              value.preset === preset
                ? 'bg-primary-500 text-white shadow-sm'
                : 'bg-white text-slate-600 border border-slate-200 hover:bg-slate-50'
            }`}
          >
            {presetLabels[preset]}
          </button>
        ))}
        <button
          onClick={() => handlePreset('custom')}
          className={`shrink-0 px-3 py-1.5 rounded-lg text-sm font-medium transition-colors cursor-pointer flex items-center gap-1.5 ${
            value.preset === 'custom'
              ? 'bg-primary-500 text-white shadow-sm'
              : 'bg-white text-slate-600 border border-slate-200 hover:bg-slate-50'
          }`}
        >
          <Calendar className="w-3.5 h-3.5" />
          {presetLabels.custom}
        </button>
      </div>

      {/* Custom date picker dropdown */}
      {showCustom && (
        <div
          ref={customRef}
          className="absolute top-full left-0 mt-2 bg-white rounded-xl border border-slate-200 shadow-lg p-4 z-20 w-full sm:w-auto sm:min-w-[320px]"
        >
          <div className="flex flex-col sm:flex-row items-stretch sm:items-end gap-3">
            <div className="flex-1">
              <label className="block text-xs font-medium text-slate-500 mb-1">{t('common.label.from')}</label>
              <input
                type="date"
                value={customFrom}
                onChange={e => setCustomFrom(e.target.value)}
                className="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
              />
            </div>
            <div className="flex-1">
              <label className="block text-xs font-medium text-slate-500 mb-1">{t('common.label.to')}</label>
              <input
                type="date"
                value={customTo}
                onChange={e => setCustomTo(e.target.value)}
                className="w-full px-3 py-2 border border-slate-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-primary-500/20 focus:border-primary-500"
              />
            </div>
            <button
              onClick={handleCustomApply}
              className="px-4 py-2 bg-primary-500 hover:bg-primary-600 text-white text-sm font-medium rounded-lg transition-colors cursor-pointer shrink-0"
            >
              {t('common.label.apply')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
