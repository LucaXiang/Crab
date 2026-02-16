import React from 'react';
import { Clock, Calendar, Repeat, CalendarClock } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import type { WizardState } from './index';
import { FormSection, FormField, WheelTimePicker, WheelDateTimePicker } from '@/shared/components/FormField';

interface Step4TimeProps {
  state: WizardState;
  updateState: (updates: Partial<WizardState>) => void;
}

export const Step4Time: React.FC<Step4TimeProps> = ({ state, updateState }) => {
  const { t } = useI18n();

  const DAYS_OF_WEEK = [
    { value: 0, label: t('calendar.days.sunday') },
    { value: 1, label: t('calendar.days.monday') },
    { value: 2, label: t('calendar.days.tuesday') },
    { value: 3, label: t('calendar.days.wednesday') },
    { value: 4, label: t('calendar.days.thursday') },
    { value: 5, label: t('calendar.days.friday') },
    { value: 6, label: t('calendar.days.saturday') },
  ];

  const timeModeOptions = [
    {
      value: 'ALWAYS',
      icon: Clock,
      label: t('settings.price_rule.time.always'),
      desc: t('settings.price_rule.wizard.time_always_desc'),
    },
    {
      value: 'SCHEDULE',
      icon: Repeat,
      label: t('settings.price_rule.time.schedule'),
      desc: t('settings.price_rule.wizard.time_schedule_desc'),
    },
    {
      value: 'ONETIME',
      icon: CalendarClock,
      label: t('settings.price_rule.time.onetime'),
      desc: t('settings.price_rule.wizard.time_onetime_desc'),
    },
  ];

  const toggleDay = (day: number) => {
    const days = state.active_days.includes(day)
      ? state.active_days.filter((d) => d !== day)
      : [...state.active_days, day].sort((a, b) => a - b);
    updateState({ active_days: days });
  };

  return (
    <FormSection title={t('settings.price_rule.wizard.step4_section')} icon={Calendar}>
      <p className="text-sm text-gray-600 mb-6">
        {t('settings.price_rule.wizard.step4_desc')}
      </p>

      {/* Time Mode Selection */}
      <div className="space-y-3 mb-6">
        {timeModeOptions.map((option) => {
          const Icon = option.icon;
          const isSelected = state.time_mode === option.value;
          return (
            <button
              key={option.value}
              type="button"
              onClick={() => updateState({ time_mode: option.value as WizardState['time_mode'] })}
              className={`w-full flex items-start gap-4 p-4 rounded-xl border-2 text-left transition-all ${
                isSelected
                  ? 'border-teal-500 bg-teal-50'
                  : 'border-gray-200 bg-white hover:border-gray-300'
              }`}
            >
              <div
                className={`w-10 h-10 rounded-lg flex items-center justify-center shrink-0 ${
                  isSelected ? 'bg-teal-100 text-teal-600' : 'bg-gray-100 text-gray-400'
                }`}
              >
                <Icon size={20} />
              </div>
              <div className="flex-1">
                <div className="flex items-center gap-2">
                  <span className={`font-medium ${isSelected ? 'text-teal-700' : 'text-gray-700'}`}>
                    {option.label}
                  </span>
                  {isSelected && (
                    <div className="w-5 h-5 rounded-full bg-teal-500 flex items-center justify-center">
                      <svg className="w-3 h-3 text-white" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={3} d="M5 13l4 4L19 7" />
                      </svg>
                    </div>
                  )}
                </div>
                <span className="text-xs text-gray-500">{option.desc}</span>
              </div>
            </button>
          );
        })}
      </div>

      {/* Schedule Config */}
      {state.time_mode === 'SCHEDULE' && (
        <div className="space-y-4 p-4 bg-gray-50 rounded-xl">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-3">
              {t('settings.price_rule.wizard.active_days')}
            </label>
            <div className="flex gap-2">
              {DAYS_OF_WEEK.map((day) => (
                <button
                  key={day.value}
                  type="button"
                  onClick={() => toggleDay(day.value)}
                  className={`w-10 h-10 rounded-lg font-medium text-sm transition-all ${
                    state.active_days.includes(day.value)
                      ? 'bg-teal-600 text-white'
                      : 'bg-white border border-gray-200 text-gray-600 hover:border-gray-300'
                  }`}
                >
                  {day.label}
                </button>
              ))}
            </div>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <FormField label={t('settings.price_rule.wizard.start_time')}>
              <WheelTimePicker
                value={state.active_start_time}
                onChange={(v) => updateState({ active_start_time: v })}
                placeholder={t('settings.price_rule.wizard.start_time')}
              />
            </FormField>
            <FormField label={t('settings.price_rule.wizard.end_time')}>
              <WheelTimePicker
                value={state.active_end_time}
                onChange={(v) => updateState({ active_end_time: v })}
                placeholder={t('settings.price_rule.wizard.end_time')}
              />
            </FormField>
          </div>
        </div>
      )}

      {/* Onetime Config */}
      {state.time_mode === 'ONETIME' && (
        <div className="space-y-4 p-4 bg-gray-50 rounded-xl">
          <div className="grid grid-cols-2 gap-4">
            <FormField label={t('settings.price_rule.wizard.valid_from')} required>
              <WheelDateTimePicker
                value={state.valid_from}
                onChange={(v) => updateState({ valid_from: v })}
                placeholder={t('settings.price_rule.wizard.valid_from')}
              />
            </FormField>
            <FormField label={t('settings.price_rule.wizard.valid_until')} required>
              <WheelDateTimePicker
                value={state.valid_until}
                onChange={(v) => updateState({ valid_until: v })}
                placeholder={t('settings.price_rule.wizard.valid_until')}
              />
            </FormField>
          </div>
        </div>
      )}
    </FormSection>
  );
};
