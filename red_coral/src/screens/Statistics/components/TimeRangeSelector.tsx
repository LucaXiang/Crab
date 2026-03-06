import React, { useState, useRef, useEffect, useMemo } from 'react';
import { Calendar } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { useStoreInfo } from '@/core/stores/settings/useStoreInfoStore';

export type TimeRangePreset = 'today' | 'yesterday' | 'this_week' | 'this_month' | 'last_month' | 'custom';

export interface TimeRangeValue {
  from: number;
  to: number;
  preset: TimeRangePreset;
}

const PRESETS: TimeRangePreset[] = ['today', 'yesterday', 'this_week', 'this_month', 'last_month'];
const DAY = 86_400_000;

/** Start of the current business day, accounting for cutoff.
 *  If now is before cutoff, the business day started yesterday at cutoff. */
function startOfBusinessDay(date: Date, cutoffMinutes: number): Date {
  const h = Math.floor(cutoffMinutes / 60);
  const m = cutoffMinutes % 60;
  const d = new Date(date.getFullYear(), date.getMonth(), date.getDate(), h, m, 0, 0);
  if (date < d) d.setDate(d.getDate() - 1);
  return d;
}

function startOfWeek(date: Date): Date {
  const d = new Date(date.getFullYear(), date.getMonth(), date.getDate());
  const day = d.getDay();
  const diff = day === 0 ? 6 : day - 1; // Monday = start of week
  d.setDate(d.getDate() - diff);
  return d;
}

function startOfMonth(date: Date): Date {
  return new Date(date.getFullYear(), date.getMonth(), 1);
}

function computeRange(preset: TimeRangePreset, cutoffMinutes: number, customFrom?: string, customTo?: string): TimeRangeValue | null {
  const cutoffH = Math.floor(cutoffMinutes / 60);
  const cutoffM = cutoffMinutes % 60;

  if (preset === 'custom') {
    if (!customFrom || !customTo) return null;
    const from = new Date(customFrom);
    from.setHours(cutoffH, cutoffM, 0, 0);
    const to = new Date(customTo);
    to.setDate(to.getDate() + 1);
    to.setHours(cutoffH, cutoffM, 0, 0);
    if (to.getTime() <= from.getTime()) return null;
    return { from: from.getTime(), to: to.getTime(), preset };
  }

  const now = new Date();
  const bizDayStart = startOfBusinessDay(now, cutoffMinutes);

  switch (preset) {
    case 'today':
      return { from: bizDayStart.getTime(), to: bizDayStart.getTime() + DAY, preset };
    case 'yesterday': {
      const yesterdayStart = new Date(bizDayStart);
      yesterdayStart.setDate(yesterdayStart.getDate() - 1);
      return { from: yesterdayStart.getTime(), to: bizDayStart.getTime(), preset };
    }
    case 'this_week': {
      const weekStart = startOfWeek(now);
      weekStart.setHours(cutoffH, cutoffM, 0, 0);
      return { from: weekStart.getTime(), to: bizDayStart.getTime() + DAY, preset };
    }
    case 'this_month': {
      const monthStart = startOfMonth(now);
      monthStart.setHours(cutoffH, cutoffM, 0, 0);
      return { from: monthStart.getTime(), to: bizDayStart.getTime() + DAY, preset };
    }
    case 'last_month': {
      const thisMonthStart = startOfMonth(now);
      thisMonthStart.setHours(cutoffH, cutoffM, 0, 0);
      const lastMonthStart = new Date(thisMonthStart);
      lastMonthStart.setMonth(lastMonthStart.getMonth() - 1);
      return { from: lastMonthStart.getTime(), to: thisMonthStart.getTime(), preset };
    }
    default:
      return null;
  }
}

function formatDateInput(d: Date): string {
  return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;
}

interface Props {
  value: TimeRangeValue;
  onChange: (range: TimeRangeValue) => void;
}

export function useTimeRange(): [TimeRangeValue, (v: TimeRangeValue) => void] {
  const cutoffMinutes = useStoreInfo().business_day_cutoff ?? 0;
  const initial = useMemo(() => computeRange('today', cutoffMinutes)!, []);
  const [range, setRange] = useState<TimeRangeValue>(initial);
  return [range, setRange];
}

export function useCutoffMinutes(): number {
  return useStoreInfo().business_day_cutoff ?? 0;
}

export function getPresetRange(preset: TimeRangePreset, cutoffMinutes: number): TimeRangeValue | null {
  return computeRange(preset, cutoffMinutes);
}

