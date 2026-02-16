import React, { useRef, useEffect, useCallback, useMemo } from 'react';

export interface WheelColumnItem {
  value: number;
  label: string;
}

interface WheelColumnProps {
  items: WheelColumnItem[];
  selected: number;
  onChange: (value: number) => void;
  label: string;
}

export const ITEM_HEIGHT = 48;
const VISIBLE_COUNT = 5;

export const WheelColumn: React.FC<WheelColumnProps> = React.memo(({ items, selected, onChange, label }) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const scrollTimer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const isUserScroll = useRef(true);

  const selectedIndex = useMemo(
    () => items.findIndex((i) => i.value === selected),
    [items, selected],
  );

  // Scroll to selected on mount and when selected changes programmatically
  useEffect(() => {
    const el = containerRef.current;
    if (!el || selectedIndex < 0) return;
    isUserScroll.current = false;
    el.scrollTop = selectedIndex * ITEM_HEIGHT;
    requestAnimationFrame(() => { isUserScroll.current = true; });
  }, [selectedIndex, items.length]);

  // Detect scroll end via debounce and snap to nearest
  const handleScroll = useCallback(() => {
    if (scrollTimer.current) clearTimeout(scrollTimer.current);
    scrollTimer.current = setTimeout(() => {
      const el = containerRef.current;
      if (!el || !isUserScroll.current) return;
      const idx = Math.round(el.scrollTop / ITEM_HEIGHT);
      const clamped = Math.max(0, Math.min(idx, items.length - 1));
      if (items[clamped] && items[clamped].value !== selected) {
        onChange(items[clamped].value);
      }
    }, 80);
  }, [items, selected, onChange]);

  // Click an item to select it
  const handleClick = useCallback((value: number, index: number) => {
    const el = containerRef.current;
    if (el) {
      el.scrollTo({ top: index * ITEM_HEIGHT, behavior: 'smooth' });
    }
    onChange(value);
  }, [onChange]);

  const padHeight = ITEM_HEIGHT * Math.floor(VISIBLE_COUNT / 2);

  return (
    <div className="flex-1 flex flex-col items-center min-w-0">
      <div className="text-xs font-semibold text-gray-400 uppercase tracking-wider mb-2">{label}</div>
      <div className="relative w-full" style={{ height: ITEM_HEIGHT * VISIBLE_COUNT }}>
        {/* Selection highlight bar */}
        <div
          className="absolute left-1 right-1 rounded-xl bg-teal-50 border border-teal-200 pointer-events-none z-10"
          style={{ top: padHeight, height: ITEM_HEIGHT }}
        />
        {/* Gradient masks */}
        <div className="absolute inset-x-0 top-0 h-20 bg-gradient-to-b from-white to-transparent pointer-events-none z-20" />
        <div className="absolute inset-x-0 bottom-0 h-20 bg-gradient-to-t from-white to-transparent pointer-events-none z-20" />
        {/* Scrollable container with CSS snap */}
        <div
          ref={containerRef}
          className="absolute inset-0 overflow-y-auto select-none scrollbar-hide"
          onScroll={handleScroll}
          style={{
            scrollSnapType: 'y mandatory',
            WebkitOverflowScrolling: 'touch',
            scrollbarWidth: 'none',
            msOverflowStyle: 'none',
          }}
        >
          <div style={{ height: padHeight }} />
          {items.map((item, i) => (
            <div
              key={item.value}
              className={`flex items-center justify-center cursor-pointer transition-colors duration-100 ${
                item.value === selected
                  ? 'text-teal-700 font-bold text-xl'
                  : 'text-gray-400 text-lg'
              }`}
              style={{ height: ITEM_HEIGHT, scrollSnapAlign: 'start' }}
              onClick={() => handleClick(item.value, i)}
            >
              {item.label}
            </div>
          ))}
          <div style={{ height: padHeight }} />
        </div>
      </div>
    </div>
  );
});
