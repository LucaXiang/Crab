import React, { useState, useEffect } from 'react';
import { ChevronLeft, ChevronRight, Check, X } from 'lucide-react';

export interface WizardStep {
  id: string;
  title: string;
  description?: string;
  component: React.ReactNode;
  isValid?: boolean;
}

interface WizardProps {
  steps: WizardStep[];
  onFinish: () => void;
  onCancel: () => void;
  finishLabel?: string;
  isSubmitting?: boolean;
  title?: string;
}

export const Wizard: React.FC<WizardProps> = ({
  steps,
  onFinish,
  onCancel,
  finishLabel = 'Finish',
  isSubmitting = false,
  title,
}) => {
  const [currentStepIndex, setCurrentStepIndex] = useState(0);

  const currentStep = steps[currentStepIndex];
  const isLastStep = currentStepIndex === steps.length - 1;
  const isFirstStep = currentStepIndex === 0;

  const handleNext = () => {
    if (isLastStep) {
      onFinish();
    } else {
      setCurrentStepIndex(prev => prev + 1);
    }
  };

  const handleBack = () => {
    if (!isFirstStep) {
      setCurrentStepIndex(prev => prev - 1);
    }
  };

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        onCancel();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [onCancel]);

  return (
    <div className="flex flex-col h-full bg-white rounded-2xl shadow-xl border border-slate-200 overflow-hidden animate-in fade-in zoom-in-95 duration-200">
      {/* Header */}
      <div className="flex items-center justify-between px-6 py-4 border-b border-slate-100 bg-slate-50/80 backdrop-blur-sm sticky top-0 z-10">
        <div>
          <h2 className="text-lg font-bold text-slate-900">{title || currentStep.title}</h2>
          {title && (
            <p className="text-sm text-slate-500 mt-0.5 flex items-center gap-2">
              <span className="font-medium text-primary-600">{currentStep.title}</span>
              {currentStep.description && (
                <>
                  <span className="w-1 h-1 rounded-full bg-slate-300" />
                  <span>{currentStep.description}</span>
                </>
              )}
            </p>
          )}
        </div>
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-1 text-sm font-medium text-slate-400 bg-white px-3 py-1 rounded-full border border-slate-100 shadow-sm">
            <span>Step</span>
            <span className="text-slate-900 font-bold">{currentStepIndex + 1}</span>
            <span className="text-slate-300">/</span>
            <span className="text-slate-600">{steps.length}</span>
          </div>
          <button 
            onClick={onCancel}
            className="p-2 -mr-2 text-slate-400 hover:text-slate-600 rounded-full hover:bg-slate-100 transition-colors"
          >
            <X className="w-5 h-5" />
          </button>
        </div>
      </div>

      {/* Progress Bar */}
      <div className="h-1 bg-slate-100 w-full">
        <div 
          className="h-full bg-primary-500 transition-all duration-500 ease-out rounded-r-full shadow-[0_0_10px_rgba(var(--primary-500),0.5)]"
          style={{ width: `${((currentStepIndex + 1) / steps.length) * 100}%` }}
        />
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto p-6 md:p-8">
        <div className="max-w-2xl mx-auto space-y-6 animate-in slide-in-from-right-4 fade-in duration-300 key={currentStep.id}">
          {currentStep.component}
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between px-6 py-4 border-t border-slate-100 bg-slate-50/50 backdrop-blur-sm sticky bottom-0 z-10">
        <button
          onClick={isFirstStep ? onCancel : handleBack}
          className="flex items-center gap-2 px-4 py-2.5 text-sm font-medium text-slate-600 hover:text-slate-900 hover:bg-white hover:shadow-sm rounded-xl transition-all"
          disabled={isSubmitting}
        >
          {isFirstStep ? null : <ChevronLeft className="w-4 h-4" />}
          {isFirstStep ? 'Cancel' : 'Back'}
        </button>

        <div className="flex items-center gap-3">
           {/* Optional: Add a "Skip" button if needed in future */}
           
          <button
            onClick={handleNext}
            disabled={!currentStep.isValid || isSubmitting}
            className={`flex items-center gap-2 px-6 py-2.5 rounded-xl text-sm font-bold text-white transition-all shadow-sm ${
              !currentStep.isValid || isSubmitting
                ? 'bg-slate-300 cursor-not-allowed'
                : 'bg-primary-600 hover:bg-primary-700 hover:shadow-md hover:-translate-y-0.5 active:translate-y-0 active:scale-[0.98]'
            }`}
          >
            {isSubmitting ? (
              <span className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
            ) : isLastStep ? (
              <Check className="w-4 h-4" />
            ) : null}
            <span>{isLastStep ? finishLabel : 'Next'}</span>
            {!isLastStep && !isSubmitting && <ChevronRight className="w-4 h-4" />}
          </button>
        </div>
      </div>
    </div>
  );
};
