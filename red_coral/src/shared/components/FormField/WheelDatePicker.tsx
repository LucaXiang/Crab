import React, { useEffect, useCallback, useState, useMemo } from 'react';
import { Calendar } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { inputClass } from './FormField';
import { WheelColumn } from './WheelColumn';
import { WheelPickerModal } from './WheelPickerModal';

function getDaysInMonth(year: number, month: number): number {
  return new Date(year, month, 0).getDate();
}

function parseDate(value: string): { year: number; month: number; day: number } {
  const match = value.match(/^(\d{4})[/-](\d{1,2})[/-](\d{1,2})$/);
  if (match) {
    return { year: parseInt(match[1]), month: parseInt(match[2]), day: parseInt(match[3]) };
  }
  return { year: 1990, month: 1, day: 1 };
}

function formatDate(year: number, month: number, day: number): string {
  return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
}

interface WheelDatePickerProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export const WheelDatePicker: React.FC<WheelDatePickerProps> = ({ value, onChange, placeholder }) => {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const initial = value ? parseDate(value) : { year: 1990, month: 1, day: 1 };
  const [year, setYear] = useState(initial.year);
  const [month, setMonth] = useState(initial.month);
  const [day, setDay] = useState(initial.day);

  useEffect(() => {
    if (value) {
      const parsed = parseDate(value);
      setYear(parsed.year);
      setMonth(parsed.month);
      setDay(parsed.day);
    }
  }, [value]);

  const currentYear = new Date().getFullYear();

  const years = useMemo(
    () => Array.from({ length: 100 }, (_, i) => {
      const y = currentYear - i;
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

  useEffect(() => {
    const maxDay = getDaysInMonth(year, month);
    if (day > maxDay) setDay(maxDay);
  }, [year, month, day]);

  const handleConfirm = useCallback(() => {
    const maxDay = getDaysInMonth(year, month);
    const clampedDay = Math.min(day, maxDay);
    onChange(formatDate(year, month, clampedDay));
    setOpen(false);
  }, [year, month, day, onChange]);

  const handleClear = useCallback(() => {
    onChange('');
    setOpen(false);
  }, [onChange]);

  const displayValue = value
    ? (() => {
        const d = parseDate(value);
        return `${d.year}/${String(d.month).padStart(2, '0')}/${String(d.day).padStart(2, '0')}`;
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
          {displayValue || placeholder || 'YYYY/MM/DD'}
        </span>
        <Calendar size={16} className="text-gray-400 shrink-0" />
      </button>

      {open && (
        <WheelPickerModal
          title={placeholder || 'YYYY/MM/DD'}
          icon={<Calendar size={18} className="text-teal-500" />}
          onClose={() => setOpen(false)}
          onConfirm={handleConfirm}
          onClear={handleClear}
          preview={
            <span className="text-2xl font-bold text-teal-600 tabular-nums">
              {year}/{String(month).padStart(2, '0')}/{String(day).padStart(2, '0')}
            </span>
          }
        >
          <div className="flex gap-2 px-4 py-4">
            <WheelColumn items={years} selected={year} onChange={setYear} label={t('common.time_unit.year')} />
            <WheelColumn items={months} selected={month} onChange={setMonth} label={t('common.time_unit.month')} />
            <WheelColumn items={days} selected={day} onChange={setDay} label={t('common.time_unit.day')} />
          </div>
        </WheelPickerModal>
      )}
    </>
  );
};
