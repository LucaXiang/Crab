import React from 'react';
import { useI18n } from '@/hooks/useI18n';

export const LoadingScreen: React.FC = () => {
  const { t } = useI18n();

  return (
    <div className="w-full h-full flex flex-col items-center justify-center bg-gray-50">
      <div className="relative flex flex-col items-center">
        {/* Animated Background Blob */}
        <div className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2 w-32 h-32 bg-[#FF5E5E]/20 rounded-full blur-2xl animate-pulse" />
        
        {/* Logo Container */}
        <div className="relative z-10 w-20 h-20 bg-white rounded-2xl flex items-center justify-center shadow-xl border border-gray-100 mb-6">
          <span className="text-4xl filter drop-shadow-md animate-[spin_3s_linear_infinite]">🐚</span>
        </div>

        {/* Loading Text */}
        <div className="flex flex-col items-center gap-3">
            <h3 className="text-xl font-medium text-gray-800 tracking-tight">
                RedCoral
            </h3>
            <div className="flex items-center gap-1.5">
                <div className="w-2 h-2 rounded-full bg-[#FF5E5E] animate-[bounce_1s_infinite_0ms]" />
                <div className="w-2 h-2 rounded-full bg-[#FF5E5E] animate-[bounce_1s_infinite_200ms]" />
                <div className="w-2 h-2 rounded-full bg-[#FF5E5E] animate-[bounce_1s_infinite_400ms]" />
            </div>
            <p className="text-sm text-gray-400 mt-2">
                {t('common.loading')}
            </p>
        </div>
      </div>
    </div>
  );
};
