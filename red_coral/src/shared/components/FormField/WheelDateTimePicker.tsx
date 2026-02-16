import React, { useEffect, useCallback, useState, useMemo } from 'react';
import { Calendar } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { inputClass } from './FormField';
import { WheelColumn } from './WheelColumn';
import { WheelPickerModal } from './WheelPickerModal';

function getDaysInMonth(year: number, month: number): number {
  return new Date(year, month, 0).getDate();
}

function parseDateTime(value: string): { year: number; month: number; day: number; hour: number; minute: number } {
  const match = value.match(/^(\d{4})[/-](\d{1,2})[/-](\d{1,2})[T ](\d{1,2}):(\d{2})$/);
  if (match) {
    return {
      year: parseInt(match[1]),
      month: parseInt(match[2]),
      day: parseInt(match[3]),
      hour: parseInt(match[4]),
      minute: parseInt(match[5]),
    };
  }
  const now = new Date();
  return { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate(), hour: 0, minute: 0 };
}

function formatDateTime(year: number, month: number, day: number, hour: number, minute: number): string {
  const d = `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
  const t = `${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}`;
  return `${d}T${t}`;
}

interface WheelDateTimePickerProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export const WheelDateTimePicker: React.FC<WheelDateTimePickerProps> = ({ value, onChange, placeholder }) => {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const initial = value ? parseDateTime(value) : (() => {
    const now = new Date();
    return { year: now.getFullYear(), month: now.getMonth() + 1, day: now.getDate(), hour: 0, minute: 0 };
  })();
  const [year, setYear] = useState(initial.year);
  const [month, setMonth] = useState(initial.month);
  const [day, setDay] = useState(initial.day);
  const [hour, setHour] = useState(initial.hour);
  const [minute, setMinute] = useState(initial.minute);

  useEffect(() => {
    if (value) {
      const parsed = parseDateTime(value);
      setYear(parsed.year);
      setMonth(parsed.month);
      setDay(parsed.day);
      setHour(parsed.hour);
      setMinute(parsed.minute);
    }
  }, [value]);

  const currentYear = new Date().getFullYear();

  const years = useMemo(
    () => Array.from({ length: 10 }, (_, i) => {
      const y = currentYear + 2 - i;
      return { value: y, label: String(y) };
    }),
    [currentYear],
  );

  const months = useMemo(
    () => Array.from({ length: 12 }, (_, i) => ({
      value: i + 1,
      label: String(i + 1).padStart(2, '0'),
    })),
    [],
  );

  const days = useMemo(() => {
    const count = getDaysInMonth(year, month);
    return Array.from({ length: count }, (_, i) => ({
      value: i + 1,
      label: String(i + 1).padStart(2, '0'),
    }));
  }, [year, month]);

  const hours = useMemo(
    () => Array.from({ length: 24 }, (_, i) => ({
      value: i,
      label: String(i).padStart(2, '0'),
    })),
    [],
  );

  const minutes = useMemo(
    () => Array.from({ length: 60 }, (_, i) => ({
      value: i,
      label: String(i).padStart(2, '0'),
    })),
    [],
  );

  useEffect(() => {
    const maxDay = getDaysInMonth(year, month);
    if (day > maxDay) setDay(maxDay);
  }, [year, month, day]);

  const handleConfirm = useCallback(() => {
    const maxDay = getDaysInMonth(year, month);
    const clampedDay = Math.min(day, maxDay);
    onChange(formatDateTime(year, month, clampedDay, hour, minute));
    setOpen(false);
  }, [year, month, day, hour, minute, onChange]);

  const handleClear = useCallback(() => {
    onChange('');
    setOpen(false);
  }, [onChange]);

  const displayValue = value
    ? (() => {
        const d = parseDateTime(value);
        return `${d.year}/${String(d.month).padStart(2, '0')}/${String(d.day).padStart(2, '0')} ${String(d.hour).padStart(2, '0')}:${String(d.minute).padStart(2, '0')}`;
      })()
    : '';

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className={`${inputClass} text-left flex items-center justify-between cursor-pointer`}
      >
        <span className={displayValue ? 'text-gray-900' : 'text-gray-400'}>
          {displayValue || placeholder || 'YYYY/MM/DD HH:MM'}
        </span>
        <Calendar size={16} className="text-gray-400 shrink-0" />
      </button>

      {open && (
        <WheelPickerModal
          title={placeholder || 'YYYY/MM/DD HH:MM'}
          icon={<Calendar size={18} className="text-teal-500" />}
          onClose={() => setOpen(false)}
          onConfirm={handleConfirm}
          onClear={handleClear}
          preview={
            <span className="text-xl font-bold text-teal-600 tabular-nums">
              {year}/{String(month).padStart(2, '0')}/{String(day).padStart(2, '0')}{' '}
              {String(hour).padStart(2, '0')}:{String(minute).padStart(2, '0')}
            </span>
          }
        >
          <div className="flex gap-1 px-3 py-3">
            <WheelColumn items={years} selected={year} onChange={setYear} label={t('common.time_unit.year')} />
            <WheelColumn items={months} selected={month} onChange={setMonth} label={t('common.time_unit.month')} />
            <WheelColumn items={days} selected={day} onChange={setDay} label={t('common.time_unit.day')} />
            <WheelColumn items={hours} selected={hour} onChange={setHour} label={t('common.time_unit.hour')} />
            <WheelColumn items={minutes} selected={minute} onChange={setMinute} label={t('common.time_unit.minute')} />
          </div>
        </WheelPickerModal>
      )}
    </>
  );
};
