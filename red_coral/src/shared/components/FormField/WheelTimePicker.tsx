import React, { useEffect, useCallback, useState, useMemo } from 'react';
import { Clock } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { inputClass } from './FormField';
import { WheelColumn } from './WheelColumn';
import { WheelPickerModal } from './WheelPickerModal';

function parseTime(value: string): { hour: number; minute: number } {
  const match = value.match(/^(\d{1,2}):(\d{2})$/);
  if (match) {
    return { hour: parseInt(match[1]), minute: parseInt(match[2]) };
  }
  return { hour: 0, minute: 0 };
}

function formatTime(hour: number, minute: number): string {
  return `${String(hour).padStart(2, '0')}:${String(minute).padStart(2, '0')}`;
}

interface WheelTimePickerProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
}

export const WheelTimePicker: React.FC<WheelTimePickerProps> = ({ value, onChange, placeholder }) => {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  const initial = value ? parseTime(value) : { hour: 0, minute: 0 };
  const [hour, setHour] = useState(initial.hour);
  const [minute, setMinute] = useState(initial.minute);

  useEffect(() => {
    if (value) {
      const parsed = parseTime(value);
      setHour(parsed.hour);
      setMinute(parsed.minute);
    }
  }, [value]);

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

  const handleConfirm = useCallback(() => {
    onChange(formatTime(hour, minute));
    setOpen(false);
  }, [hour, minute, onChange]);

  const handleClear = useCallback(() => {
    onChange('');
    setOpen(false);
  }, [onChange]);

  const displayValue = value || '';

  return (
    <>
      <button
        type="button"
        onClick={() => setOpen(true)}
        className={`${inputClass} text-left flex items-center justify-between cursor-pointer`}
      >
        <span className={displayValue ? 'text-gray-900' : 'text-gray-400'}>
          {displayValue || placeholder || 'HH:MM'}
        </span>
        <Clock size={16} className="text-gray-400 shrink-0" />
      </button>

      {open && (
        <WheelPickerModal
          title={placeholder || 'HH:MM'}
          icon={<Clock size={18} className="text-teal-500" />}
          onClose={() => setOpen(false)}
          onConfirm={handleConfirm}
          onClear={handleClear}
          preview={
            <span className="text-3xl font-bold text-teal-600 tabular-nums">
              {String(hour).padStart(2, '0')}:{String(minute).padStart(2, '0')}
            </span>
          }
        >
          <div className="flex gap-2 px-6 py-4 items-end">
            <WheelColumn items={hours} selected={hour} onChange={setHour} label={t('common.time_unit.hour')} />
            <div className="text-2xl font-bold text-gray-300 pb-[120px]">:</div>
            <WheelColumn items={minutes} selected={minute} onChange={setMinute} label={t('common.time_unit.minute')} />
          </div>
        </WheelPickerModal>
      )}
    </>
  );
};
