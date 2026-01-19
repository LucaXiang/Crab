import React, { useState, useEffect } from 'react';
import { Users, Clock, Receipt, Percent } from 'lucide-react';
import { Table, HeldOrder, Zone } from '@/core/domain/types';

interface TableCardProps {
  table: Table;
  order?: HeldOrder;
  mode: 'HOLD' | 'RETRIEVE';
  disabled?: boolean;
  className?: string;
  onClick: () => void;
  zone?: Zone;
}

export const TableCard: React.FC<TableCardProps> = React.memo(
  ({ table, order, mode, disabled, className, onClick, zone }) => {
    const isOccupied = !!order;
    const isDisabled = disabled || (mode === 'RETRIEVE' && !isOccupied);

    // Check if overtime (2 hours)
    const isLongTime =
      isOccupied && order && Date.now() - order.startTime > 2 * 60 * 60 * 1000;

    const isPrePayment = isOccupied && !!order?.isPrePayment;
    
    // Check for surcharge
    const hasSurcharge = zone?.surcharge_amount && zone.surcharge_amount > 0;
    const isPercentage = zone?.surcharge_type === 'percentage';

    // Timer Logic
    const [duration, setDuration] = useState<string>('');

    useEffect(() => {
      if (!isOccupied || !order) {
        setDuration('');
        return () => {};
      }

      const updateTime = () => {
        const now = Date.now();
        const diff = now - order.startTime;
        const minutes = Math.floor(diff / 60000);
        const hours = Math.floor(minutes / 60);
        const mins = minutes % 60;
        if (hours > 0) setDuration(`${hours}h ${mins}m`);
        else setDuration(`${mins}m`);
      };

      updateTime();
      const timer = setInterval(updateTime, 60000);
      return () => clearInterval(timer);
    }, [isOccupied, order]);

    return (
      <button
        onClick={onClick}
        disabled={isDisabled}
        className={`
          relative p-4 rounded-2xl shadow-sm border-2 transition-all duration-200 flex flex-col items-center justify-between overflow-hidden
          ${className || 'h-[120px]'}
          ${isDisabled ? 'opacity-50 cursor-not-allowed' : 'hover:shadow-md'}
          ${
            isPrePayment
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
       

        {/* Surcharge Indicator (for empty tables or tables without individual/color tags interfering) */}
        {!isOccupied && hasSurcharge && (
          <div className="absolute top-2 right-2 flex items-center gap-0.5 px-1.5 py-0.5 bg-yellow-100 text-yellow-700 rounded-full text-[10px] font-bold border border-yellow-200/50 shadow-sm">
            {isPercentage ? <Percent size={10} /> : <span className="text-[10px]">+</span>}
            <span>{zone?.surcharge_amount}</span>
          </div>
        )}

        {/* Table Name */}
        <div className="flex justify-between w-full mb-1">
          <span
            className={`text-xl font-bold ${
              isOccupied
                ? isPrePayment
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
                isPrePayment
                  ? 'text-purple-700 bg-purple-100/50'
                  : isLongTime
                  ? 'text-orange-700 bg-orange-100/50'
                  : 'text-blue-700 bg-blue-100/50'
              }`}
            >
              <Users size={14} />
              <span className="font-bold text-sm">{order?.guestCount}</span>
            </div>
            <div
              className={`flex items-center gap-1 text-xs mt-0.5 ${
                isPrePayment
                  ? 'text-purple-600 font-bold'
                  : isLongTime
                  ? 'text-orange-600 font-bold'
                  : 'text-blue-400'
              }`}
            >
              {isPrePayment ? <Receipt size={12} /> : <Clock size={12} />}
              <span>{duration || '0m'}</span>
            </div>
          </div>
        ) : (
          <div className="flex flex-col items-center gap-0.5 opacity-30 mt-auto">
            <Users size={18} />
            <span className="text-[10px]">{table.capacity}</span>
          </div>
        )}
      </button>
    );
  }
);
