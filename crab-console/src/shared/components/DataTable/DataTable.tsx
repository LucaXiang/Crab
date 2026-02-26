import React from 'react';
import { Edit3, Trash2, ChevronLeft, ChevronRight, Check, X, ListChecks } from 'lucide-react';
import { useI18n } from '@/hooks/useI18n';
import { THEMES } from './dataTableThemes';
import { useDataTableSelection } from './useDataTableSelection';

export interface Column<T> {
  key: string;
  header: string;
  width?: string;
  render?: (item: T) => React.ReactNode;
  align?: 'left' | 'center' | 'right';
}

interface DataTableProps<T> {
  data: T[];
  columns: Column<T>[];
  loading?: boolean;
  emptyText?: string;
  onEdit?: (item: T) => void;
  onDelete?: (item: T) => void;
  isEditable?: (item: T) => boolean;
  isDeletable?: (item: T) => boolean;
  onBatchDelete?: (items: T[]) => void;
  getRowKey: (item: T) => string | number;
  pageSize?: number;
  selectable?: boolean;
  themeColor?: 'blue' | 'orange' | 'purple' | 'teal' | 'indigo';
  // Server-side pagination props
  totalItems?: number;
  currentPage?: number;
  onPageChange?: (page: number) => void;
  // External selection mode control
  isSelectionMode?: boolean;
  onSelectionModeChange?: (mode: boolean) => void;
  // Min height for the table container
  minHeight?: string;
}

