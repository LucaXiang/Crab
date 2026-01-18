import React from 'react';
import { LucideIcon } from 'lucide-react';

interface IconBtnProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  icon: LucideIcon;
  size?: number;
}

export const IconBtn: React.FC<IconBtnProps> = React.memo(
  ({ icon: Icon, className = '', size = 24, ...props }) => {
    return (
      <button
        type="button"
        className={`p-2 text-white/80 hover:text-white hover:bg-white/10 rounded-full transition-colors ${className}`}
        {...props}
      >
        <Icon size={size} strokeWidth={1.5} />
      </button>
    );
  }
);
