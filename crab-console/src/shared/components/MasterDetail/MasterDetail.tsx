import React, { useCallback, useEffect, useRef, useState } from 'react';
import { Search, Plus, ArrowLeft, GripVertical } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';

type ThemeColor = 'blue' | 'teal' | 'orange' | 'purple' | 'indigo';

interface MasterDetailProps<T> {
  items: T[];
  getItemId: (item: T) => number | string;
  renderItem: (item: T, isSelected: boolean) => React.ReactNode;
  selectedId: number | string | null;
  onSelect: (item: T) => void;
  onDeselect: () => void;

  searchQuery: string;
  onSearchChange: (query: string) => void;
  totalCount: number;
  countUnit: string;

  onCreateNew: () => void;
  createLabel: string;
  isCreating: boolean;

  children: React.ReactNode;
  themeColor?: ThemeColor;
  loading?: boolean;
  emptyText?: string;
  /** When provided, enables drag-to-reorder. Called with the reordered items array. */
  onReorder?: (items: T[]) => void;
}

const themeSelected: Record<ThemeColor, string> = {
  blue:   'bg-blue-50/70 border-l-blue-500',
  teal:   'bg-teal-50/70 border-l-teal-500',
  orange: 'bg-orange-50/70 border-l-orange-500',
  purple: 'bg-purple-50/70 border-l-purple-500',
  indigo: 'bg-indigo-50/70 border-l-indigo-500',
};

const themeFocus: Record<ThemeColor, string> = {
  blue:   'focus:ring-blue-500/20 focus:border-blue-400',
  teal:   'focus:ring-teal-500/20 focus:border-teal-400',
  orange: 'focus:ring-orange-500/20 focus:border-orange-400',
  purple: 'focus:ring-purple-500/20 focus:border-purple-400',
  indigo: 'focus:ring-indigo-500/20 focus:border-indigo-400',
};

const themeDot: Record<ThemeColor, string> = {
  blue: 'bg-blue-500', teal: 'bg-teal-500', orange: 'bg-orange-500',
  purple: 'bg-purple-500', indigo: 'bg-indigo-500',
};

