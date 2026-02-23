import React from 'react';

export const LogoIcon: React.FC<{ className?: string }> = ({ className = 'w-4 h-4' }) => (
  <svg viewBox="0 0 24 24" fill="none" className={`${className} text-white`} stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
    <path d="M6 13.87A4 4 0 0 1 7.41 6a5.11 5.11 0 0 1 1.05-1.54 5 5 0 0 1 7.08 0A5.11 5.11 0 0 1 16.59 6 4 4 0 0 1 18 13.87V21H6Z" />
    <line x1="6" y1="17" x2="18" y2="17" />
  </svg>
);

export const LogoBadge: React.FC<{ size?: 'sm' | 'md' }> = ({ size = 'md' }) => {
  const cls = size === 'sm' ? 'w-7 h-7' : 'w-8 h-8';
  const iconCls = size === 'sm' ? 'w-4 h-4' : 'w-4.5 h-4.5';
  return (
    <div className={`${cls} bg-primary-500 rounded-lg flex items-center justify-center`}>
      <LogoIcon className={iconCls} />
    </div>
  );
};

export const LogoText: React.FC<{ className?: string }> = ({ className = 'text-lg' }) => (
  <span className={`${className} font-bold text-slate-900`}>
    Red<span className="text-primary-500">Coral</span>
  </span>
);
