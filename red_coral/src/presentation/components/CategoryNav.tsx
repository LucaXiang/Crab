import React, { useRef, useState, useEffect, useCallback } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { ChevronUp, ChevronDown } from 'lucide-react';
import { Category } from '@/types';

interface CategoryNavProps {
  selected: string;
  onSelect: (category: string) => void;
  categories: (string | Category)[];
}

export const CategoryNav = React.memo<CategoryNavProps>(
  ({ selected, onSelect, categories }) => {
    const { t } = useI18n();
    const scrollRef = useRef<HTMLDivElement>(null);
    const [canScrollUp, setCanScrollUp] = useState(false);
    const [canScrollDown, setCanScrollDown] = useState(false);

    const SCROLL_ROW_HEIGHT = 58;

    const checkScroll = useCallback(() => {
      if (scrollRef.current) {
        const { scrollTop, scrollHeight, clientHeight } = scrollRef.current;
        setCanScrollUp(scrollTop > 0);
        setCanScrollDown(scrollTop + clientHeight < scrollHeight - 1);
      }
    }, []);

    useEffect(() => {
      checkScroll();

      window.addEventListener('resize', checkScroll);

      const observer = new ResizeObserver(checkScroll);
      if (scrollRef.current) {
        observer.observe(scrollRef.current);
      }

      return () => {
        window.removeEventListener('resize', checkScroll);
        observer.disconnect();
      };
    }, [categories, checkScroll]);

    const scroll = useCallback((direction: 'up' | 'down') => {
      if (scrollRef.current) {
        const currentScroll = scrollRef.current.scrollTop;
        const targetScroll = direction === 'up'
          ? currentScroll - SCROLL_ROW_HEIGHT
          : currentScroll + SCROLL_ROW_HEIGHT;

        scrollRef.current.scrollTo({
          top: targetScroll,
          behavior: 'smooth'
        });

        setTimeout(checkScroll, 100);
        setTimeout(checkScroll, 300);
      }
    }, [checkScroll]);

    const showControls = canScrollUp || canScrollDown;

    return (
      <div className="w-full bg-[#FF5E5E] shadow-md relative z-20 h-[134px] flex border-t border-white/10">
        <div
          ref={scrollRef}
          onScroll={checkScroll}
          className="flex-1 px-3 py-[14px] overflow-hidden relative"
        >
          <div className="flex flex-wrap gap-3">
            {categories.map((cat) => {
              const catName = typeof cat === 'string' ? cat : cat.name;
              const isActive = selected === catName;
              const label = catName === 'all' ? (t('common.all')) : catName;
              
              return (
                <button
                  key={catName}
                  onClick={() => onSelect(catName)}
                  className={`
                    px-6 py-2.5 text-lg transition-all duration-200 cursor-pointer rounded-md whitespace-nowrap border h-[46px]
                    ${isActive
                      ? 'bg-white text-[#FF5E5E] border-white font-bold shadow-sm'
                      : 'bg-white/10 text-white border-transparent hover:bg-white/20'
                    }
                  `}
                >
                  {label}
                </button>
              );
            })}
          </div>
        </div>

        {showControls && (
          <div className="w-12 flex flex-col items-center justify-between py-4 border-l border-white/10 bg-[#FF5E5E] shrink-0 shadow-[-4px_0_10px_rgba(0,0,0,0.1)] z-30 animate-in slide-in-from-right duration-200">
            <button
              onClick={() => scroll('up')}
              disabled={!canScrollUp}
              className={`
                p-1.5 rounded-full transition-all
                ${canScrollUp
                  ? 'text-white hover:bg-white/20 active:scale-95 bg-white/5 opacity-100'
                  : 'opacity-0 pointer-events-none'
                }
              `}
              aria-label="Scroll Up"
            >
              <ChevronUp size={24} strokeWidth={3} />
            </button>
            <button
              onClick={() => scroll('down')}
              disabled={!canScrollDown}
              className={`
                p-1.5 rounded-full transition-all
                ${canScrollDown
                  ? 'text-white hover:bg-white/20 active:scale-95 bg-white/5 opacity-100'
                  : 'opacity-0 pointer-events-none'
                }
              `}
              aria-label="Scroll Down"
            >
              <ChevronDown size={24} strokeWidth={3} />
            </button>
          </div>
        )}
      </div>
    );
  },
  // Only re-render if selected or categories change
  (prevProps, nextProps) => {
    // Helper to get category name
    const getCatName = (c: string | Category) => typeof c === 'string' ? c : c.name;

    return (
      prevProps.selected === nextProps.selected &&
      prevProps.categories.length === nextProps.categories.length &&
      prevProps.categories.every((cat, idx) => getCatName(cat) === getCatName(nextProps.categories[idx]))
    );
  }
);

CategoryNav.displayName = 'CategoryNav';
