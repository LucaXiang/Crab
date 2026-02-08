import React, { useRef, useState, useEffect, useCallback, useMemo } from 'react';
import { useI18n } from '@/hooks/useI18n';
import { ChevronUp, ChevronDown, Tag } from 'lucide-react';
import { Category } from '@/core/domain/types';

const SCROLL_CHECK_DELAY_MS = 100;
const SCROLL_ANIMATION_DURATION_MS = 300;

interface CategoryNavProps {
  selected: string;
  onSelect: (category: string) => void;
  categories: Category[];
}

export const CategoryNav = React.memo<CategoryNavProps>(
  ({ selected, onSelect, categories }) => {
    const { t } = useI18n();
    const scrollRef = useRef<HTMLDivElement>(null);
    const [canScrollUp, setCanScrollUp] = useState(false);
    const [canScrollDown, setCanScrollDown] = useState(false);

    const SCROLL_ROW_HEIGHT = 58;

    // Organize categories: [all] | [virtual categories] | [regular categories]
    const organizedCategories = useMemo(() => {
      // Filter and sort virtual categories (is_virtual=true)
      const virtualCats = categories
        .filter((c) => c.is_virtual)
        .sort((a, b) => a.sort_order - b.sort_order);

      // Filter and sort regular categories (is_virtual=false)
      const regularCats = categories
        .filter((c) => !c.is_virtual)
        .sort((a, b) => a.sort_order - b.sort_order);

      return { virtualCats, regularCats };
    }, [categories]);

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

        setTimeout(checkScroll, SCROLL_CHECK_DELAY_MS);
        setTimeout(checkScroll, SCROLL_ANIMATION_DURATION_MS);
      }
    }, [checkScroll]);

    const showControls = canScrollUp || canScrollDown;

    // Render a category button
    const renderCategoryButton = (
      catName: string,
      label: string,
      isVirtual: boolean = false
    ) => {
      const isActive = selected === catName;

      return (
        <button
          key={catName}
          onClick={() => onSelect(catName)}
          className={`
            px-6 py-2.5 text-lg transition-all duration-200 cursor-pointer rounded-xl whitespace-nowrap border h-[2.875rem] flex items-center gap-1.5
            ${isActive
              ? 'bg-white text-primary-500 border-white font-bold shadow-sm'
              : isVirtual
                ? 'bg-white/15 text-white border-white/30 hover:bg-white/25'
                : 'bg-white/10 text-white border-transparent hover:bg-white/20'
            }
          `}
        >
          {isVirtual && <Tag size={14} className="opacity-70" />}
          {label}
        </button>
      );
    };

    const { virtualCats, regularCats } = organizedCategories;
    const hasVirtualCats = virtualCats.length > 0;

    return (
      <div className="w-full bg-primary-500 shadow-md relative z-20 h-[8.375rem] flex border-t border-white/10">
        <div
          ref={scrollRef}
          onScroll={checkScroll}
          className="flex-1 px-3 py-3.5 overflow-hidden relative"
        >
          <div className="flex flex-wrap gap-3">
            {/* 1. "All" button - always first */}
            {renderCategoryButton('all', t('common.status.all'))}

            {/* 2. Virtual categories with visual separator */}
            {hasVirtualCats && (
              <>
                <div className="w-px h-[2.875rem] bg-white/20 mx-1" />
                {virtualCats.map((cat) =>
                  renderCategoryButton(cat.name, cat.name, true)
                )}
                <div className="w-px h-[2.875rem] bg-white/20 mx-1" />
              </>
            )}

            {/* 3. Regular categories */}
            {regularCats.map((cat) =>
              renderCategoryButton(cat.name, cat.name, false)
            )}
          </div>
        </div>

        {showControls && (
          <div className="w-14 flex flex-col items-center justify-between py-3 border-l border-white/10 bg-primary-500 shrink-0 shadow-left z-30 animate-in slide-in-from-right duration-200">
            <button
              onClick={() => scroll('up')}
              disabled={!canScrollUp}
              className={`
                p-2.5 min-w-[2.75rem] min-h-[2.75rem] flex items-center justify-center rounded-full transition-all
                ${canScrollUp
                  ? 'text-white hover:bg-white/20 active:scale-95 active:bg-white/30 bg-white/5 opacity-100'
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
                p-2.5 min-w-[2.75rem] min-h-[2.75rem] flex items-center justify-center rounded-full transition-all
                ${canScrollDown
                  ? 'text-white hover:bg-white/20 active:scale-95 active:bg-white/30 bg-white/5 opacity-100'
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
    // Shallow compare categories by id and is_virtual
    const isCategoriesEqual = (prev: Category[], next: Category[]) => {
      if (prev.length !== next.length) return false;
      return prev.every((cat, idx) =>
        cat.id === next[idx].id &&
        cat.name === next[idx].name &&
        cat.is_virtual === next[idx].is_virtual &&
        cat.sort_order === next[idx].sort_order
      );
    };

    return (
      prevProps.selected === nextProps.selected &&
      isCategoriesEqual(prevProps.categories, nextProps.categories)
    );
  }
);

CategoryNav.displayName = 'CategoryNav';
