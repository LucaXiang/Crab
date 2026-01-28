import React from 'react';
import { X } from 'lucide-react';

interface GuestCountInputProps {
  guestInput: string;
  onGuestInputChange: (value: string) => void;
}

export const GuestCountInput: React.FC<GuestCountInputProps> = ({
  guestInput,
  onGuestInputChange,
}) => {
  const handleNumpad = (val: string) => {
    if (guestInput.length >= 2) return;
    onGuestInputChange(guestInput + val);
  };

  const handleBackspace = () => {
    onGuestInputChange(guestInput.slice(0, -1));
  };

  return (
    <div className="flex flex-col items-center p-0 w-full max-w-sm">
      <div className="text-6xl font-bold text-primary-500 mb-2 font-mono">
        {guestInput || <span className="text-gray-200">0</span>}
      </div>

      {/* Quick Select */}
      <div className="grid grid-cols-4 gap-2 mb-3 w-full px-8">
        {[2, 4, 6, 8].map((n) => (
          <button
            key={n}
            onClick={() => onGuestInputChange(n.toString())}
            className="py-2.5 bg-gray-100 hover:bg-gray-200 rounded-lg text-sm font-bold text-gray-600 border border-gray-200"
          >
            {n}
          </button>
        ))}
      </div>

      {/* Numpad */}
      <div className="grid grid-cols-3 gap-2 w-full px-8 pb-2">
        {[1, 2, 3, 4, 5, 6, 7, 8, 9].map((n) => (
          <button
            key={n}
            onClick={() => handleNumpad(n.toString())}
            className="h-14 rounded-xl border border-gray-200 text-2xl font-bold text-gray-700 hover:bg-blue-50 hover:border-blue-200 active:bg-blue-100 shadow-sm transition-all"
          >
            {n}
          </button>
        ))}
        <button
          onClick={handleBackspace}
          className="h-14 rounded-xl bg-gray-50 border border-gray-200 text-gray-500 hover:bg-gray-100 hover:text-red-500 flex items-center justify-center"
        >
          <X size={24} />
        </button>
        <button
          onClick={() => handleNumpad('0')}
          className="h-14 rounded-xl border border-gray-200 text-2xl font-bold text-gray-700 hover:bg-blue-50 hover:border-blue-200 active:bg-blue-100 shadow-sm transition-all"
        >
          0
        </button>
      </div>
    </div>
  );
};
