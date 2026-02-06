import React from 'react';
import { Check, Delete } from 'lucide-react';

interface NumpadProps {
  onNumber: (n: string) => void;
  onDelete?: () => void;
  onClear?: () => void;
  onEnter?: () => void;
  className?: string;
  showDecimal?: boolean;
  showEnter?: boolean;
}

export const Numpad: React.FC<NumpadProps> = React.memo(
  ({ onNumber, onDelete, onClear, onEnter, className = '', showDecimal = true, showEnter = true }) => (
    <div className={`grid grid-cols-3 grid-rows-5 gap-2 h-full p-2 ${className}`}>
      {[1, 2, 3, 4, 5, 6, 7, 8, 9].map((n) => (
        <button
          key={n}
          onClick={() => onNumber(n.toString())}
          className="bg-white border border-gray-100 rounded-2xl text-2xl font-bold text-gray-800 hover:bg-gray-50 hover:border-blue-500 hover:text-blue-600 active:scale-95 active:bg-blue-50 transition-all shadow-sm"
        >
          {n}
        </button>
      ))}

      {showDecimal ? (
        <button
          onClick={() => onNumber('.')}
          className="bg-white border border-gray-100 rounded-2xl text-2xl font-bold text-gray-800 hover:bg-gray-50 hover:border-blue-500 hover:text-blue-600 active:scale-95 active:bg-blue-50 transition-all shadow-sm"
        >
          .
        </button>
      ) : (
        <div />
      )}

      <button
        onClick={() => onNumber('0')}
        className="bg-white border border-gray-100 rounded-2xl text-2xl font-bold text-gray-800 hover:bg-gray-50 hover:border-blue-500 hover:text-blue-600 active:scale-95 active:bg-blue-50 transition-all shadow-sm"
      >
        0
      </button>

      <button
        onClick={onDelete}
        className="bg-white border border-gray-100 rounded-2xl text-gray-500 hover:bg-red-50 hover:text-red-600 hover:border-red-200 active:scale-95 active:bg-red-100 transition-all shadow-sm flex items-center justify-center"
      >
        <Delete size={24} />
      </button>

      {/* Action Row */}
      <button
        onClick={onClear}
        className={`bg-red-50 border border-red-100 rounded-2xl text-red-500 font-bold hover:bg-red-100 hover:text-red-700 active:scale-95 active:bg-red-200 transition-all shadow-sm ${!showEnter ? 'col-span-3' : ''}`}
      >
        C
      </button>

      {showEnter && (
        <button
          onClick={onEnter}
          className="col-span-2 bg-gray-900 text-white rounded-2xl flex items-center justify-center shadow-lg shadow-gray-200 hover:bg-black active:scale-95 active:bg-gray-800 transition-all"
        >
          <Check size={32} />
        </button>
      )}
    </div>
  )
);