export function previousRange(r: TimeRangeValue): { from: number; to: number } {
  if (r.preset === 'this_month' || r.preset === 'last_month') {
    const prevStart = new Date(r.from);
    prevStart.setMonth(prevStart.getMonth() - 1);
    const prevEnd = new Date(r.to);
    prevEnd.setMonth(prevEnd.getMonth() - 1);
    return { from: prevStart.getTime(), to: prevEnd.getTime() };
  }
  const duration = r.to - r.from;
  return { from: r.from - duration, to: r.from };
}

export function lastWeekRange(r: TimeRangeValue): { from: number; to: number } {
  const shift = 7 * DAY;
  return { from: r.from - shift, to: r.to - shift };
}

export const TimeRangeSelector: React.FC<Props> = ({ value, onChange }) => {
  const { t } = useI18n();
  const cutoffMinutes = useCutoffMinutes();
  const [showCustom, setShowCustom] = useState(false);
  const customRef = useRef<HTMLDivElement>(null);
  const [customFrom, setCustomFrom] = useState(() => formatDateInput(new Date(value.from)));
  const [customTo, setCustomTo] = useState(() => formatDateInput(new Date(value.to)));
  const [customError, setCustomError] = useState('');

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
    const range = computeRange(preset, cutoffMinutes);
    if (range) onChange(range);
  };

  const handleCustomApply = () => {
    const range = computeRange('custom', cutoffMinutes, customFrom, customTo);
    if (!range) {
      setCustomError(t('statistics.time.custom'));
      return;
    }
    const days = Math.round((range.to - range.from) / DAY);
    if (days > 90) {
      setCustomError('Max 90 days');
      return;
    }
    setCustomError('');
    onChange(range);
    setShowCustom(false);
  };

  const presetLabels: Record<TimeRangePreset, string> = {
    today: t('statistics.time.today'),
    yesterday: t('statistics.time.yesterday'),
    this_week: t('statistics.time.this_week'),
    this_month: t('statistics.time.this_month'),
    last_month: t('statistics.time.last_month'),
    custom: t('statistics.time.custom'),
  };

  return (
    <div className="relative mb-6">
      <div className="flex items-center gap-1.5 flex-wrap">
        {PRESETS.map(preset => (
          <button
            key={preset}
            onClick={() => handlePreset(preset)}
            className={`px-3.5 py-1.5 rounded-lg text-sm font-medium transition-colors cursor-pointer ${
              value.preset === preset
                ? 'bg-blue-500 text-white shadow-sm'
                : 'bg-white text-gray-600 border border-gray-200 hover:bg-gray-50'
            }`}
          >
            {presetLabels[preset]}
          </button>
        ))}
        <button
          onClick={() => handlePreset('custom')}
          className={`px-3.5 py-1.5 rounded-lg text-sm font-medium transition-colors cursor-pointer flex items-center gap-1.5 ${
            value.preset === 'custom'
              ? 'bg-blue-500 text-white shadow-sm'
              : 'bg-white text-gray-600 border border-gray-200 hover:bg-gray-50'
          }`}
        >
          <Calendar className="w-3.5 h-3.5" />
          {presetLabels.custom}
        </button>
      </div>

      {showCustom && (
        <div
          ref={customRef}
          className="absolute top-full left-0 mt-2 bg-white rounded-xl border border-gray-200 shadow-lg p-4 z-20 min-w-[340px]"
        >
          <div className="flex items-end gap-3">
            <div className="flex-1">
              <label className="block text-xs font-medium text-gray-500 mb-1">From</label>
              <input
                type="date"
                value={customFrom}
                onChange={e => { setCustomFrom(e.target.value); setCustomError(''); }}
                className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
              />
            </div>
            <div className="flex-1">
              <label className="block text-xs font-medium text-gray-500 mb-1">To</label>
              <input
                type="date"
                value={customTo}
                onChange={e => { setCustomTo(e.target.value); setCustomError(''); }}
                className="w-full px-3 py-2 border border-gray-200 rounded-lg text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/20 focus:border-blue-500"
              />
            </div>
            <button
              onClick={handleCustomApply}
              className="px-4 py-2 bg-blue-500 hover:bg-blue-600 text-white text-sm font-medium rounded-lg transition-colors cursor-pointer shrink-0"
            >
              {t('common.confirm')}
            </button>
          </div>
          {customError && <p className="text-xs text-red-500 mt-2">{customError}</p>}
        </div>
      )}
    </div>
  );
};
