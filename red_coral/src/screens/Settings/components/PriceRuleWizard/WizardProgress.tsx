import React from 'react';
import { Check } from 'lucide-react';

interface WizardProgressProps {
  currentStep: number;
  totalSteps: number;
  titles: string[];
}

export const WizardProgress: React.FC<WizardProgressProps> = ({
  currentStep,
  totalSteps,
  titles,
}) => {
  return (
    <div className="flex items-center justify-between">
      {Array.from({ length: totalSteps }, (_, i) => {
        const step = i + 1;
        const isActive = step === currentStep;
        const isCompleted = step < currentStep;

        return (
          <React.Fragment key={step}>
            {/* Step indicator */}
            <div className="flex flex-col items-center">
              <div
                className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-semibold transition-colors ${
                  isCompleted
                    ? 'bg-teal-600 text-white'
                    : isActive
                    ? 'bg-teal-600 text-white ring-4 ring-teal-100'
                    : 'bg-gray-100 text-gray-400'
                }`}
              >
                {isCompleted ? <Check size={16} /> : step}
              </div>
              <span
                className={`mt-1 text-xs font-medium truncate max-w-[60px] text-center ${
                  isActive ? 'text-teal-600' : 'text-gray-400'
                }`}
              >
                {titles[i]}
              </span>
            </div>

            {/* Connector line */}
            {step < totalSteps && (
              <div
                className={`flex-1 h-0.5 mx-2 transition-colors ${
                  isCompleted ? 'bg-teal-600' : 'bg-gray-200'
                }`}
              />
            )}
          </React.Fragment>
        );
      })}
    </div>
  );
};
