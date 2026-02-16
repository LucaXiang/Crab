import React, { useState, useMemo } from 'react';
import { Search, Check } from 'lucide-react';

interface MultiCardGridSelectorProps<T extends { id: number; name: string }> {
  items: T[];
  selectedIds: Set<number>;
  onToggle: (id: number) => void;
  searchPlaceholder: string;
  emptyText: string;
  accentColor?: 'teal' | 'violet';
  renderExtra?: (item: T) => React.ReactNode;
}

export function MultiCardGridSelector<T extends { id: number; name: string }>({
  items,
  selectedIds,
  onToggle,
  searchPlaceholder,
  emptyText,
  accentColor = 'teal',
  renderExtra,
}: MultiCardGridSelectorProps<T>) {
  const [search, setSearch] = useState('');

  const filtered = useMemo(() => {
    if (!search.trim()) return items;
    const lower = search.toLowerCase();
    return items.filter((item) => item.name.toLowerCase().includes(lower));
  }, [items, search]);

  const colors = accentColor === 'teal'
    ? { border: 'border-teal-500', bg: 'bg-teal-50', ring: 'ring-teal-200', text: 'text-teal-800', check: 'bg-teal-500', hover: 'hover:border-teal-300 hover:bg-teal-50/30', focus: 'focus:ring-teal-500/20 focus:border-teal-500' }
    : { border: 'border-violet-500', bg: 'bg-violet-50', ring: 'ring-violet-200', text: 'text-violet-800', check: 'bg-violet-500', hover: 'hover:border-violet-300 hover:bg-violet-50/30', focus: 'focus:ring-violet-500/20 focus:border-violet-500' };

  return (
    <div className="space-y-3">
      <div className="relative">
        <Search size={14} className="absolute left-3 top-1/2 -translate-y-1/2 text-gray-400" />
        <input
          type="text"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          placeholder={searchPlaceholder}
          className={`w-full pl-9 pr-3 py-2 text-sm border border-gray-200 rounded-xl focus:outline-none focus:ring-2 ${colors.focus} bg-white`}
        />
      </div>
      <div className="grid grid-cols-3 gap-2 max-h-[14rem] overflow-y-auto custom-scrollbar content-start">
        {filtered.length === 0 ? (
          <div className="col-span-3 text-center py-6 text-sm text-gray-400">{emptyText}</div>
        ) : (
          filtered.map((item) => {
            const isSelected = selectedIds.has(item.id);
            return (
              <button
                key={item.id}
                type="button"
                onClick={() => onToggle(item.id)}
                className={`relative p-3 rounded-xl border-2 transition-all text-left flex flex-col items-start min-h-[3.5rem] justify-center ${
                  isSelected
                    ? `${colors.border} ${colors.bg} ring-2 ${colors.ring}`
                    : `bg-white text-gray-700 border-gray-200 ${colors.hover}`
                }`}
              >
                <span className={`text-xs font-bold leading-tight ${isSelected ? colors.text : 'text-gray-900'}`}>
                  {item.name}
                </span>
                {renderExtra?.(item)}
                {isSelected && (
                  <div className="absolute top-1.5 right-1.5">
                    <div className={`w-4 h-4 ${colors.check} rounded-full flex items-center justify-center`}>
                      <Check size={10} className="text-white" strokeWidth={3} />
                    </div>
                  </div>
                )}
              </button>
            );
          })
        )}
      </div>
    </div>
  );
}