export function MasterDetail<T>({
  items, getItemId, renderItem, selectedId, onSelect, onDeselect,
  searchQuery, onSearchChange, totalCount, countUnit,
  onCreateNew, createLabel, isCreating,
  children, themeColor = 'blue', loading, emptyText, onReorder,
}: MasterDetailProps<T>) {
  const { t } = useI18n();
  const [isMobile, setIsMobile] = useState(false);
  const showDetail = selectedId !== null || isCreating;
  const dragIdx = useRef<number | null>(null);
  const [dragOverIdx, setDragOverIdx] = useState<number | null>(null);

  useEffect(() => {
    const mq = window.matchMedia('(max-width: 1023px)');
    setIsMobile(mq.matches);
    const handler = (e: MediaQueryListEvent) => setIsMobile(e.matches);
    mq.addEventListener('change', handler);
    return () => mq.removeEventListener('change', handler);
  }, []);

  const handleBack = useCallback(() => {
    onDeselect();
  }, [onDeselect]);

  // ─── 列表内容 (claude.ai 风格: 搜索突出, 卡片宽敞) ───
  const listContent = (
    <div className="flex flex-col h-full">
      {/* 顶部：搜索 + 新建按钮 + 计数 */}
      <div className="p-4 space-y-3 border-b border-gray-100">
        {/* 搜索栏 — 独立一行，大而突出 */}
        <div className="relative">
          <Search size={16} className="absolute left-3.5 top-1/2 -translate-y-1/2 text-gray-400" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            placeholder={t('common.hint.search_placeholder')}
            className={`w-full pl-10 pr-3 py-2.5 text-sm border border-gray-200 rounded-xl bg-gray-50/50 focus:bg-white focus:outline-none focus:ring-2 transition-colors ${themeFocus[themeColor]}`}
          />
        </div>
        {/* 计数 + 新建按钮 */}
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className={`w-2 h-2 rounded-full ${themeDot[themeColor]}`} />
            <span className="text-xs text-gray-500">
              {totalCount} {countUnit}
            </span>
          </div>
          <button
            onClick={onCreateNew}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-primary-500 text-white rounded-lg text-xs font-medium hover:bg-primary-600 transition-colors"
          >
            <Plus className="w-3.5 h-3.5" />
            {createLabel}
          </button>
        </div>
      </div>

      {/* 列表区域 */}
      <div className="flex-1 overflow-y-auto">
        {loading ? (
          <div className="flex items-center justify-center py-16 text-gray-400">
            <div className="animate-spin w-6 h-6 border-2 border-gray-300 border-t-gray-600 rounded-full" />
          </div>
        ) : items.length === 0 ? (
          <div className="text-center py-16 text-gray-400 text-sm">
            {emptyText || t('common.label.none')}
          </div>
        ) : (
          <div className="p-2">
            {items.map((item, idx) => {
              const id = getItemId(item);
              const isSelected = id === selectedId;
              const isDragTarget = dragOverIdx === idx && dragIdx.current !== idx;
              return (
                <div
                  key={String(id)}
                  className={`flex items-stretch rounded-xl border-l-[3px] transition-all mb-0.5 ${
                    isSelected
                      ? themeSelected[themeColor]
                      : isDragTarget
                        ? 'border-l-transparent bg-blue-50/50 ring-1 ring-blue-300'
                        : 'border-l-transparent hover:bg-gray-50'
                  }`}
                  onDragOver={onReorder ? (e) => { e.preventDefault(); setDragOverIdx(idx); } : undefined}
                  onDragLeave={onReorder ? () => { if (dragOverIdx === idx) setDragOverIdx(null); } : undefined}
                  onDrop={onReorder ? (e) => {
                    e.preventDefault();
                    if (dragIdx.current !== null && dragIdx.current !== idx) {
                      const reordered = [...items];
                      const [moved] = reordered.splice(dragIdx.current, 1);
                      reordered.splice(idx, 0, moved);
                      onReorder(reordered);
                    }
                    dragIdx.current = null;
                    setDragOverIdx(null);
                  } : undefined}
                >
                  {onReorder && (
                    <div
                      draggable
                      onDragStart={() => { dragIdx.current = idx; }}
                      onDragEnd={() => { dragIdx.current = null; setDragOverIdx(null); }}
                      className="flex items-center px-1 cursor-grab active:cursor-grabbing text-gray-300 hover:text-gray-500 shrink-0"
                    >
                      <GripVertical size={14} />
                    </div>
                  )}
                  <button
                    onClick={() => onSelect(item)}
                    className="flex-1 text-left cursor-pointer min-w-0"
                  >
                    {renderItem(item, isSelected)}
                  </button>
                </div>
              );
            })}
          </div>
        )}
      </div>
    </div>
  );

  // ─── 详情内容 ───
  const detailContent = showDetail ? (
    <div className="flex flex-col h-full bg-white">
      {isMobile && (
        <div className="flex items-center gap-3 px-4 py-3 border-b border-slate-100 bg-white sticky top-0 z-20 shadow-sm">
          <button 
            onClick={handleBack} 
            className="p-2 -ml-2 rounded-full hover:bg-slate-100 active:bg-slate-200 transition-colors text-slate-600"
          >
            <ArrowLeft className="w-6 h-6" />
          </button>
          <span className="font-semibold text-slate-900 text-lg">{isCreating ? createLabel : t('common.action.edit')}</span>
        </div>
      )}
      <div className="flex-1 overflow-y-auto bg-white">
        {children}
      </div>
    </div>
  ) : (
    <div className="flex flex-col items-center justify-center h-full text-slate-300 gap-3 p-8 text-center">
      <div className="w-16 h-16 rounded-2xl bg-slate-50 flex items-center justify-center">
        <Search className="w-8 h-8 text-slate-300" />
      </div>
      <p className="text-sm font-medium text-slate-400 max-w-[200px]">{t('common.hint.select_item')}</p>
    </div>
  );

  // ─── 手机端：列表或详情全屏 ───
  if (isMobile) {
    return (
      <div className="h-full relative overflow-hidden bg-white">
        <div 
          className={`absolute inset-0 transition-transform duration-300 ease-out bg-white z-10 ${
            showDetail ? '-translate-x-full opacity-0 pointer-events-none' : 'translate-x-0 opacity-100'
          }`}
        >
          {listContent}
        </div>
        <div 
          className={`absolute inset-0 transition-transform duration-300 ease-out bg-white z-20 ${
            showDetail ? 'translate-x-0' : 'translate-x-full pointer-events-none'
          }`}
        >
          {detailContent}
        </div>
      </div>
    );
  }

  // ─── 桌面端：左右分栏 ───
  return (
    <div className="flex h-full border border-gray-200 rounded-2xl overflow-hidden bg-white shadow-sm">
      <div className="w-[40%] border-r border-gray-100 overflow-hidden flex flex-col bg-white">
        {listContent}
      </div>
      <div className="flex-1 overflow-hidden flex flex-col bg-gray-50/30">
        {detailContent}
      </div>
    </div>
  );
}
