import React, { useState, useEffect } from 'react';
import { Users, Clock, Receipt } from 'lucide-react';
import { Table, HeldOrder } from '@/core/domain/types';

interface TableCardProps {
  table: Table;
  order?: HeldOrder;
  mode: 'HOLD' | 'RETRIEVE';
  disabled?: boolean;
  className?: string;
  onClick: () => void;
}

export const TableCard: React.FC<TableCardProps> = React.memo(
  ({ table, order, mode, disabled, className, onClick }) => {
    const isOccupied = !!order;
    const isDisabled = disabled || (mode === 'RETRIEVE' && !isOccupied);

    // Check if overtime (2 hours)
    const isLongTime =
      isOccupied && order && Date.now() - order.start_time > 2 * 60 * 60 * 1000;

    const is_pre_payment = isOccupied && !!order?.is_pre_payment;

    // Timer Logic
    const [duration, setDuration] = useState<string>('');

    useEffect(() => {
      if (!isOccupied || !order) {
        setDuration('');
        return;
      }

      const updateTime = () => {
        const now = Date.now();
        const diff = now - order.start_time;
        const minutes = Math.floor(diff / 60000);
        const hours = Math.floor(minutes / 60);
        const mins = minutes % 60;
        if (hours > 0) setDuration(`${hours}h ${mins}m`);
        else setDuration(`${mins}m`);
      };

      updateTime();
      // Update every minute for accurate time display
      const timer = setInterval(updateTime, 60 * 1000);
      return () => clearInterval(timer);
    }, [isOccupied, order?.order_id]);

    return (
      <button
        onClick={onClick}
        disabled={isDisabled}
        className={`
          relative p-4 rounded-2xl shadow-sm border-2 transition-all duration-200 flex flex-col items-center justify-between overflow-hidden
          ${className || 'h-[7.5rem]'}
          ${isDisabled ? 'opacity-50 cursor-not-allowed' : 'hover:shadow-md'}
          ${
            is_pre_payment
              ? 'bg-purple-50 border-purple-300 hover:border-purple-500'
              : isLongTime && isOccupied
              ? 'bg-orange-50 border-orange-300 hover:border-orange-500 animate-pulse'
              : isOccupied
              ? 'bg-blue-50 border-blue-200 hover:border-blue-400'
              : isDisabled
              ? 'bg-gray-50 border-gray-100'
              : 'bg-white border-gray-100 hover:border-green-300 text-gray-500 hover:text-green-600'
          }
        `}
      >
        {/* Table Name */}
        <div className="flex justify-between w-full mb-1">
          <span
            className={`text-xl font-bold ${
              isOccupied
                ? is_pre_payment
                  ? 'text-purple-600'
                  : isLongTime
                  ? 'text-orange-600'
                  : 'text-blue-600'
                : 'text-inherit'
            }`}
          >
            {table.name}
          </span>
        </div>

        {/* Content */}
        {isOccupied ? (
          <div className="flex flex-col items-center gap-1 w-full">
            <div
              className={`flex items-center gap-1 px-2 py-0.5 rounded-full ${
                is_pre_payment
                  ? 'text-purple-700 bg-purple-100/50'
                  : isLongTime
                  ? 'text-orange-700 bg-orange-100/50'
                  : 'text-blue-700 bg-blue-100/50'
              }`}
            >
              <Users size={14} />
              <span className="font-bold text-sm">{order?.guest_count}</span>
            </div>
            <div
              className={`flex items-center gap-1 text-xs mt-0.5 ${
                is_pre_payment
                  ? 'text-purple-600 font-bold'
                  : isLongTime
                  ? 'text-orange-600 font-bold'
                  : 'text-blue-400'
              }`}
            >
              {is_pre_payment ? <Receipt size={12} /> : <Clock size={12} />}
              <span>{duration || '0m'}</span>
            </div>
          </div>
        ) : (
          <div className="flex flex-col items-center gap-0.5 opacity-30 mt-auto">
            <Users size={18} />
            <span className="text-[0.625rem]">{table.capacity}</span>
          </div>
        )}
      </button>
    );
  }
);