export function DataTable<T>({
  data,
  columns,
  loading = false,
  emptyText,
  onEdit,
  onDelete,
  isEditable,
  isDeletable,
  onBatchDelete,
  getRowKey,
  pageSize = 10,
  selectable = true,
  themeColor = 'blue',
  totalItems,
  currentPage: propCurrentPage,
  onPageChange,
  isSelectionMode: propIsSelectionMode,
  onSelectionModeChange,
  minHeight,
}: DataTableProps<T>) {
  const { t } = useI18n();
  const theme = THEMES[themeColor] || THEMES.blue;
  const [internalPage, setInternalPage] = React.useState(1);
  const [longPressTimer, setLongPressTimer] = React.useState<NodeJS.Timeout | null>(null);

  const isServerSide = typeof totalItems === 'number';
  const currentPage = isServerSide ? (propCurrentPage || 1) : internalPage;
  const finalTotalItems = isServerSide ? (totalItems || 0) : data.length;
  const totalPages = Math.ceil(finalTotalItems / pageSize);

  const currentData = isServerSide ? data : data.slice((currentPage - 1) * pageSize, currentPage * pageSize);

  const {
    selectedKeys,
    isSelectionMode,
    toggleSelection,
    toggleSelectAll,
    handleBatchDelete,
    exitSelectionMode,
  } = useDataTableSelection({
    data,
    currentData,
    getRowKey,
    onBatchDelete,
    propIsSelectionMode,
    onSelectionModeChange,
  });

  const startIndex = (currentPage - 1) * pageSize;
  const endIndex = startIndex + currentData.length;

  // Reset to page 1 when data changes (only for client-side)
  React.useEffect(() => {
    if (!isServerSide) {
      setInternalPage(1);
    }
  }, [data.length, isServerSide]);

  // Cleanup long press timer on unmount
  React.useEffect(() => {
    return () => {
      if (longPressTimer) {
        clearTimeout(longPressTimer);
      }
    };
  }, [longPressTimer]);

  const showActions = onEdit || onDelete;
  const canBatchDelete = selectable && onBatchDelete;

  const getAlignClass = (align?: 'left' | 'center' | 'right') => {
    switch (align) {
      case 'center': return 'text-center';
      case 'right': return 'text-right';
      default: return 'text-left';
    }
  };

  const handlePageChange = (page: number) => {
    const p = Math.max(1, Math.min(totalPages, page));
    if (isServerSide && onPageChange) {
      onPageChange(p);
    } else {
      setInternalPage(p);
    }
  };

  // Long press handler
  const handleLongPress = (item: T, callback?: (item: T) => void) => {
    if (!callback) return;
    const timer = setTimeout(() => {
      callback(item);
    }, 500);
    setLongPressTimer(timer);
  };

  const clearLongPress = () => {
    if (longPressTimer) {
      clearTimeout(longPressTimer);
      setLongPressTimer(null);
    }
  };

  // Double click handler
  const [lastClickTime, setLastClickTime] = React.useState(0);
  const handleDoubleClick = (item: T, callback?: (item: T) => void) => {
    if (!callback) return;
    const now = Date.now();
    if (now - lastClickTime < 300) {
      callback(item);
      setLastClickTime(0);
    } else {
      setLastClickTime(now);
    }
  };

  const getPaginationRange = () => {
    const siblingCount = 1;
    if (totalPages <= 7) {
      return Array.from({ length: totalPages }, (_, i) => i + 1);
    }

    const leftSiblingIndex = Math.max(currentPage - siblingCount, 1);
    const rightSiblingIndex = Math.min(currentPage + siblingCount, totalPages);

    const showLeftDots = leftSiblingIndex > 2;
    const showRightDots = rightSiblingIndex < totalPages - 1;

    if (!showLeftDots && showRightDots) {
      const leftItemCount = 5;
      const leftRange = Array.from({ length: leftItemCount }, (_, i) => i + 1);
      return [...leftRange, '...', totalPages];
    }

    if (showLeftDots && !showRightDots) {
      const rightItemCount = 5;
      const rightRange = Array.from({ length: rightItemCount }, (_, i) => totalPages - rightItemCount + i + 1);
      return [1, '...', ...rightRange];
    }

    if (showLeftDots && showRightDots) {
      const middleRange = Array.from({ length: rightSiblingIndex - leftSiblingIndex + 1 }, (_, i) => leftSiblingIndex + i);
      return [1, '...', ...middleRange, '...', totalPages];
    }

    return [];
  };

  const paginationRange = getPaginationRange();

  if (loading && (!data || data.length === 0)) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 overflow-hidden" style={{ minHeight }}>
        <div className="animate-pulse">
          {/* Desktop skeleton */}
          <div className="hidden md:block">
            <div className="bg-gradient-to-r from-gray-50 to-gray-100 border-b border-gray-200 p-4">
              <div className="flex gap-4">
                {columns.map((_, i) => (
                  <div key={i} className="h-4 bg-gray-200 rounded flex-1" />
                ))}
              </div>
            </div>
            {[1, 2, 3].map((i) => (
              <div key={i} className="border-b border-gray-100 p-4">
                <div className="flex gap-4">
                  {columns.map((_, j) => (
                    <div key={j} className="h-4 bg-gray-100 rounded flex-1" />
                  ))}
                </div>
              </div>
            ))}
          </div>
          {/* Mobile skeleton */}
          <div className="md:hidden">
            {[1, 2, 3].map((i) => (
              <div key={i} className="p-4 border-b border-gray-100 space-y-2">
                <div className="h-5 bg-gray-200 rounded w-2/3" />
                <div className="h-3 bg-gray-100 rounded w-full" />
                <div className="h-3 bg-gray-100 rounded w-1/2" />
              </div>
            ))}
          </div>
        </div>
      </div>
    );
  }

  if (!loading && data.length === 0 && !isServerSide) {
    return (
      <div className="bg-white rounded-xl border border-gray-200 overflow-hidden" style={{ minHeight }}>
        <div className="flex flex-col items-center justify-center py-12 px-4">
          <div className="w-16 h-16 bg-gray-100 rounded-full flex items-center justify-center mb-4">
            <svg className="w-8 h-8 text-gray-400" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={1.5} d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0v5a2 2 0 01-2 2H6a2 2 0 01-2-2v-5m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
            </svg>
          </div>
          <p className="text-gray-500 text-sm">{emptyText || t('common.label.none')}</p>
        </div>
      </div>
    );
  }

  const allSelected = currentData.length > 0 && selectedKeys.size === currentData.length;
  const someSelected = selectedKeys.size > 0 && selectedKeys.size < currentData.length;

  // Identify primary column (first column, usually name) and secondary columns
  const primaryCol = columns[0];
  const secondaryColumns = columns.slice(1);

  return (
    <div className="relative bg-white rounded-xl border border-gray-200 overflow-hidden shadow-sm" style={{ minHeight }}>
      {/* Loading Overlay */}
      {loading && data.length > 0 && (
        <div className="absolute inset-0 bg-white/60 z-10 flex items-center justify-center backdrop-blur-[1px]">
          <div className={`w-8 h-8 border-4 border-gray-200 rounded-full animate-spin ${
            themeColor === 'orange' ? 'border-t-orange-500' :
            themeColor === 'purple' ? 'border-t-purple-600' :
            themeColor === 'teal' ? 'border-t-teal-600' :
            themeColor === 'indigo' ? 'border-t-indigo-600' :
            'border-t-blue-600'
          }`} />
        </div>
      )}

      {/* Selection Mode Header */}
      {isSelectionMode && (
        <div className={`${theme.selectModeHeader} border-b px-4 py-3 flex items-center justify-between`}>
          <div className="flex items-center gap-3">
            <span className={`text-sm ${theme.selectModeText} font-medium`}>
              {t('common.selection.selected')} {selectedKeys.size} {t('common.label.items')}
            </span>
          </div>
          <div className="flex items-center gap-2">
            <button
              onClick={handleBatchDelete}
              disabled={selectedKeys.size === 0}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-red-600 text-white rounded-lg text-xs font-medium hover:bg-red-700 transition-colors disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <Trash2 size={14} />
              <span className="hidden sm:inline">{t('common.action.batch_delete')}</span>
            </button>
            <button
              onClick={exitSelectionMode}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 bg-gray-200 text-gray-700 rounded-lg text-xs font-medium hover:bg-gray-300 transition-colors"
            >
              <X size={14} />
              <span className="hidden sm:inline">{t('common.action.cancel')}</span>
            </button>
          </div>
        </div>
      )}

      {/* ── Mobile Card View ── */}
      <div className="md:hidden space-y-3 p-3 bg-slate-50/50">
        {currentData.map((item, index) => {
          const key = getRowKey(item);
          const isSelected = selectedKeys.has(key);
          return (
            <div
              key={key}
              className={`p-4 rounded-xl border transition-all duration-200 relative overflow-hidden ${
                isSelected 
                  ? 'bg-blue-50/60 border-blue-200 shadow-sm' 
                  : 'bg-white border-slate-200/60 shadow-sm active:scale-[0.98]'
              }`}
              onTouchStart={() => handleLongPress(item, onEdit)}
              onTouchEnd={clearLongPress}
            >
              <div className="flex items-start justify-between gap-3">
                {/* Selection checkbox */}
                {isSelectionMode && (
                  <button
                    onClick={() => toggleSelection(key)}
                    className={`mt-0.5 w-6 h-6 rounded-lg border-2 flex items-center justify-center shrink-0 transition-all ${
                      isSelected ? 'bg-blue-500 border-blue-500' : 'bg-white border-slate-300'
                    }`}
                  >
                    {isSelected && <Check size={14} className="text-white" />}
                  </button>
                )}

                {/* Primary column (name) */}
                <div className="flex-1 min-w-0">
                  <div className={`font-semibold text-base mb-1 ${isSelected ? 'text-blue-700' : 'text-slate-900'}`}>
                    {primaryCol.render ? primaryCol.render(item) : String((item as Record<string, unknown>)[primaryCol.key] ?? '')}
                  </div>
                </div>

                {/* Action buttons - Improved touch targets */}
                {showActions && !isSelectionMode && (
                  <div className="flex items-center gap-2 shrink-0">
                    {onEdit && (!isEditable || isEditable(item)) && (
                      <button
                        onClick={() => onEdit(item)}
                        className="w-8 h-8 flex items-center justify-center bg-slate-50 text-slate-600 rounded-full hover:bg-primary-50 hover:text-primary-600 transition-colors border border-slate-200"
                        title={t('common.action.edit')}
                      >
                        <Edit3 size={16} />
                      </button>
                    )}
                    {onDelete && (!isDeletable || isDeletable(item)) && (
                      <button
                        onClick={() => onDelete(item)}
                        className="w-8 h-8 flex items-center justify-center bg-slate-50 text-slate-600 rounded-full hover:bg-red-50 hover:text-red-600 transition-colors border border-slate-200"
                        title={t('common.action.delete')}
                      >
                        <Trash2 size={16} />
                      </button>
                    )}
                  </div>
                )}
              </div>

              {/* Secondary columns as key:value pairs */}
              {secondaryColumns.length > 0 && (
                <div className="mt-3 pt-3 border-t border-slate-100 grid grid-cols-2 gap-x-4 gap-y-2">
                  {secondaryColumns.map(col => {
                    const val = col.render ? col.render(item) : String((item as Record<string, unknown>)[col.key] ?? '');
                    return (
                      <div key={col.key} className="flex flex-col">
                        <span className="text-[10px] uppercase tracking-wider font-semibold text-slate-400 mb-0.5">{col.header}</span>
                        <span className="text-sm text-slate-700 font-medium truncate">{val}</span>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {/* ── Desktop Table View ── */}
      <div className="hidden md:block">
        {/* Header */}
        <div className="bg-gradient-to-r from-gray-50 to-gray-100 border-b border-gray-200">
          <div className="flex">
            {isSelectionMode && (
              <div className="px-4 py-3.5 flex items-center w-12">
                <button
                  onClick={toggleSelectAll}
                  className={`w-5 h-5 rounded-sm border-2 flex items-center justify-center transition-colors ${
                    allSelected
                      ? theme.checkboxActive
                      : someSelected
                        ? theme.checkboxActive
                        : `border-gray-300 ${theme.checkboxHover}`
                  }`}
                >
                  {allSelected && <Check size={12} className="text-white" />}
                  {someSelected && <div className="w-2 h-0.5 bg-white rounded" />}
                </button>
              </div>
            )}
            {columns.map((col) => (
              <div
                key={col.key}
                className={`px-4 py-3.5 text-xs font-semibold text-gray-600 uppercase tracking-wider whitespace-nowrap ${getAlignClass(col.align)}`}
                style={{ width: col.width, flex: col.width ? undefined : 1 }}
              >
                {col.header}
              </div>
            ))}
            {showActions && !isSelectionMode && (
              <div className="px-4 py-3.5 text-xs font-semibold text-gray-600 uppercase tracking-wider text-right w-32 whitespace-nowrap">
                {t('settings.common.actions')}
              </div>
            )}
          </div>
        </div>

        {/* Body */}
        <div className="divide-y divide-gray-100">
          {currentData.map((item, index) => {
            const key = getRowKey(item);
            const isSelected = selectedKeys.has(key);
            return (
              <div
                key={key}
                className={`flex items-center transition-colors ${
                  isSelected
                    ? theme.rowSelected
                    : index % 2 === 0
                      ? `bg-white ${theme.rowHover}`
                      : `bg-gray-50/30 ${theme.rowHover}`
                }`}
                onMouseDown={() => handleLongPress(item, onEdit)}
                onMouseUp={clearLongPress}
                onMouseLeave={clearLongPress}
                onTouchStart={() => handleLongPress(item, onEdit)}
                onTouchEnd={clearLongPress}
                onDoubleClick={() => handleDoubleClick(item, onEdit)}
              >
                {isSelectionMode && (
                  <div className="px-4 py-3.5 flex items-center w-12">
                    <button
                      onClick={() => toggleSelection(key)}
                      className={`w-5 h-5 rounded-sm border-2 flex items-center justify-center transition-colors ${
                        isSelected
                          ? theme.checkboxActive
                          : `border-gray-300 ${theme.checkboxHover}`
                      }`}
                    >
                      {isSelected && <Check size={12} className="text-white" />}
                    </button>
                  </div>
                )}
                {columns.map((col) => (
                  <div
                    key={col.key}
                    className={`px-4 py-3.5 text-sm text-gray-700 ${getAlignClass(col.align)}`}
                    style={{ width: col.width, flex: col.width ? undefined : 1 }}
                  >
                    {col.render ? col.render(item) : String((item as Record<string, unknown>)[col.key] ?? '')}
                  </div>
                ))}
                {showActions && !isSelectionMode && (
                  <div className="px-4 py-2 flex items-center justify-end gap-2 w-32">
                    {(
                      <>
                        {onEdit && (!isEditable || isEditable(item)) && (
                          <button
                            onClick={() => onEdit(item)}
                            className="p-2 bg-amber-50 text-amber-700 rounded-lg hover:bg-amber-100 transition-colors border border-amber-200/50"
                            title={t('common.action.edit')}
                          >
                            <Edit3 size={14} />
                          </button>
                        )}
                        {onDelete && (!isDeletable || isDeletable(item)) && (
                          <button
                            onClick={() => onDelete(item)}
                            className="p-2 bg-red-50 text-red-600 rounded-lg hover:bg-red-100 transition-colors border border-red-200/50"
                            title={t('common.action.delete')}
                          >
                            <Trash2 size={14} />
                          </button>
                        )}
                      </>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      </div>

      {/* Footer */}
      <div className="bg-gray-50 border-t border-gray-200 px-4 py-3 flex flex-col sm:flex-row items-center justify-between gap-2">
        <div className="flex items-center gap-3">
          {totalPages > 1 && (
            <div className="text-xs text-gray-500">
              {t('common.selection.showing')} {startIndex + 1}-{Math.min(endIndex, finalTotalItems)} / {finalTotalItems}
            </div>
          )}
          {totalPages <= 1 && (
            <div className="text-xs text-gray-500">
              {t('common.label.total')} {finalTotalItems} {t('common.label.items')}
            </div>
          )}
        </div>
        {totalPages > 0 && (
          <div className="flex items-center gap-1">
            <button
              onClick={() => handlePageChange(currentPage - 1)}
              disabled={currentPage === 1}
              className="p-1.5 rounded-lg hover:bg-gray-200 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              <ChevronLeft size={16} className="text-gray-600" />
            </button>
            {paginationRange.map((page, index) => {
              if (page === '...') {
                return (
                  <span key={`dots-${index}`} className="px-2 text-gray-400 select-none flex items-center">
                    ...
                  </span>
                );
              }
              return (
                <button
                  key={page}
                  onClick={() => handlePageChange(page as number)}
                  className={`min-w-[1.75rem] h-7 px-2 rounded-lg text-xs font-medium transition-colors ${
                    page === currentPage
                      ? theme.paginationActive
                      : 'text-gray-600 hover:bg-gray-200'
                  }`}
                >
                  {page}
                </button>
              );
            })}
            <button
              onClick={() => handlePageChange(currentPage + 1)}
              disabled={currentPage === totalPages}
              className="p-1.5 rounded-lg hover:bg-gray-200 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
            >
              <ChevronRight size={16} className="text-gray-600" />
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
